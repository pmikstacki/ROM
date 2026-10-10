//! One owned fixture worker: setup observation never cancels physical work.
use super::ReadBlock;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::oneshot, task::JoinHandle};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum StartupError {
    SetupTimeout,
    SignalClosed,
    CompletionTimeout,
    ExitedBeforeEntry,
    Worker(rom::Error),
    Panicked(String),
}
#[derive(Debug)]
pub(super) struct Terminal {
    pub(super) outcome: Result<(), StartupError>,
    pub(super) observed_at: Instant,
}
pub(super) struct Worker {
    block: Arc<ReadBlock>,
    handle: Option<JoinHandle<rom::Result<()>>>,
    terminal: Option<Terminal>,
}
impl Worker {
    pub(super) fn new(block: Arc<ReadBlock>, handle: JoinHandle<rom::Result<()>>) -> Self {
        Self {
            block,
            handle: Some(handle),
            terminal: None,
        }
    }
    pub(super) fn terminal(&self) -> Option<&Terminal> {
        self.terminal.as_ref()
    }
    pub(super) fn pending(&self) -> bool {
        self.handle.is_some()
    }
    fn completed(
        &mut self,
        result: Result<rom::Result<()>, tokio::task::JoinError>,
    ) -> Result<(), StartupError> {
        drop(self.handle.take());
        let outcome = match result {
            Ok(outcome) => outcome.map_err(StartupError::Worker),
            Err(error) => Err(StartupError::Panicked(error.to_string())),
        };
        self.terminal = Some(Terminal {
            outcome: outcome.clone(),
            observed_at: Instant::now(),
        });
        outcome
    }
    pub(super) async fn ready(
        &mut self,
        mut entered: oneshot::Receiver<Instant>,
        bound: Duration,
    ) -> Result<Instant, StartupError> {
        let handle = self
            .handle
            .as_mut()
            .expect("worker readiness observed once");
        enum Observation {
            Entry(Result<Instant, oneshot::error::RecvError>),
            Finished(Result<rom::Result<()>, tokio::task::JoinError>),
        }
        let observation = tokio::time::timeout(bound, async {
            tokio::select! { biased;
                entered = &mut entered => Observation::Entry(entered),
                result = handle => Observation::Finished(result),
            }
        })
        .await
        .map_err(|_| StartupError::SetupTimeout)?;
        match observation {
            Observation::Entry(entered) => entered.map_err(|_| StartupError::SignalClosed),
            Observation::Finished(result) => {
                self.completed(result)?;
                Err(StartupError::ExitedBeforeEntry)
            }
        }
    }
    pub(super) async fn finish(&mut self, bound: Duration) -> Result<(), StartupError> {
        let Some(handle) = self.handle.as_mut() else {
            return self
                .terminal
                .as_ref()
                .expect("consumed worker has a retained terminal")
                .outcome
                .clone();
        };
        let result = tokio::time::timeout(bound, handle)
            .await
            .map_err(|_| StartupError::CompletionTimeout)?;
        self.completed(result)
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        // Cancellation releases the fixture child. Drop does not prove worker join or Runtime drain.
        self.block.release();
    }
}
