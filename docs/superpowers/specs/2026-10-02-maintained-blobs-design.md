# Maintained blob attachment design

Resource remains the only domain entity. This package supplies an ordinary `Blob`
Resource, a provider-neutral byte store port and a bounded lifecycle service.
`rom` acquires no object-store or HTTP dependency. `rom-blob-object-store` keeps
the researched `object_store` 0.14.2 implementation private and is MIT licensed;
the dependency is Apache-2.0, compatible with the project license.

The selected whole-object profile caps blobs at 16 MiB, input chunks and chunk
count, concurrent accepted operations and input staging time. The normal Resource create path reserves a blob with the caller identity, store name, SHA-256
digest, and byte length. Before any provider mutation, a complete upload must match those declared bytes. Physical keys bind the reservation id/revision and content
manifest; two logical reservations do not share physical deletion ownership.
The store implements atomic create-only publication. Existing identical bytes
allow retry; conflicting content never overwrites. ETags are not content hashes.

The lifecycle is Pending → Ready → Detached. Ready is committed conditionally
only after successful complete publication. Field policies reject forged Ready
values submitted through generic Resource invocation. A fixed, explicitly
trusted service principal performs internal finalization; it must be enabled in
an installed identity gate. Completion of accepted Pending work may continue
under worker authority after caller disconnection/revocation. The current caller
is checked before provider access and before any result/content disclosure.
Resource revision arbitration prevents finalization over a changed reservation.

The service keeps accepted asynchronous work and its permits until actual
completion, including after the caller stops waiting. Shutdown closes intake and
drains those tasks before the host closes its core Runtime. Input timeout occurs
before provider publication. Folder I/O has no claimed hard cancellation deadline;
S3 uses bounded requests and disabled automatic retries. Provider uncertainty is
distinct from definite failure.

Failed attachment commit leaves a complete unattached object and an explicit
orphan receipt; unknown provider publication remains unknown until retry.
Restart recovery retries a persisted Pending reservation with the complete input,
never attaches solely by guessed digest. Before backend access, reads authorize the Resource. They bound and verify the content. Before they return bytes, they recheck current authorization and
the attachment revision. Detachment commits the ordinary
Resource state before any host cleanup. Physical garbage collection is explicitly
host maintenance: use the opaque object receipt, a grace period, quiescence and
current-reference checks before calling the store's idempotent delete. No automatic
collector, distributed SQL/S3 transaction or physical erasure claim is made.

Folder roots and descendants must be exclusively trusted. Because object_store follows
symlinks, its configured prefix is not a sandbox. Folder publication enables
fsync; tests cover process reopen, not power loss. S3 accepts explicit HTTPS host
configuration or a deliberately selected numeric loopback HTTP fixture. Only the
configured endpoint passing the actual shared suite is interoperability evidence.
No multipart/resumable upload, range, listing, presigned URL or arbitrary overwrite
capability is advertised.

Evidence: `docs/research/storage-and-notification-adapters.md` and the disposable
`prototype/blob-adapters` assessment. Exact crate source is inspected alongside
the lockfile; official references are the [object_store contract](https://docs.rs/object_store/0.14.2/object_store/trait.ObjectStore.html),
[put modes](https://docs.rs/object_store/0.14.2/object_store/enum.PutMode.html) and
[local filesystem profile](https://docs.rs/object_store/0.14.2/object_store/local/struct.LocalFileSystem.html).
