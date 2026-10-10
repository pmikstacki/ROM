//! A bounded pure calculation exposes real physical ownership across caller timeout.
use std::{sync::{Condvar, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}}, time::Duration};

pub struct ReadGate {
    released: Mutex<bool>,
    changed: Condvar,
    pub started: AtomicBool,
    pub calls: AtomicU64,
}
impl ReadGate {
    pub fn new() -> Self {
        Self { released: Mutex::new(false), changed: Condvar::new(), started: AtomicBool::new(false), calls: AtomicU64::new(0) }
    }
    pub fn wait(&self) -> rom::Result<bool> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.started.store(true, Ordering::SeqCst);
        let (released, _) = self.changed.wait_timeout_while(
            self.released.lock().map_err(|_| rom::Error::Panicked)?,
            Duration::from_secs(20), |value| !*value,
        ).map_err(|_| rom::Error::Panicked)?;
        if *released { Ok(true) } else { Err(rom::Error::Closed) }
    }
    pub fn release(&self) -> rom::Result<()> {
        *self.released.lock().map_err(|_| rom::Error::Panicked)? = true;
        self.changed.notify_all();
        Ok(())
    }
}
