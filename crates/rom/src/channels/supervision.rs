//! Shared cooperative async cancellation for delivery and read-only reconciliation.
use std::{future::Future, time::Duration};
pub(crate) enum Callback<T> {
    Returned(T),
    Panicked,
    TimedOut,
}
/// Called inside existing supervised blocking I/O; cancellation retains its permit until join.
/// Host futures must yield; abort cannot preempt arbitrary blocking Rust code.
pub(crate) fn run<F, Fut, T>(timeout: Duration, callback: F) -> Callback<T>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    tokio::runtime::Handle::current().block_on(async move {
        let mut task = tokio::spawn(async move { callback().await });
        match tokio::time::timeout(timeout, &mut task).await {
            Ok(Ok(value)) => Callback::Returned(value),
            Ok(Err(_)) => Callback::Panicked,
            Err(_) => {
                task.abort();
                let _ = task.await;
                Callback::TimedOut
            }
        }
    })
}
