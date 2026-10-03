//! Runtime-owned admission, accepted-work supervision and orderly shutdown.
use super::Limits;
use crate::{BlobStore, Error, Result};
use futures_util::FutureExt;
use std::{
    collections::BTreeMap,
    future::Future,
    panic::AssertUnwindSafe,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot, watch};
#[derive(Default)]
pub(super) struct Lifecycle {
    closed: bool,
    active: usize,
    panicked: bool,
}
pub(super) struct Inner {
    pub(super) runtime: rom::Runtime,
    pub(super) stores: BTreeMap<String, Arc<dyn BlobStore>>,
    pub(super) limits: Limits,
    pub(super) admission: Arc<Semaphore>,
    pub(super) lifecycle: Arc<Mutex<Lifecycle>>,
    pub(super) changed: watch::Sender<u64>,
}
struct Work {
    lifecycle: Arc<Mutex<Lifecycle>>,
    changed: watch::Sender<u64>,
    _permit: OwnedSemaphorePermit,
}
impl Drop for Work {
    fn drop(&mut self) {
        self.lifecycle.lock().unwrap().active -= 1;
        self.changed
            .send_modify(|value| *value = value.wrapping_add(1));
    }
}
/// Host-owned lifecycle. Drain this service before shutting down its core Runtime.
#[derive(Clone)]
pub struct BlobService(pub(super) Arc<Inner>);
impl BlobService {
    pub(super) async fn run<T: Send + 'static>(
        &self,
        f: impl FnOnce(Arc<Inner>) -> Pin<Box<dyn Future<Output = Result<T>> + Send>> + Send + 'static,
    ) -> Result<T> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.0.lifecycle.lock().unwrap();
            if state.closed {
                return Err(Error::Closed);
            }
            let permit = self
                .0
                .admission
                .clone()
                .try_acquire_owned()
                .map_err(|_| Error::Overloaded)?;
            state.active += 1;
            let work = Work {
                lifecycle: self.0.lifecycle.clone(),
                changed: self.0.changed.clone(),
                _permit: permit,
            };
            let inner = self.0.clone();
            tokio::spawn(async move {
                let result = AssertUnwindSafe(async { f(inner.clone()).await })
                    .catch_unwind()
                    .await;
                let result = match result {
                    Ok(value) => value,
                    Err(_) => {
                        let mut state = inner.lifecycle.lock().unwrap();
                        state.panicked = true;
                        state.closed = true;
                        inner.admission.close();
                        Err(Error::Panicked)
                    }
                };
                // Release runtime/provider ownership before advertising quiescence.
                drop(inner);
                drop(work);
                let _ = sender.send(result);
            });
        }
        receiver.await.map_err(|_| Error::Panicked)?
    }
    pub async fn shutdown(&self) -> Result<()> {
        let mut changed = self.0.changed.subscribe();
        {
            let mut state = self.0.lifecycle.lock().unwrap();
            state.closed = true;
            self.0.admission.close();
        }
        loop {
            {
                let state = self.0.lifecycle.lock().unwrap();
                if state.active == 0 {
                    return if state.panicked {
                        Err(Error::Panicked)
                    } else {
                        Ok(())
                    };
                }
            }
            changed.changed().await.map_err(|_| Error::Panicked)?;
        }
    }
}
