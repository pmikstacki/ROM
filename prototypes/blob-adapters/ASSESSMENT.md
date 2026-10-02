# Implementer assessment

Date: 2026-10-02. Branch `prototype/blob-adapters`; isolated from main, no commits. Toolchain: rustc 1.99.0 (b940084d7 2026-09-28), cargo 1.99.0; `object_store` 0.14.2; MinIO RELEASE.2024-09-22T00-33-43Z from Nix.

## Finding

The same provider-free contract can perform bounded whole-object put/get/delete against a directory and a real loopback S3-compatible server. The adapter implementation shares normalization, validation and buffering while delegating actual storage and signing to an established crate. `cargo tree -p blob-contract` contains only the contract and `futures-core`; provider SDK types are private to the adapter crate.

The useful seam is smaller than `object_store::ObjectStore`. It expresses the guarantees ROM needs instead of requiring clients to understand library paths, AWS options or response types. This is enough evidence to retain the adapter-family direction, not enough to select every production storage feature.

## Executed evidence

The three initial contract tests failed against the placeholder adapter (missing normalization, writes and validation), then passed after implementation. An additional empty-chunk flood case reproduced an actual missing work bound: seventeen empty chunks replaced existing content with an empty blob. Chunk-count admission fixed that behavior while preserving existing content on rejection.

The first MinIO run found a real configuration ordering mistake: replacing `ClientOptions` erased the earlier HTTP allowance, causing reqwest `BadScheme`. Configuring timeout and HTTP allowance together fixed the reproduced failure. A hanging fault fixture exposed missing test time bounds; waiting for its relay now has a deadline.

Final verification runs formatting, Clippy with warnings denied, docs with warnings denied, default tests and dependency checks. `verify-s3` executes all eight tests, including the identical folder/S3 contract cases and a real protocol fault. The TCP relay forwards a signed PUT, observes MinIO's HTTP 200, then closes the client connection without forwarding the acknowledgment. The adapter returns `Unknown`; independent GET returns `accepted`. No memory fake substitutes for this test.

Tests also cover missing deletion/get, empty and replacement writes, exact-limit acceptance, chunk/total/chunk-count rejection, partial input failure, cancellation of a stalled partial stream, unsupported conditions, invalid keys, oversized externally written files, explicit symlink escape characterization and refusal of non-loopback endpoints. Scratch data and the test server are discarded after the verifier exits.

## What remains unproved

There is no simulated kernel crash, power failure, full disk, permission revocation, concurrent overwrite history, competing process inserting symlinks, TLS, real cloud deployment or MinIO restart test. fsync is configured but crash durability is not measured. Stream bounds concern adapter buffers and do not bound SDK internals or concurrent aggregate memory. Cancellation while the remote PUT is in flight remains ambiguous; the partial-stream cancellation test covers only the pre-write staging phase.

The small core currently returns whole bounded blobs and lacks head/metadata, capability-specific extension traits, registry ergonomics and typed key constructors. Unconditional overwrite is unsuitable for claiming immutable content or atomic cross-system resource creation. Conditional writes remain unsupported rather than being weakly emulated. Library errors collapse conservatively to `Unknown` after a mutation starts; production needs richer sanitized diagnostics and distinguishable known failures.

For a next production-facing slice, add a public downstream author example, bounded concurrency, operation deadlines, verified conditional capabilities and a deliberate filesystem security model. Large uploads need a separate multipart lifecycle contract and fault suite. Resource metadata commit, blob orphan collection and external-effect delivery belong to their respective semantic contracts, not a fictional SQL/S3 transaction.
