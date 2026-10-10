//! Authorized reservation, upload, read and logical detachment.
use super::{BlobService, ObjectReceipt, UploadOutcome};
use crate::resource::owner;
use crate::staging::{hash, stage};
use crate::{Blob, BlobState, BlobStore, Digest, Error, ObjectKey, Result, Upload, worker_actor};
use rom::Actor;
impl BlobService {
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
                        read_verified(store.as_ref(), &object.key, &blob, inner.limits.blob_bytes)
                            .await?;
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
                        let current = match inner.runtime.read::<Blob>(&actor, &id).await {
                            Ok(current) => current,
                            Err(cause) => return Ok(UploadOutcome::Unattached { object, cause }),
                        };
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
                let bytes =
                    read_verified(store.as_ref(), &object.key, &blob, inner.limits.blob_bytes)
                        .await?;
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
                let changed = blob.state != BlobState::Detached;
                if changed {
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
                if let Err(cause) = inner.runtime.read::<Blob>(&actor, &id).await {
                    // The committed mutation has no authorized acknowledgement.
                    return Err(if changed {
                        Error::Unknown
                    } else {
                        Error::Core(cause)
                    });
                }
                Ok(object)
            })
        })
        .await
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

/// Verify retrieved immutable bytes before either accepting duplicate publication or returning a read.
async fn read_verified(
    store: &dyn BlobStore,
    key: &ObjectKey,
    blob: &Blob,
    max_bytes: usize,
) -> Result<Vec<u8>> {
    let (bytes, digest) = hash(store.get(key, max_bytes).await?).await?;
    if digest != blob.digest || bytes.len() as u64 != blob.bytes {
        return Err(Error::Conflict);
    }
    Ok(bytes)
}
