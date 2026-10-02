# Capability adapter experiments

Date: 2026-10-02. Owner-confirmed direction: ROM core depends on generic semantic contracts; concrete databases, file stores and notification providers implement those contracts. Resource remains the sole domain entity. Adapter configuration and work envelopes are infrastructure values, not competing domain models.

## Questions and bounded probes

1. **Persistence:** can the same core commit a resource transition, revision, receipt and events through a relational adapter and a transactional key-value adapter? Run identical conformance cases against scratch SQLite and a Rust embedded transactional KV engine. The core dependency graph must contain neither driver. Exercise conflicts, rollback, duplicate command identity, mismatched reuse, restart and unknown commit outcomes. Do not interpret an absent receipt after timeout as proof of rollback. Database-native transactions and query syntax remain inside adapters.
2. **Blob storage:** can the same host-facing contract store/read/delete content using a directory and an S3-compatible implementation? Use scratch data, bind any test server to loopback, and test actual S3 protocol behavior when a local compatible server is available. Distinguish real interoperability from a memory fake or compile-only adapter. Unsupported preconditions must fail explicitly, never become unconditional writes. File paths, credentials and provider SDK types stay outside core. A path prefix alone is not a sandbox; document trust and symlink rules.
3. **Notification channels:** can applications register a named async Rust function, have committed work delivered through it, and survive retry/unknown outcomes without claiming exactly-once external effects? Use a local recording or loopback receiver, never send real email. Demonstrate stable delivery identity, backoff/attempt bounds, default behavior on permanent failure, and the ambiguous window after a provider accepts but before acknowledgment is recorded. Record limits if the durable-work implementation is separate from the persistence prototype.

These are executable contract probes, not production integration selection. Prefer focused reuse of existing Rust libraries, keep driver types out of contracts, and record exact crate/toolchain versions and observed failures. Internal machinery may be complex; the resource author's path should remain small and readable.

## Contract design constraints

- Separate semantic interfaces for persistence, blobs and notifications. A universal string-command plugin interface would erase useful typing and guarantees.
- Mandatory guarantees and optional capabilities are explicit and validated during host setup. No adapter advertises behavior it cannot provide in its configured mode.
- Atomic database commit does not make an S3 upload or email send transactional. Persist work intention with the resource transition; execute external effects after commit and recover through stable work identity.
- Live queries and permissions belong to ROM. Database change feeds may help invalidate reads, but a raw vendor feed is not the resource action/event contract.
- The source review samples database families and concrete representatives. It is not an exhaustive test of every database product.

Each experiment belongs on an isolated prototype branch with a runnable verifier, limitations and implementer assessment. Main keeps the specification, evidence and resulting decisions.
