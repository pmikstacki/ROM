use crate::{
    csrf,
    login::Attempt,
    oidc::Credentials,
    router::{Shared, no_store},
    session::{SessionEvidence, secret},
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rom::{Actor, Error, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(crate) async fn resolve(shared: &Arc<Shared>, cookie: &str) -> Result<Actor> {
    let session = shared
        .sessions
        .lookup(cookie, shared.config.clock.now())
        .ok_or(Error::Denied)?;
    let credentials = session.evidence.credentials.clone().ok_or(Error::Denied)?;
    let work_shared = shared.clone();
    let work_session = session.clone();
    let result = shared
        .auth
        .run(async move {
            if *work_session.cancellation().borrow() {
                return Err(Error::Denied);
            }
            let actor = credentials.actor(&work_shared).await?;
            if *work_session.cancellation().borrow() {
                return Err(Error::Denied);
            }
            Ok(actor)
        })
        .await;
    if let Err(error) = &result {
        shared.sessions.failed(cookie, error);
    }
    result
}
pub(crate) async fn login(
    State(shared): State<Arc<Shared>>,
    Path(authority): Path<String>,
) -> Response {
    match begin(&shared, &authority).await {
        Ok(response) => response,
        Err(error) => failure(error),
    }
}
async fn begin(shared: &Arc<Shared>, authority: &str) -> Result<Response> {
    let config = shared
        .config
        .approved
        .iter()
        .find(|provider| provider.authority == authority)
        .ok_or(Error::Denied)?;
    let activation = rom_identity::ProviderActivation::read(
        &shared.runtime,
        &shared.config.host_actor,
        authority,
    )
    .await?;
    if activation.config().issuer != config.issuer
        || activation.config().audience != config.client_id
        || activation.config().profile != rom_identity::ProviderProfile::OidcRs256Human
    {
        return Err(Error::Denied);
    }
    let browser = secret()?;
    let nonce = secret()?;
    let verifier = secret()?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = shared.attempts.insert(
        Attempt {
            provider: authority.into(),
            browser: browser.clone(),
            nonce: nonce.clone(),
            verifier,
            expires: shared
                .config
                .clock
                .now()
                .saturating_add(shared.config.limits.login_seconds),
            activation: Some(activation),
        },
        shared.config.clock.now(),
    )?;
    let mut url = url::Url::parse(&config.authorization_endpoint).map_err(|_| Error::Denied)?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", config.client_id.as_str()),
        ("redirect_uri", shared.config.callback(authority).as_str()),
        ("response_type", "code"),
        ("scope", "openid profile"),
        ("state", state.as_str()),
        ("nonce", nonce.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
    ]);
    let mut response = Redirect::to(url.as_str()).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie(
            shared,
            "rom_login",
            &browser,
            shared.config.limits.login_seconds,
        )?,
    );
    Ok(no_store(response))
}
#[derive(Deserialize)]
pub(crate) struct Callback {
    code: Option<String>,
    state: String,
    iss: Option<String>,
    error: Option<String>,
}
pub(crate) async fn callback(
    State(shared): State<Arc<Shared>>,
    Path(authority): Path<String>,
    Query(query): Query<Callback>,
    headers: HeaderMap,
) -> Response {
    let result = finish(shared, authority, query, headers).await;
    match result {
        Ok(response) => response,
        Err(error) => failure(error),
    }
}
async fn finish(
    shared: Arc<Shared>,
    authority: String,
    query: Callback,
    headers: HeaderMap,
) -> Result<Response> {
    let browser = csrf::session_cookie(&headers, "rom_login")?;
    let attempt = shared.attempts.consume(
        &query.state,
        &authority,
        &browser,
        shared.config.clock.now(),
    )?;
    let config = shared
        .config
        .approved
        .iter()
        .find(|provider| provider.authority == authority)
        .cloned()
        .ok_or(Error::Denied)?;
    if query.error.is_some()
        || query
            .iss
            .as_ref()
            .is_some_and(|issuer| issuer != &config.issuer)
    {
        return Err(Error::Denied);
    }
    let code = query
        .code
        .filter(|code| !code.is_empty() && code.len() <= 4096)
        .ok_or(Error::Denied)?;
    let work = shared.clone();
    let session = shared
        .auth
        .run(async move {
            let (credentials, actor) = Credentials::exchange(&work, &config, attempt, code).await?;
            let user_id = rom_identity::linked_user_id(&actor).ok_or(Error::Denied)?;
            work.sessions.insert(
                SessionEvidence {
                    actor,
                    user_id,
                    token_expiry: credentials.expiry,
                    credentials: Some(Arc::new(credentials)),
                },
                work.config.clock.now(),
            )
        })
        .await?;
    if let Ok(old) = csrf::session_cookie(&headers, "rom_session") {
        shared.sessions.remove(&old);
    }
    let mut response = Redirect::to(&shared.config.base_path).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        cookie(
            &shared,
            "rom_session",
            session.cookie(),
            session
                .expires_at()
                .saturating_sub(shared.config.clock.now()),
        )?,
    );
    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie(&shared, "rom_login", "", 0)?);
    Ok(no_store(response))
}
pub(crate) fn cookie(
    shared: &Shared,
    name: &str,
    value: &str,
    seconds: u64,
) -> Result<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{name}={value}; Path={}; Max-Age={seconds}; HttpOnly; SameSite=Lax{}",
        shared.config.base_path,
        if shared.config.secure() {
            "; Secure"
        } else {
            ""
        }
    ))
    .map_err(|_| Error::Denied)
}

pub(crate) fn failure(error: Error) -> Response {
    let (status, category) = match error {
        Error::Overloaded => (StatusCode::SERVICE_UNAVAILABLE, "overloaded"),
        Error::Closed => (StatusCode::SERVICE_UNAVAILABLE, "closed"),
        Error::Panicked => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        _ => (StatusCode::UNAUTHORIZED, "denied"),
    };
    no_store((status, axum::Json(serde_json::json!({"category":category}))).into_response())
}
