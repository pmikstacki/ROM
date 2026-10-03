# rom-blob

Register `definition()` as an ordinary Resource. Configure named byte stores
with `BlobService::builder(runtime).store("attachments", adapter).build()`.
Every reservation, attachment and detachment uses the existing Resource policy,
revision, receipt, journal and persistence pipeline. Another application Resource can refer to `ResourceRef<Blob>` without a repository or controller.

```no_run
use rom::{Actor, Runtime};
use rom_blob::{BlobService, BlobStore, Digest, UploadOutcome};
use std::sync::Arc;
async fn attach(runtime: Runtime, store: Arc<dyn BlobStore>, actor: Actor)
    -> rom_blob::Result<()> {
    // The host registered rom_blob::definition() before building runtime.
    let blobs = BlobService::builder(runtime.clone()).store("attachments", store).build()?;
    blobs.reserve(&actor, "photo", "attachments", Digest::of(b"image bytes"),
        11, "reserve-photo").await?;
    let upload = Box::pin(futures_util::stream::iter([Ok(b"image bytes".to_vec())]));
    match blobs.upload(&actor, "photo", upload).await? {
        UploadOutcome::Attached(_) => assert_eq!(blobs.read(&actor, "photo").await?, b"image bytes"),
        UploadOutcome::Unattached { .. } => { /* reconcile receipt before cleanup */ }
    }
    blobs.shutdown().await?;
    runtime.shutdown().await?;
    Ok(())
}
```

The supported state machine is Pending → Ready → Detached. Callers can reserve
or edit their own Pending records. Generic Create/Replace cannot manufacture Ready
or change a Ready reference: field policies restrict those writes to the explicit
`worker_actor()` service identity. When you install `IdentityGate`, allow-list that
trusted local service identity. Do not grant it to HTTP request identity fields.
Using a different Blob definition transfers these invariants to the host.

Uploads must receive the actual complete bytes. Before publication, they verify
SHA-256 and length. Knowing another object's digest is insufficient to attach it. Keys
bind owner, Resource id, reservation revision, store and manifest. Thus, separate
reservations do not share physical deletion ownership. Existing keys are immutable;
a retry reads and verifies existing content rather than overwriting it. Restart
retry consumes complete input again. A persisted Ready record supports reads after
reopen without uploading again. ETags are never treated as content digests.

Read authorization happens before provider access and again after bounded retrieval
and digest verification; revision changes and revocation suppress the bytes.
Upload admission and pre-publication checks use the current caller. Conditional
finalization runs as the explicit host worker to complete accepted work. It can
complete after caller disconnection or a last-moment revocation. Current caller
checks still prevent disclosure. Concurrent reservation edits lose
the expected-revision race and produce `Unattached` with an opaque object receipt.
A core `Unknown` in that receipt can mean metadata committed. Before you delete
anything, inspect/retry. Provider `Unknown` can mean a complete object exists while
its reservation remains Pending. There is no distributed database/object transaction.

The default limits are four operations, one MiB per object, 64 KiB per input chunk,
1024 chunks including empty ones, and ten seconds to stage input. Host configuration
is capped at 64 operations, 16 MiB per object and 65536 chunks. These bound accepted
bytes and framework work, not producer/SDK buffers or exact process heap size.
SHA-256 runs on Tokio blocking workers. Caller cancellation retains the operation
permit and accepted task. Shutdown closes intake and drains those tasks. Keep the
Tokio runtime alive. Shut down BlobService before its core Runtime. A backend
panic closes the service and is reported. No hard cancellation of filesystem I/O
is claimed; staging timeout occurs before provider publication.

Detachment is logical and does not delete bytes. Its stable `ObjectReceipt` is for
trusted maintenance. Physical cleanup is deliberately explicit. Before the host invokes
the storage port's idempotent delete, it establishes a grace period, upload
quiescence and absence of current references. A failed finalization or interrupted process
can leave an orphan. There is no automatic garbage collector, durable cleanup
scheduler, multipart/resumable upload, ranges or public blob HTTP route in this
package. The raw `BlobStore` is a trusted adapter port, not an authenticated API.

Run `cargo test -p rom-blob --locked`; the adapter crate supplies real folder/MinIO
conformance and folder-plus-SQLite restart evidence.
