//! Trusted calculations use the existing pool, admission and shutdown ownership.
use super::Runtime;
use crate::{Error, Result};
use std::panic::{AssertUnwindSafe, catch_unwind};

impl Runtime {
    /// Run a terminating synchronous calculation in this Runtime's shared Rayon pool.
    ///
    /// Admission shares `Limits::io_jobs` with storage work and returns
    /// `Error::Overloaded` when full. The closure receives no Runtime or actor.
    /// It must not block on nested Runtime work. Captured application handles
    /// remain trusted; this method is not a sandbox or Resource mutation API.
    /// Join all child work before returning. Detached threads or Rayon tasks
    /// started by the closure are not tracked by this Runtime.
    ///
    /// Cancelling the returned future stops waiting, not the calculation.
    /// Actual work retains its permit and participates in `shutdown` until it
    /// ends. A nonterminating closure can therefore prevent shutdown completion.
    /// Wrap the future in a caller deadline when needed; that deadline cannot
    /// preempt synchronous code. A caught panic returns `Error::Panicked` without
    /// closing this Runtime. The process panic hook can still emit diagnostics.
    pub async fn calculate<T, F>(&self, calculation: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        self.io(move |runtime| {
            runtime.0.pool.install(|| {
                catch_unwind(AssertUnwindSafe(calculation)).unwrap_or(Err(Error::Panicked))
            })
        })
        .await
    }
}
