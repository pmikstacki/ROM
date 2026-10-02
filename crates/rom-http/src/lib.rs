//! HTTP wire binding. The host resolver establishes trusted identity; request JSON never does.
//! Every route is generic across registered Resource kinds. TLS and credential verification
//! belong to the host. Use `serve` to coordinate stream termination and runtime draining.
#![forbid(unsafe_code)]
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::post,
};
use futures_util::stream;
use rom::{Actor, Error, Invocation, JournalCursor, Runtime, Value};
use serde::Deserialize;
use std::{convert::Infallible, future::Future, sync::Arc, time::Duration};
use tokio::sync::{Semaphore, watch};
mod json;

/// Must be fast and nonblocking. The host verifies credentials before returning an Actor.
/// Arbitrary subject/authority headers must never be treated as proof of identity.
pub type AuthResolver = Arc<dyn Fn(&HeaderMap) -> rom::Result<Actor> + Send + Sync>;
#[derive(Clone, Copy)]
pub struct Limits {
    pub body_bytes: usize,
    pub bodies: usize,
    pub body_timeout: Duration,
    pub observation_poll: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            body_bytes: 16 * 1024,
            bodies: 16,
            body_timeout: Duration::from_secs(5),
            observation_poll: Duration::from_millis(100),
        }
    }
}
#[derive(Clone)]
struct Shared {
    runtime: Runtime,
    auth: AuthResolver,
    limits: Limits,
    bodies: Arc<Semaphore>,
    closed: watch::Sender<bool>,
}
#[derive(Clone)]
pub struct Http {
    shared: Shared,
}
impl Http {
    pub fn new(runtime: Runtime, auth: AuthResolver, limits: Limits) -> rom::Result<Self> {
        if limits.body_bytes == 0
            || limits.bodies == 0
            || limits.body_timeout.is_zero()
            || limits.observation_poll.is_zero()
        {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            shared: Shared {
                runtime,
                auth,
                limits,
                bodies: Arc::new(Semaphore::new(limits.bodies)),
                closed: watch::channel(false).0,
            },
        })
    }
    pub fn router(&self) -> Router {
        Router::new()
            .route("/invoke", post(invoke))
            .route("/read", post(read))
            .route("/query", post(query))
            .route("/live", post(live))
            .route("/journal", post(journal))
            .route("/journal/head", post(journal_head))
            .route("/subscribe", post(subscribe))
            .with_state(self.shared.clone())
    }
    /// Close application intake and observers; drain runtime-owned accepted work.
    pub async fn shutdown(&self) -> rom::Result<()> {
        self.shared.bodies.close();
        self.shared.closed.send_replace(true);
        self.shared.runtime.shutdown().await
    }
    pub async fn serve(
        self,
        listener: tokio::net::TcpListener,
        stop: impl Future<Output = ()> + Send + 'static,
    ) -> rom::Result<()> {
        let shutting_down = self.clone();
        axum::serve(listener, self.router())
            .with_graceful_shutdown(async move {
                stop.await;
                shutting_down.shared.bodies.close();
                shutting_down.shared.closed.send_replace(true);
                // Keep graceful HTTP shutdown independent of a cancelled runtime drain waiter.
                tokio::spawn(async move { shutting_down.shared.runtime.shutdown().await });
            })
            .await
            .map_err(|_| Error::Storage)?;
        self.shutdown().await
    }
}
struct Failure(Error);
impl From<Error> for Failure {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
fn category(e: &Error) -> (&'static str, StatusCode) {
    match e {
        Error::Denied => ("denied", StatusCode::FORBIDDEN),
        Error::Missing | Error::Unregistered => ("missing", StatusCode::NOT_FOUND),
        Error::Conflict => ("conflict", StatusCode::CONFLICT),
        Error::IdentityMismatch => ("identity_mismatch", StatusCode::CONFLICT),
        Error::HistoryGap => ("history_gap", StatusCode::GONE),
        Error::TooLarge => ("too_large", StatusCode::PAYLOAD_TOO_LARGE),
        Error::Overloaded => ("overloaded", StatusCode::TOO_MANY_REQUESTS),
        Error::Closed => ("closed", StatusCode::SERVICE_UNAVAILABLE),
        Error::Unknown => ("outcome_unknown", StatusCode::SERVICE_UNAVAILABLE),
        Error::NotCommitted => ("not_committed", StatusCode::SERVICE_UNAVAILABLE),
        Error::Invalid { .. } | Error::Unsupported(_) | Error::Duplicate(_) => {
            ("invalid", StatusCode::BAD_REQUEST)
        }
        _ => ("internal", StatusCode::INTERNAL_SERVER_ERROR),
    }
}
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let (code, status) = category(&self.0);
        (status, Json(serde_json::json!({"error":code}))).into_response()
    }
}
async fn decode<T: serde::de::DeserializeOwned>(
    shared: &Shared,
    request: Request,
) -> Result<(Actor, T, tokio::sync::OwnedSemaphorePermit), Failure> {
    if *shared.closed.borrow() {
        return Err(Error::Closed.into());
    }
    let permit = shared.bodies.clone().try_acquire_owned().map_err(|e| {
        Failure(match e {
            tokio::sync::TryAcquireError::Closed => Error::Closed,
            _ => Error::Overloaded,
        })
    })?;
    let actor = (shared.auth)(request.headers())?;
    let bytes = tokio::time::timeout(
        shared.limits.body_timeout,
        to_bytes(request.into_body(), shared.limits.body_bytes),
    )
    .await
    .map_err(|_| Failure(Error::Overloaded))?
    .map_err(|_| Failure(Error::TooLarge))?;
    Ok((actor, json::parse(&bytes)?, permit))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    kind: String,
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EqualityQuery {
    kind: String,
    field: String,
    value: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructuredQuery {
    kind: String,
    query: rom::QuerySpec,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Query {
    Structured(StructuredQuery),
    Equality(EqualityQuery),
}
impl Query {
    fn into_spec(self) -> (String, rom::QuerySpec) {
        match self {
            Self::Structured(q) => (q.kind, q.query),
            Self::Equality(q) => (q.kind, rom::QuerySpec::equal(q.field, q.value)),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    kind: String,
    after: Option<JournalCursor>,
}
async fn invoke(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Invocation>(&s, r).await?;
    Ok(Json(s.runtime.invoke_projected(&a, c).await?).into_response())
}
async fn read(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Read>(&s, r).await?;
    Ok(Json(s.runtime.read_projected(&a, &c.kind, &c.id).await?).into_response())
}
async fn query(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Query>(&s, r).await?;
    let (kind, spec) = c.into_spec();
    Ok(Json(s.runtime.query_spec_projected(&a, &kind, spec).await?).into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalHead {
    kind: String,
}
async fn journal_head(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<JournalHead>(&s, r).await?;
    Ok(Json(s.runtime.journal_head(&a, &c.kind).await?).into_response())
}
async fn journal(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Journal>(&s, r).await?;
    Ok(Json(s.runtime.journal(&a, &c.kind, c.after.as_ref()).await?).into_response())
}
async fn live(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Query>(&s, r).await?;
    let (kind, spec) = c.into_spec();
    let handle = s.runtime.live_spec_projected(&a, &kind, spec).await?;
    Ok(observe(s, a, handle, |handle| {
        Box::pin(async move { handle.changed().await })
    }))
}
async fn subscribe(State(s): State<Shared>, r: Request) -> Result<Response, Failure> {
    let (a, c, _permit) = decode::<Journal>(&s, r).await?;
    let handle = s.runtime.subscribe(&a, &c.kind, c.after).await?;
    Ok(observe(s, a, handle, |handle| {
        Box::pin(async move { handle.next().await })
    }))
}
type PendingObservation<H, T> = std::pin::Pin<Box<dyn Future<Output = (H, rom::Result<T>)> + Send>>;

fn pending<H, T, F>(mut handle: H, next: Arc<F>) -> PendingObservation<H, T>
where
    H: Send + 'static,
    T: Send + 'static,
    F: for<'a> Fn(&'a mut H) -> std::pin::Pin<Box<dyn Future<Output = rom::Result<T>> + Send + 'a>>
        + Send
        + Sync
        + 'static,
{
    Box::pin(async move {
        let result = next(&mut handle).await;
        (handle, result)
    })
}

fn observe<H, T, F>(s: Shared, actor: Actor, handle: H, next: F) -> Response
where
    H: Send + 'static,
    T: serde::Serialize + Send + 'static,
    F: for<'a> Fn(&'a mut H) -> std::pin::Pin<Box<dyn Future<Output = rom::Result<T>> + Send + 'a>>
        + Send
        + Sync
        + 'static,
{
    let closed = s.closed.subscribe();
    let next = Arc::new(next);
    let events = stream::unfold(
        Some((pending(handle, next.clone()), closed, next)),
        move |state| {
            let poll = s.limits.observation_poll;
            let runtime = s.runtime.clone();
            let actor = actor.clone();
            async move {
                let (mut pending_read, mut closed, next) = state?;
                if *closed.borrow() {
                    return None;
                }
                if let Err(error) = runtime.observation_status(&actor) {
                    return Some((
                        Ok::<_, Infallible>(
                            Event::default().event("error").data(category(&error).0),
                        ),
                        None,
                    ));
                }
                let result = tokio::select! {
                    _=closed.changed()=>return None,
                    result=tokio::time::timeout(poll,&mut pending_read)=>result,
                };
                if let Err(error) = runtime.observation_status(&actor) {
                    return Some((
                        Ok(Event::default().event("error").data(category(&error).0)),
                        None,
                    ));
                }
                match result {
                    Err(_) => Some((
                        Ok(Event::default().comment("keepalive")),
                        Some((pending_read, closed, next)),
                    )),
                    Ok((handle, Ok(value))) => {
                        match Event::default().event("data").json_data(value) {
                            Ok(event) => Some((
                                Ok(event),
                                Some((pending(handle, next.clone()), closed, next)),
                            )),
                            Err(_) => {
                                Some((Ok(Event::default().event("error").data("internal")), None))
                            }
                        }
                    }
                    Ok((_, Err(error))) => Some((
                        Ok(Event::default().event("error").data(category(&error).0)),
                        None,
                    )),
                }
            }
        },
    );
    Sse::new(events).into_response()
}
