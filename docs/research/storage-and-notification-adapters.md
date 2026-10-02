# Blob storage and notification adapters

Date: 2026-10-02. Research and proposed contracts; no implementation or provider selection is implied. Resource remains ROM's sole domain entity. Blob references, channel handles, delivery IDs and outbox records are infrastructure values, not additional domain entities.

## Recommendation

Use separate `BlobStore` and `NotificationChannel` semantic ports. Ship useful adapters and accept ordinary custom Rust implementations; do not make applications implement an entire universal integration framework. Keep provider configuration, SDK types, credentials and wire errors inside adapters. Register named instances through the fluent application builder. Tokio handles asynchronous I/O; expensive encoding/compression can use the existing bounded CPU execution service.

Prototype `object_store` as the first folder/S3/S3-compatible implementation behind ROM's port. OpenDAL is a credible alternative when broader storage coverage becomes necessary. Neither library should define ROM's public API, database contract, or notification lifecycle.

## Storage evidence and tradeoffs

The inspected `object_store` documentation reports 0.14.2. Its narrow object abstraction includes local files and cloud stores; OpenDAL exposes a broader operator and a detailed capability inventory. Both justify reuse rather than hand-written S3 clients. Pin the actual dependency and verify its source: moving OpenDAL documentation exposes differing descriptions of unsupported write conditions. ROM must always reject an unsupported requested guarantee. [object_store](https://docs.rs/object_store/0.14.2/object_store/), [OpenDAL capabilities](https://opendal.apache.org/docs/rust/opendal/struct.Capability.html), [OpenDAL conditional specification](https://opendal.apache.org/docs/rust/opendal/docs/specs/conditional_operations/index.html).

| Concern | Evidence and ROM consequence |
| --- | --- |
| Atomic visibility | `object_store` guarantees complete-object visibility for `put_opts`. OpenDAL's filesystem adapter exposes `atomic_write_dir`; append bypasses that mechanism. Require old-or-new visibility for ROM replacement; configure and test each implementation. This is distinct from surviving power loss. [ObjectStore](https://docs.rs/object_store/0.14.2/object_store/trait.ObjectStore.html), [Fs](https://opendal.apache.org/docs/rust/opendal/services/struct.Fs.html). |
| Conditional writes | `object_store` offers overwrite, create and update modes, but its local implementation rejects `PutMode::Update`. OpenDAL advertises individual condition capabilities. Atomic compare-and-swap is optional; never emulate it with head-then-put. [PutMode](https://docs.rs/object_store/0.14.2/object_store/enum.PutMode.html), [local source](https://docs.rs/object_store/0.14.2/src/object_store/local.rs.html), [conditional specification](https://opendal.apache.org/docs/rust/opendal/docs/specs/conditional_operations/index.html). |
| Streaming/multipart | `object_store` separates buffered puts from multipart uploads with completion/abort. OpenDAL supplies a streaming writer. ROM should expose bounded streaming with explicit successful completion; multipart part sizes and upload IDs remain adapter details unless resumability is separately supported. [MultipartUpload](https://docs.rs/object_store/0.14.2/object_store/trait.MultipartUpload.html), [Writer](https://opendal.apache.org/docs/rust/opendal/struct.Writer.html). |
| Listing/paths | `object_store` recursive listing uses segment prefixes and does not guarantee order. Filesystem restrictions differ by OS; its local backend follows symlinks outside its prefix. ROM keys need validation and controlled storage roots; a configured prefix is not a sandbox. [ObjectStore](https://docs.rs/object_store/0.14.2/object_store/trait.ObjectStore.html), [LocalFileSystem](https://docs.rs/object_store/0.14.2/object_store/local/struct.LocalFileSystem.html). |
| ETags/versions | Preserve opaque comparison tokens, scoped to adapter and key. Store content digests separately: S3 multipart ETags are not whole-object MD5 digests. [S3 integrity](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity-upload.html). |
| Signed requests | `object_store` has a separate signing abstraction; OpenDAL advertises presign operations. Make direct access optional, scoped by operation and expiry. Local storage may instead use an authenticated ROM download endpoint. [Signer source](https://docs.rs/object_store/0.14.2/src/object_store/signer.rs.html), [OpenDAL capabilities](https://opendal.apache.org/docs/rust/opendal/struct.Capability.html). |

Local `object_store` durability requires opting into `with_fsync`; directory syncing is Unix-specific. Record the supported durability profile instead of equating atomic rename with crash durability. S3-compatible means protocol compatibility, not proof of every AWS guarantee: certify the configured endpoint against the same suite. [LocalFileSystem](https://docs.rs/object_store/0.14.2/object_store/local/struct.LocalFileSystem.html), [OpenDAL S3](https://opendal.apache.org/docs/rust/opendal_service_s3/struct.S3.html).

## Proposed blob contract

Illustrative Rust surface, not compiled API:

```rust,ignore
app.blobs("attachments", Folder::new(root)); // or S3/custom adapter
let receipt = ctx.blobs("attachments").put(key, stream).await?;
```

Mandatory methods: `put(key, stream) -> BlobReceipt`, `get(key) -> BlobRead`, `head(key) -> BlobMetadata`, and idempotent `delete(key)`. All async operations are bounded and cancellation-aware. `put` replaces atomically on successful completion; timeout/cancellation can leave an unknown commit outcome. A receipt includes byte length and any opaque revision token. Metadata supports absent fields rather than inventing ETags or timestamps. Domain metadata and permissions belong in the durable metadata database.

Optional extensions: atomic create-only, conditional replacement/deletion, ranges, listing, version retrieval, signed access and resumable uploads. Validate required capabilities at application build time and reject unsupported calls at runtime. Capability combinations matter: conditional multipart completion cannot be inferred from ordinary conditional-put support. Normalize errors into missing, conflict, denied, unsupported, invalid input, transient failure and unknown outcome without leaking credentials.

Keys are validated relative logical names, with no traversal or absolute paths; specify segment-prefix listing, unspecified order and no snapshot guarantee. Namespace isolation and local symlink confinement require explicit enforcement or a trusted exclusive directory. Keep provider pagination tokens internal. An author uses the same blob methods when switching named storage configuration.

For resource-plus-blob creation, upload to an immutable unique key, then commit the resource's reference in the database. A failed database commit leaves a collectible orphan; do not claim a distributed transaction. Garbage collection needs age/grace and reference checks. For deletion, commit detachment plus cleanup intent, then delete asynchronously. Never hold a database transaction open while uploading.

## Named notification channels

Prefer a typed closure convenience wrapper over a small object-safe sending port:

```rust,ignore
app.channel("account-updates", channel_fn(
    |delivery: Delivery<AccountNotice>, context: SendContext| async move {
        custom_send(delivery.payload, context.idempotency_key).await
    },
));
// Inside a resource mutation transaction:
tx.notify("account-updates", AccountNotice::changed(resource_id))?;
```

Registration supplies a versioned payload codec and validation; internally persisted payloads are serialized values, never Rust closures. `NotificationChannel::send(Delivery, SendContext)` returns `Accepted(receipt)`, `Retryable(retry_after)`, `Permanent(reason)` or `Unknown(reason)`. A closure adapter handles type erasure so authors write normal Rust functions. The same seam can wrap SMTP, HTTP email APIs, webhooks or application code. Optional lettre integration translates email payloads to its async transport; lettre's message/envelope types never become the universal notification model. [lettre AsyncTransport](https://docs.rs/lettre/0.11.23/lettre/trait.AsyncTransport.html).

Mandatory channel behavior is validation, asynchronous sending, bounded execution and explicit outcome classification. Optional capabilities include provider idempotency, receipt lookup, delivery callbacks, batching and cancellation. An accepted send means the downstream service accepted responsibility; it does not prove inbox arrival or human attention. SMTP specifically admits duplicate delivery following timeouts. A stable Message-ID is not a portable exactly-once guarantee. [SMTP RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html#section-6.1).

Commit resource changes and notification intent in one database transaction. Workers lease intents, invoke the channel after commit and durably record outcomes. Retry with bounded exponential backoff, jitter, attempt/deadline budgets and inspectable terminal failures. This provides durable at-least-once attempts, not guaranteed eventual delivery. A crash after provider acceptance but before receipt persistence creates ambiguity; an outbox does not eliminate duplicates. [Transactional outbox](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html).

Keep one stable delivery/idempotency key across retries. Enqueue deduplication belongs to the database; external-effect deduplication requires the recipient/provider's support and retention window. For unknown outcomes, reconcile receipts when supported; otherwise apply the channel's explicit retry-or-review policy. Leases prevent normal concurrent work but cannot fence an external service by themselves. Email/S3/SQL cannot share ROM's atomic commit promise.

## Negative and fault tests before selection

- Run identical blob contract cases against a temporary folder and selected S3 endpoints: empty/large streams, replacement visibility, missing deletion, invalid keys, symlink escape, permissions and disk exhaustion.
- Race create-only and stale conditional writes; reject unsupported conditions before mutation. Test conditional multipart separately, abort/cancellation, lost completion responses, process crashes, orphan cleanup and stream backpressure.
- Check misleading ETags, range bounds, unordered/multipage listings and signed-request expiry. Verify local durability on supported filesystems separately from process-restart tests.
- Use fake channels and local SMTP/HTTP fixtures: transaction rollback sends nothing; crash at every outbox/send/receipt boundary; duplicate claims, stale leases, poison payloads, channel version changes, rate limits, permanent failures and acceptance followed by connection loss.
- Prove repeated attempts preserve the key, providers without dedup can duplicate effects, unknown outcomes remain inspectable, and logs redact payload secrets. No genuine email or cloud provisioning is required for the initial harness.
