# Persistence adapter probe

Throwaway executable evidence for the owner-confirmed semantic persistence boundary. This is not a production database selection or a complete ROM runtime.

Run from this directory with Rust 1.99:

```sh
./verify
```

In the repository's development container, the worktree-root verifier is also available:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/persistence-adapters && ./verify'
```

`core` defines resources, transitions, receipts, events, semantic outcomes, capability requirements, and a generic runtime. It has only Serde and its derive machinery in its dependency graph. `adapters` implements SQLite and redb behind that contract; SQL, transactions, table definitions, persistence settings, and cursor interpretation remain there. Both implementations share the same integration cases and application function. Their modules share one crate for this experiment; production packaging can split that crate without changing the core contract.

The host selects a store once. Application persistence code uses `Runtime::new(store, requirements)`, `execute(&transition)`, `load(&key)`, `receipt(action)`, and `journal(cursor, limit)` identically. See [the application example](adapters/examples/app.rs). `Transition` is an infrastructure proposal emitted by the future action pipeline, not a replacement for typed resource declarations or an authorization boundary.

The committed unit is resource value/tombstone, new revision, action receipt, and all events. The action identity is unique within the database. Its receipt retains the entire proposed transition and compares it exactly, avoiding a hash collision assumption; changed input under the same identity fails. Old receipts resolve before checking the current resource revision. Event identity is the tuple `(action, ordinal)` and event order within a resource follows revision then ordinal.

SQLite uses `BEGIN IMMEDIATE`, WAL, and checked `synchronous=FULL`; redb uses its single-writer transaction and explicit `Durability::Immediate`. A counter updated inside that transaction allocates journal positions. A later writer cannot commit a higher position while an earlier writer remains uncommitted. This local serialization is intentionally simple and is not a distributed/global ordering contract. Journal cursors carry format version, backend tag, a persisted random database namespace, and position as opaque bytes. Empty tail reads do not advance past unpublished entries. Cursors are scoped continuation values, not authentication tokens.

Both stores reject snapshot-query and multi-resource-commit requirements. Neither implements queries or live reads. The advertised profile allows at most 65,536 aggregate input bytes, 32 events per transition, and pages of 1–128 events. Limits are measured over identity/value/event input bytes, not final storage overhead. Receipts and journal entries are retained indefinitely in this bounded experiment; pruning, retention gaps, migration, and restored/forked database generation fencing need further design.

`Unknown { action }` is distinct from `NotCommitted`. A commit error is conservatively unknown. `AbsentNow` explicitly does not imply rollback: another request or a transaction already in flight may later commit. Retry the same action/proposal or recover the stored receipt; do not invent a new identity. A redb commit I/O error may require closing and reopening the database before retrying.

Tests use real local scratch databases. Feature-gated `fault-injection` hooks only exist in test builds and can stop between writes, hold a native transaction open, or lose its acknowledgment. A separate subprocess exits without Rust destructors at selected checkpoints. These prove adapter behavior for those windows; they do not simulate power loss, disk controller failures, corruption, remote partitions, or fleet failover. See [the assessment](ASSESSMENT.md) for exact results and limitations.
