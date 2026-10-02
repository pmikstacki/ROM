# Provider-independent blob probe

This throwaway workspace asks whether one small blob contract can serve a folder and an actual S3-compatible server without exposing either provider to core. It is not ROM production code. Resource remains the domain entity; storage handles and receipts are infrastructure values.

- `core`: `BlobStore`, upload stream, limits, capabilities and normalized errors. Its only dependency is `futures-core`.
- `adapters`: private `object_store = 0.14.2` implementation, configured as a trusted folder or loopback S3 endpoint.
- `adapters/tests/conformance.rs`: shared contract checks, filesystem trust characterization and actual lost-response fault injection through a loopback TCP relay.

Applications consume `&dyn BlobStore`; adapter constructors belong to host setup. A caller submits a stream of owned byte chunks with `put(key, stream, Condition::Any)`, then uses the same `get` and `delete` methods for either backend. This probe has no named registry or fluent application builder yet.

## Run

Requires Rust 1.99, Bash, ripgrep and curl. The lockfile pins the complete dependency graph. From this directory, `./verify` checks formatting, Clippy, all default tests, documentation and the core dependency boundary. Two real S3 tests are visibly ignored by default; they are not represented as executed interoperability evidence.

To run all eight tests against a disposable MinIO server:

```sh
MINIO_BIN=/path/to/minio ./verify-s3
```

The script creates scratch storage and a local test bucket, binds API and console to loopback, uses dummy local credentials, runs all tests and stops its server on exit. Default ports are 19080/19081; override `ROM_BLOB_API_PORT`/`ROM_BLOB_CONSOLE_PORT` if occupied. Never point the fixture at existing data. S3 constructors reject non-loopback addresses. No external bucket or credentials are needed.

On this development host, obtain MinIO with:

```sh
nix --extra-experimental-features 'nix-command flakes' build nixpkgs#minio --no-link --print-out-paths
```

The verified binary was `/nix/store/7yy1v6cln7calpqkqax9bilszlr25wra-minio-2024-09-22T00-33-43Z/bin/minio`. Run verification through `rom-dev`:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/blob-adapters/prototypes/blob-adapters && ./verify'
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/blob-adapters/prototypes/blob-adapters && MINIO_BIN=/nix/store/7yy1v6cln7calpqkqax9bilszlr25wra-minio-2024-09-22T00-33-43Z/bin/minio ./verify-s3'
```

## Exact guarantees and limits

Uploads stage into one fixed-capacity buffer before starting a provider write. Limits bound total accepted bytes, individual input chunks and chunk count (including empty chunks). Configuration itself is capped at 16 MiB per blob and 65,536 chunks. Tests deliberately use tiny limits: 16 bytes, 8-byte chunks and 16 chunks. Reads check advertised size and count actual streamed bytes before returning a bounded buffer.

This bounds this adapter's staging/return buffers per operation, not memory already allocated by producers, transport/SDK buffers, concurrent operations or the whole process. There is no admission control. A producer can stall; callers own the overall deadline. S3 request timeout is three seconds and automatic retries are disabled for this experiment. Retrying a failed unconditional overwrite can clobber a competing writer.

Replacement uses the library's atomic whole-object put. Partial input, size rejection and cancellation during staging never start a write. Once the provider request starts, dropping the future gives no rollback guarantee. Unknown mutating errors return `Unknown`; callers must reconcile rather than assume absence. The relay test proves a real server can commit while the caller receives no success receipt. It is not a power-loss durability test.

Conditional writes are explicitly unsupported on both configured adapters, even where an underlying provider has stronger capabilities. They fail before reading the stream or mutating storage. There is no head-then-put emulation. Delete normalizes a missing object to success. Provider errors and credentials remain outside core; error classification is intentionally coarse.

Keys use bounded lowercase ASCII path segments. Traversal, backslashes, absolute paths and empty segments are rejected. **The folder adapter is for an exclusively trusted directory, not a sandbox.** `object_store` follows symlinks, including outside its prefix; an executable characterization test demonstrates this. Key validation cannot stop another process inserting symlinks. The local adapter enables fsync; no crash/power-cut verification was performed, and this probe was run on Linux only.

Multipart/resumable upload, range reads, listing, signed URLs, metadata/revision APIs, content digests, retries, database/blob coordination and durable cleanup are deliberately absent. No test claims partial multipart recovery or compatibility with AWS/all S3-compatible products.
