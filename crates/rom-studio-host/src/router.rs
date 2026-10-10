use crate::auth_diagnostics::{AuthOperation, AuthOutcome, AuthStage};
use crate::{HostConfig, assets::Assets, csrf, session::SessionStore};
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rom::{Error, Result, Runtime};
use std::{future::Future, sync::Arc};
use tokio::net::TcpListener;

pub(crate) struct Shared {
    pub(crate) runtime: Runtime,
    pub(crate) config: HostConfig,
    pub(crate) sessions: SessionStore,
    assets: Assets,
    pub(crate) client: reqwest::Client,
    pub(crate) attempts: crate::login::Attempts,
    pub(crate) auth: crate::lifecycle::Supervisor,
    shutdown: tokio::sync::Mutex<()>,
    pub(crate) blob_bodies: Arc<tokio::sync::Semaphore>,
}
/// Optional HTTP host. Resource operations use the ordinary generic ROM binding.
#[derive(Clone)]
pub struct StudioHost {
    pub(crate) shared: Arc<Shared>,
    pub(crate) http: rom_http::Http,
}
impl StudioHost {
    pub fn new(runtime: Runtime, config: HostConfig) -> Result<Self> {
        config.validate()?;
        if config.http_limits.bodies == 0
            || config.http_limits.bodies > tokio::sync::Semaphore::MAX_PERMITS
        {
            return Err(Error::TooLarge);
        }
        if config
            .blobs
            .as_ref()
            .is_some_and(|service| !service.uses_runtime(&runtime))
        {
            return Err(Error::Invalid {
                kind: "studio-host".into(),
                field: "blob runtime".into(),
            });
        }
        let mut assets = Assets::load(
            &config.asset_directory,
            config.limits.assets,
            config.limits.asset_bytes,
        )?;
        if let Some(profile) = &config.studio_bootstrap {
            assets.install_bootstrap(profile, config.limits.asset_bytes)?;
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.limits.acquisition_timeout)
            .connect_timeout(config.limits.acquisition_timeout)
            .build()
            .map_err(|_| Error::Storage)?;
        let shared = Arc::new(Shared {
            blob_bodies: Arc::new(tokio::sync::Semaphore::new(config.http_limits.bodies)),
            shutdown: tokio::sync::Mutex::new(()),
            client,
            attempts: crate::login::Attempts::new(config.limits.login_attempts),
            auth: crate::lifecycle::Supervisor::observed(
                config.limits.authentication_jobs,
                config
                    .authentication_diagnostics
                    .then(crate::auth_diagnostics::Capture::new),
            ),
            sessions: SessionStore::new(config.limits.sessions, config.limits.session_seconds),
            runtime: runtime.clone(),
            config,
            assets,
        });
        let resolver = shared.clone();
        let auth: rom_http::AsyncAuthResolver = Arc::new(move |headers| {
            let shared = resolver.clone();
            Box::pin(async move {
                let cookie = csrf::session_cookie(&headers, "rom_session")?;
                crate::authentication::resolve_tagged(
                    &shared,
                    &cookie,
                    AuthOperation::GenericHttpResolver,
                )
                .await
            })
        });
        let http = rom_http::Http::new_async(runtime, auth, shared.config.http_limits)?;
        Ok(Self { shared, http })
    }
    /// Return an optional bounded authentication snapshot.
    /// None means disabled; an error means diagnostics are unavailable, not an auth decision.
    pub fn authentication_diagnostics(&self) -> Result<Option<crate::AuthSnapshot>> {
        self.shared.auth.authentication_diagnostics()
    }
    pub fn router(&self) -> Router {
        let api = self
            .http
            .router()
            .layer(middleware::from_fn_with_state(self.shared.clone(), protect));
        let routes = Router::new()
            .route("/", get(assets))
            .route("/auth/login/{provider}", get(crate::authentication::login))
            .route(
                "/auth/callback/{provider}",
                get(crate::authentication::callback),
            )
            .route("/auth/session", get(session))
            .route("/auth/providers", get(providers))
            .route("/auth/logout", post(logout))
            .route("/blobs/reserve", post(crate::blobs::reserve))
            .route("/blobs/capabilities", get(crate::blob_capabilities::read))
            .route("/blobs/upload", post(crate::blobs::upload))
            .route("/blobs/detach", post(crate::blobs::detach))
            .route("/blobs/attachment/{id}", get(crate::blobs::download))
            .route("/blobs/attachment", get(crate::blobs::download_query))
            .fallback(assets)
            .with_state(self.shared.clone())
            .nest("/api", api);
        if self.shared.config.base_path == "/" {
            routes
        } else {
            Router::new()
                .route(&self.shared.config.base_path, get(assets))
                .with_state(self.shared.clone())
                .nest(self.shared.config.base_path.trim_end_matches('/'), routes)
        }
    }
    pub async fn shutdown(&self) -> Result<()> {
        let _guard = self.shared.shutdown.lock().await;
        self.shared.auth.close();
        self.shared.attempts.close();
        self.shared.sessions.close();
        let auth = self.shared.auth.drain().await;
        let blobs = if let Some(blobs) = &self.shared.config.blobs {
            blobs.shutdown().await.map_err(|_| Error::Storage)
        } else {
            Ok(())
        };
        let runtime = self.http.shutdown().await;
        auth.and(blobs).and(runtime)
    }
    pub async fn serve<F>(self, listener: TcpListener, stop: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let closer = self.clone();
        let result = axum::serve(listener, self.router())
            .with_graceful_shutdown(async move {
                stop.await;
                closer.shared.auth.close();
                closer.shared.attempts.close();
                closer.shared.sessions.close();
            })
            .await
            .map_err(|_| Error::Storage);
        let shutdown = self.shutdown().await;
        result.and(shutdown)
    }
}
async fn session(State(shared): State<Arc<Shared>>, headers: axum::http::HeaderMap) -> Response {
    let session = csrf::session_cookie(&headers, "rom_session")
        .ok()
        .and_then(|cookie| shared.sessions.lookup(&cookie, shared.config.clock.now()));
    let session = if let Some(session) = session {
        match crate::authentication::resolve(&shared, session.cookie()).await {
            Ok(_) => Some(session),
            Err(Error::Denied) => None,
            Err(error) => return crate::authentication::failure(error),
        }
    } else {
        None
    };
    let value = match session {
        Some(session) => {
            serde_json::json!({"authenticated":true,"generation":session.generation(),"csrf_token":session.csrf(),"user_id":session.evidence.user_id,"expires_at":session.expires_at()})
        }
        None => serde_json::json!({"authenticated":false,"generation":"anonymous"}),
    };
    no_store(axum::Json(value).into_response())
}
async fn providers(State(shared): State<Arc<Shared>>) -> Response {
    let mut providers = Vec::new();
    for config in &shared.config.approved {
        if let Ok(activation) = rom_identity::ProviderActivation::read(
            &shared.runtime,
            &shared.config.host_actor,
            &config.authority,
        )
        .await
            && activation.config().issuer == config.issuer
            && activation.config().audience == config.client_id
            && activation.config().profile == rom_identity::ProviderProfile::OidcRs256Human
        {
            providers.push(serde_json::json!({"id":config.authority,"label":config.label}));
        }
    }
    let selected = if let Some(id) = &shared.config.settings_id {
        match shared
            .runtime
            .read::<crate::StudioSettings>(&shared.config.host_actor, id)
            .await
        {
            Ok(view) => view.value.and_then(|settings| settings.primary_provider),
            Err(_) => None,
        }
    } else {
        shared.config.primary.clone()
    };
    let primary = selected.filter(|id| {
        providers
            .iter()
            .any(|provider| provider["id"].as_str() == Some(id.as_str()))
    });
    no_store(
        axum::Json(serde_json::json!({"providers":providers,"primary":primary})).into_response(),
    )
}
async fn protect(State(shared): State<Arc<Shared>>, request: Request, next: Next) -> Response {
    if csrf::check_origin(request.headers(), &shared.config.public_origin).is_err() {
        return crate::authentication::denied(StatusCode::UNAUTHORIZED);
    }
    let Ok(cookie) = csrf::session_cookie(request.headers(), "rom_session") else {
        return crate::authentication::denied(StatusCode::UNAUTHORIZED);
    };
    let session = shared.sessions.lookup(&cookie, shared.config.clock.now());
    shared.auth.record(
        AuthOperation::ProtectedMiddleware,
        AuthStage::SessionLookup,
        if session.is_some() {
            AuthOutcome::Succeeded
        } else {
            AuthOutcome::Denied
        },
    );
    let Some(session) = session else {
        return crate::authentication::denied(StatusCode::UNAUTHORIZED);
    };
    if csrf::check_token(request.headers(), &session).is_err() {
        return crate::authentication::denied(StatusCode::FORBIDDEN);
    }
    let actor = match crate::authentication::resolve_tagged(
        &shared,
        &cookie,
        AuthOperation::ProtectedMiddleware,
    )
    .await
    {
        Ok(actor) => actor,
        Err(error) => return crate::authentication::failure(error),
    };
    let response = next.run(request).await;
    let response = if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/event-stream"))
    {
        crate::session_observation::guard(response, shared, session, actor)
    } else {
        response
    };
    no_store(response)
}
async fn logout(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    if csrf::check_origin(request.headers(), &shared.config.public_origin).is_err() {
        return crate::authentication::denied(StatusCode::UNAUTHORIZED);
    }
    let Ok(cookie) = csrf::session_cookie(request.headers(), "rom_session") else {
        return crate::authentication::denied(StatusCode::UNAUTHORIZED);
    };
    let Some(session) = shared.sessions.lookup(&cookie, shared.config.clock.now()) else {
        return crate::authentication::denied(StatusCode::UNAUTHORIZED);
    };
    if csrf::check_token(request.headers(), &session).is_err() {
        return crate::authentication::denied(StatusCode::FORBIDDEN);
    }
    shared.sessions.remove(&cookie);
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "rom_session=; Path={}; Max-Age=0; HttpOnly; SameSite=Lax{}",
            shared.config.base_path,
            if shared.config.secure() {
                "; Secure"
            } else {
                ""
            }
        ))
        .unwrap(),
    );
    no_store(response)
}
async fn assets(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    if request.method() != axum::http::Method::GET {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = request.uri().path();
    let path = path
        .strip_prefix(&shared.config.base_path)
        .unwrap_or(path)
        .trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let Some(asset) = shared.assets.get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = Body::from(asset.bytes.to_vec()).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(asset.content_type),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}
pub(crate) fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
