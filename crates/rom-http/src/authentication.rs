//! Host identity resolution shared by every generic HTTP route.
use axum::http::HeaderMap;
use rom::Actor;
use std::{future::Future, pin::Pin, sync::Arc};
/// Must be fast and nonblocking. The host verifies credentials before returning an Actor.
/// Arbitrary subject/authority headers must never be treated as proof of identity.
pub type AuthResolver = Arc<dyn Fn(&HeaderMap) -> rom::Result<Actor> + Send + Sync>;
/// Authentication result future supplied by the host.
pub type AuthFuture = Pin<Box<dyn Future<Output = rom::Result<Actor>> + Send>>;
/// Resolve owned request headers to a verified Actor before body decoding.
/// The host owns finite deadlines, bounded admission and cancellation supervision.
/// This callback alone supplies none of those guarantees. Do not block async workers
/// or trust request Actor metadata, subject headers or host stamps.
pub type AsyncAuthResolver = Arc<dyn Fn(HeaderMap) -> AuthFuture + Send + Sync>;
#[derive(Clone)]
pub(super) enum Resolver {
    Sync(AuthResolver),
    Async(AsyncAuthResolver),
}
impl Resolver {
    pub(super) async fn resolve(&self, headers: HeaderMap) -> rom::Result<Actor> {
        match self {
            Self::Sync(auth) => auth(&headers),
            Self::Async(auth) => auth(headers).await,
        }
    }
}
