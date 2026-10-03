//! Host-owned Unix signal registration before serving readiness.
use rom::{Error, Result};
use tokio::signal::unix::{Signal, SignalKind, signal};

pub(crate) struct SignalReceiver {
    interrupt: Signal,
    termination: Signal,
}
impl SignalReceiver {
    pub(crate) fn install() -> Result<Self> {
        Ok(Self {
            interrupt: signal(SignalKind::interrupt()).map_err(|_| Error::Storage)?,
            termination: signal(SignalKind::terminate()).map_err(|_| Error::Storage)?,
        })
    }
    pub(crate) async fn wait(mut self) {
        tokio::select! {_ = self.interrupt.recv()=>{},_ = self.termination.recv()=>{}}
    }
}
