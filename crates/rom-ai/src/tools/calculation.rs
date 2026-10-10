//! Restricted pure CPU work shares Runtime supervision and the original execution deadline.
use super::ReadContext;
use std::time::Duration;
impl ReadContext {
    /// Run a trusted synchronous calculation on the existing supervised CPU pool.
    /// The closure must join its child work and must not block on nested Runtime calls.
    /// Cancellation stops waiting; the shared I/O admission remains held until calculation ends.
    pub async fn calculate<T, F>(&self, calculate: F) -> rom::Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> rom::Result<T> + Send + 'static,
    {
        let milliseconds = self
            .deadline
            .remaining_ms()
            .map_err(|_| rom::Error::Closed)?;
        let lease = self.lease.clone();
        tokio::time::timeout(
            Duration::from_millis(milliseconds),
            self.runtime.calculate(move || {
                let _physical_lease = lease;
                calculate()
            }),
        )
        .await
        .map_err(|_| rom::Error::Closed)?
    }
}
