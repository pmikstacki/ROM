use super::*;
use futures_util::{FutureExt, StreamExt};
use std::{
    collections::BTreeMap,
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot, watch};
#[derive(Clone, Copy)]
pub struct Limits {
    pub operations: usize,
    pub blob_bytes: usize,
    pub chunk_bytes: usize,
    pub chunks: usize,
    pub staging_timeout: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            operations: 4,
            blob_bytes: 1024 * 1024,
            chunk_bytes: 64 * 1024,
            chunks: 1024,
            staging_timeout: Duration::from_secs(10),
        }
    }
}
#[derive(Clone, Debug)]
pub struct ObjectReceipt {
    pub key: ObjectKey,
    pub digest: Digest,
    pub bytes: u64,
}
#[derive(Clone, Debug)]
pub enum UploadOutcome {
    Attached(rom::Snapshot<Blob>),
    /// Complete object exists but metadata was not acknowledged; inspect/retry before cleanup.
    Unattached {
        object: ObjectReceipt,
        cause: rom::Error,
    },
}
#[derive(Default)]
struct Lifecycle {
    closed: bool,
    active: usize,
    panicked: bool,
}
struct Inner {
    runtime: rom::Runtime,
    stores: BTreeMap<String, Arc<dyn BlobStore>>,
    limits: Limits,
    admission: Arc<Semaphore>,
    lifecycle: Mutex<Lifecycle>,
    changed: watch::Sender<u64>,
}
struct Work {
    inner: Arc<Inner>,
    _permit: OwnedSemaphorePermit,
}
impl Drop for Work {
    fn drop(&mut self) {
        self.inner.lifecycle.lock().unwrap().active -= 1;
        self.inner
            .changed
            .send_modify(|value| *value = value.wrapping_add(1));
    }
}
/// Host-owned lifecycle. Drain this service before shutting down its core Runtime.
#[derive(Clone)]
pub struct BlobService(Arc<Inner>);
pub struct BlobServiceBuilder {
    runtime: rom::Runtime,
    stores: BTreeMap<String, Arc<dyn BlobStore>>,
    limits: Limits,
    duplicate: bool,
}
impl BlobServiceBuilder {
    pub fn store(mut self, name: &str, store: Arc<dyn BlobStore>) -> Self {
        self.duplicate |= self.stores.insert(name.into(), store).is_some();
        self
    }
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
    pub fn build(self) -> Result<BlobService> {
        if self.duplicate {
            return Err(Error::Invalid);
        }
        BlobService::new(self.runtime, self.stores, self.limits)
    }
}
impl BlobService {
    pub fn builder(runtime: rom::Runtime) -> BlobServiceBuilder {
        BlobServiceBuilder {
            runtime,
            stores: BTreeMap::new(),
            limits: Limits::default(),
            duplicate: false,
        }
    }
    pub fn new(
        runtime: rom::Runtime,
        stores: BTreeMap<String, Arc<dyn BlobStore>>,
        limits: Limits,
    ) -> Result<Self> {
        if stores.is_empty()
            || stores.len() > 32
            || stores.keys().any(|s| s.is_empty() || s.len() > 64)
            || limits.operations == 0
            || limits.operations > 64
            || limits.blob_bytes == 0
            || limits.blob_bytes > 16 * 1024 * 1024
            || limits.chunk_bytes == 0
            || limits.chunk_bytes > limits.blob_bytes
            || limits.chunks == 0
            || limits.chunks > 65_536
            || limits.staging_timeout.is_zero()
        {
            return Err(Error::Invalid);
        }
        Ok(Self(Arc::new(Inner {
            runtime,
            stores,
            limits,
            admission: Arc::new(Semaphore::new(limits.operations)),
            lifecycle: Mutex::new(Lifecycle::default()),
            changed: watch::channel(0).0,
        })))
    }
    async fn run<T: Send + 'static>(
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
                inner: self.0.clone(),
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
                drop(work);
                let _ = sender.send(result);
            });
        }
        receiver.await.map_err(|_| Error::Panicked)?
    }
    pub async fn reserve(
        &self,
        actor: &Actor,
        id: &str,
        store: &str,
        digest: Digest,
        bytes: u64,
        idempotency: &str,
    ) -> Result<rom::Snapshot<Blob>> {
        let (actor, id, store, idempotency) = (
            actor.clone(),
            id.to_owned(),
            store.to_owned(),
            idempotency.to_owned(),
        );
        self.run(move |inner| {
            Box::pin(async move {
                if bytes > inner.limits.blob_bytes as u64 || !inner.stores.contains_key(&store) {
                    return Err(Error::Invalid);
                }
                Ok(inner
                    .runtime
                    .execute(
                        &actor,
                        rom::Command::create(
                            &id,
                            Blob {
                                owner: owner(&actor),
                                store,
                                digest,
                                bytes,
                                state: BlobState::Pending,
                                upload_revision: 0,
                            },
                        )
                        .idempotency(&idempotency),
                    )
                    .await?)
            })
        })
        .await
    }
    /// Consume and verify complete input, then publish and conditionally attach.
    /// Cancellation of this waiter does not cancel accepted work.
    pub async fn upload(&self, actor: &Actor, id: &str, input: Upload) -> Result<UploadOutcome> {
        let (actor, id) = (actor.clone(), id.to_owned());
        self.run(move |inner| {
            Box::pin(async move {
                let initial = inner.runtime.read::<Blob>(&actor, &id).await?;
                let mut blob = initial.value.clone().ok_or(Error::Missing)?;
                if blob.state == BlobState::Detached {
                    return Err(Error::Conflict);
                }
                if blob.bytes > inner.limits.blob_bytes as u64 {
                    return Err(Error::TooLarge);
                }
                let bytes =
                    tokio::time::timeout(inner.limits.staging_timeout, stage(input, inner.limits))
                        .await
                        .map_err(|_| Error::Timeout)??;
                let (bytes, digest) = hash(bytes).await?;
                if digest != blob.digest || bytes.len() as u64 != blob.bytes {
                    return Err(Error::Conflict);
                }
                let current = inner.runtime.read::<Blob>(&actor, &id).await?;
                if current.revision != initial.revision {
                    return Err(Error::Conflict);
                }
                if blob.state == BlobState::Ready {
                    return Ok(UploadOutcome::Attached(current));
                }
                let store = inner.stores.get(&blob.store).ok_or(Error::Invalid)?;
                let object = receipt(&id, initial.revision, &blob);
                match store.create(&object.key, bytes).await {
                    Ok(()) => {}
                    Err(Error::Conflict) => {
                        let (existing, digest) =
                            hash(store.get(&object.key, inner.limits.blob_bytes).await?).await?;
                        if digest != blob.digest || existing.len() as u64 != blob.bytes {
                            return Err(Error::Conflict);
                        }
                    }
                    Err(error) => return Err(error),
                }
                // The conditional core mutation arbitrates reservation edits. The worker
                // identity represents completion of accepted work, not a forged caller.
                if let Err(cause) = inner.runtime.read::<Blob>(&actor, &id).await {
                    return Ok(UploadOutcome::Unattached { object, cause });
                }
                blob.state = BlobState::Ready;
                blob.upload_revision = initial.revision;
                let committed = inner
                    .runtime
                    .execute(
                        &worker_actor(),
                        rom::Command::replace(&id, blob)
                            .at_revision(initial.revision)
                            .idempotency(&format!("attach-{}", initial.revision)),
                    )
                    .await;
                match committed {
                    Ok(_) => {
                        let current = inner.runtime.read::<Blob>(&actor, &id).await?;
                        if current.revision != initial.revision + 1
                            || current
                                .value
                                .as_ref()
                                .is_none_or(|blob| blob.state != BlobState::Ready)
                        {
                            return Ok(UploadOutcome::Unattached {
                                object,
                                cause: rom::Error::Conflict,
                            });
                        }
                        Ok(UploadOutcome::Attached(current))
                    }
                    Err(cause) => Ok(UploadOutcome::Unattached { object, cause }),
                }
            })
        })
        .await
    }
    pub async fn read(&self, actor: &Actor, id: &str) -> Result<Vec<u8>> {
        let (actor, id) = (actor.clone(), id.to_owned());
        self.run(move |inner| {
            Box::pin(async move {
                let initial = inner.runtime.read::<Blob>(&actor, &id).await?;
                let blob = initial.value.ok_or(Error::Missing)?;
                if blob.state != BlobState::Ready || blob.upload_revision == 0 {
                    return Err(Error::Missing);
                }
                if blob.bytes > inner.limits.blob_bytes as u64 {
                    return Err(Error::TooLarge);
                }
                let store = inner.stores.get(&blob.store).ok_or(Error::Invalid)?;
                let object = receipt(&id, blob.upload_revision, &blob);
                let (bytes, digest) =
                    hash(store.get(&object.key, inner.limits.blob_bytes).await?).await?;
                if digest != blob.digest || bytes.len() as u64 != blob.bytes {
                    return Err(Error::Conflict);
                }
                let current = inner.runtime.read::<Blob>(&actor, &id).await?;
                if current.revision != initial.revision {
                    return Err(Error::Conflict);
                }
                Ok(bytes)
            })
        })
        .await
    }
    /// Commit logical detachment before the host considers physical cleanup.
    /// The returned opaque receipt is maintenance evidence, not authorization to delete.
    pub async fn detach(&self, actor: &Actor, id: &str) -> Result<ObjectReceipt> {
        let (actor, id) = (actor.clone(), id.to_owned());
        self.run(move |inner| {
            Box::pin(async move {
                let initial = inner.runtime.read::<Blob>(&actor, &id).await?;
                let mut blob = initial.value.ok_or(Error::Missing)?;
                let revision = if blob.upload_revision == 0 {
                    initial.revision
                } else {
                    blob.upload_revision
                };
                let object = receipt(&id, revision, &blob);
                if blob.state != BlobState::Detached {
                    blob.state = BlobState::Detached;
                    blob.upload_revision = revision;
                    inner
                        .runtime
                        .execute(
                            &worker_actor(),
                            rom::Command::replace(&id, blob)
                                .at_revision(initial.revision)
                                .idempotency(&format!("detach-{}", initial.revision)),
                        )
                        .await?;
                }
                inner.runtime.read::<Blob>(&actor, &id).await?;
                Ok(object)
            })
        })
        .await
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
fn receipt(id: &str, revision: u64, blob: &Blob) -> ObjectReceipt {
    let key = Digest::of(
        rom::json!([
            blob.owner,
            id,
            revision,
            blob.store,
            blob.digest.as_str(),
            blob.bytes
        ])
        .to_string()
        .as_bytes(),
    );
    ObjectReceipt {
        key: ObjectKey(key),
        digest: blob.digest.clone(),
        bytes: blob.bytes,
    }
}
async fn hash(bytes: Vec<u8>) -> Result<(Vec<u8>, Digest)> {
    tokio::task::spawn_blocking(move || {
        let digest = Digest::of(&bytes);
        (bytes, digest)
    })
    .await
    .map_err(|_| Error::Panicked)
}
async fn stage(mut input: Upload, limits: Limits) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut count = 0usize;
    while let Some(chunk) = input.next().await {
        count += 1;
        let chunk = chunk?;
        if count > limits.chunks
            || chunk.len() > limits.chunk_bytes
            || chunk.len() > limits.blob_bytes - bytes.len()
        {
            return Err(Error::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
