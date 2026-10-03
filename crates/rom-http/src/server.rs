use crate::routes::{discover, invoke, journal, journal_head, live, query, read, subscribe};
use axum::{Router, http::HeaderMap, routing::post};
use rom::{Actor, Error, Runtime};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::{Semaphore, watch};

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
pub(super) struct Shared {
    pub(super) runtime: Runtime,
    pub(super) auth: AuthResolver,
    pub(super) limits: Limits,
    pub(super) bodies: Arc<Semaphore>,
    pub(super) closed: watch::Sender<bool>,
}
#[derive(Clone)]
pub struct Http {
    shared: Shared,
}
impl Http {
    pub fn new(runtime: Runtime, auth: AuthResolver, limits: Limits) -> rom::Result<Self> {
        if limits.body_bytes == 0
            || limits.bodies == 0
            || limits.bodies > Semaphore::MAX_PERMITS
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
            .route("/discover", post(discover))
            .route("/invoke", post(invoke))
            .route("/read", post(read))
            .route("/query", post(query))
            .route("/live", post(live))
            .route("/journal", post(journal))
            .route("/journal/head", post(journal_head))
            .route("/subscribe", post(subscribe))
            .route("/work/capabilities", post(crate::operator::capabilities))
            .route("/work/list", post(crate::operator::list))
            .route("/work/read", post(crate::operator::read))
            .route("/work/control", post(crate::operator::control))
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
