use rom::{Error, Result};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
};
use tokio::{
    sync::{Semaphore, oneshot},
    task::JoinHandle,
};

struct Jobs {
    tasks: Vec<JoinHandle<()>>,
    failed: bool,
}
pub(crate) struct Supervisor {
    admission: Arc<Semaphore>,
    jobs: Mutex<Jobs>,
}
impl Supervisor {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            admission: Arc::new(Semaphore::new(capacity)),
            jobs: Mutex::new(Jobs {
                tasks: Vec::new(),
                failed: false,
            }),
        }
    }
    pub(crate) async fn run<T, F>(&self, work: F) -> Result<T>
    where
        T: Send + 'static,
        F: Future<Output = Result<T>> + Send + 'static,
    {
        let permit = self.admission.clone().try_acquire_owned().map_err(|_| {
            if self.admission.is_closed() {
                Error::Closed
            } else {
                Error::Overloaded
            }
        })?;
        let (send, receive) = oneshot::channel();
        {
            let mut jobs = self.jobs.lock().map_err(|_| Error::Panicked)?;
            if self.admission.is_closed() {
                return Err(Error::Closed);
            }
            // Completed callbacks already report their result. Keep unfinished tasks owned.
            jobs.tasks.retain(|handle| !handle.is_finished());
            jobs.tasks.push(tokio::spawn(async move {
                let result = work.await;
                let _ = send.send(result);
                drop(permit);
            }));
        }
        receive.await.map_err(|_| Error::Panicked)?
    }
    pub(crate) fn close(&self) {
        self.admission.close();
    }
    pub(crate) async fn drain(&self) -> Result<()> {
        self.close();
        poll_fn(|cx| {
            let Ok(mut jobs) = self.jobs.lock() else {
                return Poll::Ready(Err(Error::Panicked));
            };
            let mut index = 0;
            while index < jobs.tasks.len() {
                match Pin::new(&mut jobs.tasks[index]).poll(cx) {
                    Poll::Ready(result) => {
                        if result.is_err() {
                            jobs.failed = true;
                        }
                        drop(jobs.tasks.swap_remove(index));
                    }
                    Poll::Pending => index += 1,
                }
            }
            if jobs.tasks.is_empty() {
                Poll::Ready(if jobs.failed {
                    Err(Error::Panicked)
                } else {
                    Ok(())
                })
            } else {
                Poll::Pending
            }
        })
        .await
    }
}
