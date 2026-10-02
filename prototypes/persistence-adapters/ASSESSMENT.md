# Persistence adapter assessment

Date: 2026-10-02. Branch: `prototype/persistence-adapters`. Base: `3bdee66` (`main` when this worktree was created). No production engine chosen.

## Observed evidence

`./verify` completed successfully in `rom-dev` with:

| Component | Observed version |
|---|---|
| rustc | 1.99.0 (`b940084d7`, 2026-09-28) |
| cargo | 1.99.0 (`5f94df478`, 2026-08-27) |
| rusqlite / bundled SQLite | 0.40.2 / 3.53.2 |
| redb | 4.3.0 |
| serde / serde_json | 1.0.229 / 1.0.151 |
| getrandom / tempfile | 0.4.3 / 3.27.0 |

The verifier checks formatting, Clippy for every target and feature with warnings denied, the complete workspace test suite, a build without fault-injection features, the application example, and a positive allowlist of core dependencies. `Cargo.lock` fixes transitive versions. Core's graph contains Serde and macro dependencies only; neither database driver appears.

Eighteen tests passed: ten shared conformance cases, six shared failure cases, and two process-recovery cases. Each process-recovery case exercises four subprocess exits, for eight distinct crash checkpoints across engines. No ignored tests or external services are involved.

| Evidence | What it establishes |
|---|---|
| Atomic bundle survives reopen | State/revision, receipt, both ordered events, and a mid-page cursor survive close/open |
| Identity/conflict/tombstone case | Identical retries resolve the old receipt; changed events fail; losing revision leaves no receipt; tombstone recreation keeps revisions monotonic |
| Concurrent revision race | Two threads propose revision 2; one wins, the other receives a conflict, and only one extra event set exists |
| Capability/limit rejection | Unsupported snapshot/multi-resource requirements and oversized transitions fail; rejection creates no state or receipt |
| Scoped cursor/tail | Foreign and malformed cursors fail; empty tail continuation and one-event pagination do not skip later commits |
| Rollback at write boundaries | Failure after state, receipt, first event, or before commit leaves none of the bundle after reopen; retry still appends a complete bundle |
| Lost acknowledgment | A real successful commit followed by a simulated missing response is `Unknown`; reopen and retry recover revision 1 without another event set |
| Absent receipt while in flight | A native write transaction pauses before commit; another read handle sees no state, receipt, or journal entries; the original and a racing duplicate then resolve to one revision/event set |
| Process exit recovery | `process::exit(73)` bypasses Rust destructors after resource, first event, precommit, and postcommit. The first three recover no bundle; postcommit recovers the complete bundle |

The in-flight SQLite case uses separate database connections, so correctness is not inferred solely from a shared application mutex. redb readers use a shared database handle with separate read transactions. Native commit-error classification is conservative in code; hooks exercise an unknown response after a real commit, not every native storage error code.

## Failures encountered and resolved

The initial empty implementations failed all ten conformance tests for missing persistence and permissive capability admission. Those became green after implementation. Six subsequently added failure tests failed while checkpoint hooks were inert, then passed when hooks were wired into real transaction boundaries. Initial compilation also found that rusqlite 0.40.2 does not accept `u64`/`usize` as SQL integers; checked conversion now stays wholly inside the SQLite adapter. No failure remains in the final verifier run.

## Interpretation

The core abstraction can express this bounded transition without SQL or key-value engine types. Internal adapter work absorbs transaction setup, conditional revision checks, exact receipt matching, ordering, persistent identity, and error translation. The same application function changes only its host-selected adapter. A universal string command or exposed vendor transaction is unnecessary for this probe.

The local journal counter deliberately takes advantage of the single-writer commit order of these two embedded engines. It is not evidence that timestamp cursors, preallocated sequences, sharded journals, or remote replicas are safe. A distributed adapter must provide its own proven continuation algorithm under the same semantic contract.

The simple synchronous trait makes this an integration probe. It does not establish the final asynchronous API, admission limits, retry policy, cancellation ownership, or efficient large-scale scheduling. The value codec is opaque bytes; Serde derives support adapter serialization but do not define final ROM field/query semantics. Action authorization, schema registration, typed derive helpers, live queries, external effects, leases, migration, and encryption are absent. Raw storage access is an infrastructure interface and must not become an application authorization bypass.

Receipts retain complete transition payloads forever here. Production needs explicit retention, privacy, storage-budget, and schema-evolution rules. Journal cursors are not cryptographically authenticated, and copying a database copies its namespace. Backups/restores/forks need a generation policy. The test profile does not claim survival of total disk loss, power-cut durability on arbitrary filesystems, replication, or multi-machine availability.

## Primary references used

- [redb 4.3.0](https://docs.rs/redb/4.3.0/redb/) describes its transactional embedded model.
- [redb write transactions](https://docs.rs/redb/latest/redb/struct.WriteTransaction.html) documents durability configuration and that a failed commit can already have durable effects and require reopen.
- [rusqlite transactions](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Transaction.html) and [SQLite transaction behavior](https://www.sqlite.org/lang_transaction.html) ground the adapter's transaction boundary.
- [SQLite synchronous pragma](https://www.sqlite.org/pragma.html#pragma_synchronous) documents WAL/FULL persistence behavior. Process recovery tests are narrower evidence than a power-loss simulation.
- [getrandom fill](https://docs.rs/getrandom/0.4.3/getrandom/fn.fill.html) supplies the persisted random namespace used for cursor scoping.
