# Relational storage capabilities for a database-independent ROM

Date: 2026-10-02. Primary-source research; no new experiments or conformance claims. This covers representative embedded, replicated-edge, and client/server relational deployments, not every database product. Existing SQLite prototypes establish only their exercised behavior; they do not select ROM's production database.

## Put operations, not SQL, at the boundary

**Recommendation/inference:** ROM should specify an operation-level persistence protocol. `Resource` remains its sole entity concept; receipts and journal entries are internal protocol records. Actions produce a candidate transition. A durable adapter conditionally commits the Resource state, revision, action receipt, and resulting events in one atomic domain. The protocol must describe observable guarantees without prescribing tables, SQL statements, driver transactions, or database-wide writer serialization.

| Core owns | Adapter owns |
| --- | --- |
| Resource identity, field semantics, expected revisions, action identity/fingerprint, transition/event meaning | Physical layout, codecs, constraints, transaction scope, conditional-write implementation |
| Query predicate/order semantics, authorization, live-query reconciliation | Query compilation, indexes, snapshot implementation, freshness routing |
| Conflict/retry policy, receipt retention contract, consumer deduplication | Native error classification, uncertain-commit recovery, durable journal/cursor implementation |
| Requested durability and consistency | Validating the deployment can provide them; rejecting unsupported requests |

A generic `begin/execute/commit` SQL driver interface belongs inside relational adapters. It neither describes conditional atomicity nor prevents a core from assuming SQLite locking. It also excludes non-SQL implementations unnecessarily. Prefer semantic operations such as conditional commit, receipt lookup, bounded query/read, and journal scan; names are illustrative, not a proposed Rust API.

## Minimum durable contract and optional capabilities

The following are proposed minimum guarantees, derived from the failure modes below:

1. **Conditional atomic commit:** within a declared atomic scope, validate expected revisions and uniqueness, then make all transition records visible together or none. Include create-if-absent and deletion/tombstone conditions. Explicitly declare any limits on Resource count, bytes, or partitions.
2. **Receipt arbitration:** a scoped action key plus canonical request fingerprint identifies one operation. Concurrent duplicate submissions cannot both apply. Retrying the same request returns the stored outcome; reusing the key for different input is rejected. Receipt retention bounds this guarantee.
3. **Honest completion:** distinguish committed, condition conflict, definitely aborted/retryable, unsupported, and outcome unknown. Cancellation or connection loss is not proof of rollback. Recover an uncertain outcome through authoritative receipt lookup or an atomic retry of the same identity. An absent receipt observed while the original transaction may still finish is not proof of failure.
4. **Coherent observation:** reads identify their consistency/freshness contract; supported query results contain no partially committed transition. Provide an authoritative path for revision checks and receipt recovery. A read-after-write request must observe at least the acknowledged operation or report inability to establish that boundary.
5. **Recoverable events:** committed journal records survive under the same durability profile as state. Bounded scans use opaque continuation tokens, explicit retention/expiry, and stable event identities. A token must never silently skip a later-visible commit. Redelivery is allowed; exactly-once external effects are not implied.
6. **Declared recovery boundary:** durable acknowledgment states whether it survives process restart, OS/power failure, and selected failovers. These are different profiles, not one boolean.

Optional capabilities include multi-Resource transactions, serializable predicate validation, historical/time-travel queries, portable snapshot handles, replica freshness barriers, native CDC, full-text/vector queries, and index-backed execution guarantees. For live queries, an adapter additionally needs a coherent initial result/journal handoff, or a documented conservative reconciliation algorithm with explicit resync on gaps. An adapter may meet durable mutation requirements without offering every query capability.

## Backend mappings and restrictions

### SQLite through rusqlite

SQLite serializes writers; WAL permits readers to retain an older snapshot while writing proceeds. A stale read transaction cannot upgrade to a writer and may fail with `SQLITE_BUSY_SNAPSHOT`. A bounded `BEGIN IMMEDIATE` transaction is one possible adapter implementation for authoritative read/validate/write; it is not a core scheduling rule. A single conditional update on identity and revision can arbitrate the transition, with receipt and journal inserts in that transaction. [SQLite isolation](https://www.sqlite.org/isolation.html)

Keep the atomic envelope in one database: WAL transactions across attached databases are not collectively atomic. WAL requires same-host shared memory and does not support ordinary multi-host network-filesystem access. Long read transactions can prevent checkpoint progress. [WAL restrictions](https://www.sqlite.org/wal.html)

Busy-at-commit can leave the transaction active, whereas other errors can roll back a statement or the transaction. The adapter must inspect/normalize transaction state before retrying. A stale-snapshot retry must restart the transaction, not merely rerun the failed statement. [Transaction handling](https://www.sqlite.org/lang_transaction.html)

For power-loss durability, SQLite documents WAL plus `synchronous=FULL`; rollback-journal mode needs `EXTRA` for the stronger guarantee across relevant filesystems. WAL `NORMAL` can lose acknowledged recent transactions after power loss. Verify actual settings. [Synchronous modes](https://www.sqlite.org/pragma.html#pragma_synchronous)

rusqlite exposes synchronous methods and a `Connection` that is `Send` but not `Sync`. With the selected Tokio/Rayon runtime, confine connections to bounded blocking ownership; Rayon remains a CPU execution choice, not a database coordination mechanism. [rusqlite Connection](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html)

### libSQL versus the newer Turso engine

These require separate profiles. libSQL is a SQLite fork; Turso Database is a Rust rewrite with native asynchronous support. The current Turso repository reports production use but remains pre-1.0 and explicitly does not claim complete SQLite compatibility. SQL/file compatibility does not transfer every concurrency, durability, or extension guarantee. [Turso project status and distinction](https://github.com/tursodatabase/turso/blob/main/README.md)

The libSQL SDK documents `write` as `BEGIN IMMEDIATE`, forwarding replica write transactions to the primary; `read` can run on replicas; `deferred` upgrades can fail. Its documented interactive transactions hold the write lock and have a five-second timeout. Treat this as that SDK/service contract, not an intrinsic timeout for every local libSQL deployment. [libSQL transaction modes](https://docs.turso.tech/sdk/ts/reference#transaction-modes)

Embedded replicas normally forward writes, including their transactional reads. With read-your-writes enabled, the initiating replica observes its successful write; other replicas advance on synchronization. Offline mode and disabling read-your-writes change the contract. A local offline acknowledgment cannot automatically mean authoritative global success. [Embedded replicas](https://docs.turso.tech/features/embedded-replicas/introduction)

As documented now, Turso Cloud concurrent writes are early preview: dashboard opt-in, a `tursodb` database, and `BEGIN CONCURRENT` are required. Conflicts concern rows and may surface during a statement or commit; concurrent transactions exclude DDL such as index creation. This is neither ordinary SQLite nor libSQL. The cited material establishes write-conflict behavior, not arbitrary serializable predicate validation; require separate evidence for that capability and for the deployed durability profile. [Concurrent-write requirements](https://turso.tech/blog/concurrent-writes-in-practice)

### PostgreSQL

At default Read Committed, each statement receives a new snapshot. A conditional `UPDATE ... WHERE revision = expected` rechecks its predicate after waiting for a concurrent updater, making it useful for Resource CAS. Multi-statement reads need an appropriate transaction snapshot; multi-Resource invariants require validated dependencies, explicit locking, or Serializable with retries. Repeatable Read alone does not remove every serialization anomaly. [Isolation levels](https://www.postgresql.org/docs/current/transaction-iso.html)

Serialization failures require retrying the entire transaction, including the decisions that produced its statements. Do not rerun external side effects inside that retry. [Retry requirements](https://www.postgresql.org/docs/current/mvcc-serialization-failure-handling.html) The error catalog distinguishes transaction-resolution-unknown from ordinary rollback errors; a lost commit response also warrants an unknown outcome even without that specific SQLSTATE. [Error classifications](https://www.postgresql.org/docs/current/errcodes-appendix.html)

Durability depends on `fsync`, WAL configuration, and `synchronous_commit`. Turning synchronous commit off permits acknowledged transaction loss. Synchronous standby flush acknowledgment does not itself establish query visibility there; `remote_apply` waits for replay on configured synchronous standbys. Arbitrary replicas still need routing/barriers. [WAL and acknowledgment settings](https://www.postgresql.org/docs/current/runtime-config-wal.html)

### MySQL/InnoDB

Require transactional InnoDB tables for the entire envelope and one transaction/connection. Keep schema work outside action commits: numerous DDL statements implicitly commit. [Implicit commits](https://dev.mysql.com/doc/refman/8.4/en/implicit-commit.html)

Default Repeatable Read gives ordinary reads a transaction snapshot, while locking reads and updates use different visibility rules. Do not combine an old snapshot read with a current write and assume dependencies were validated. Use revision predicates and, when needed, deliberate locking reads under an explicit transaction. [Isolation behavior](https://dev.mysql.com/doc/refman/8.4/en/innodb-transaction-isolation-levels.html), [locking reads](https://dev.mysql.com/doc/refman/8.4/en/innodb-locking-reads.html)

Deadlock aborts the transaction; a lock timeout normally rolls back only the statement. Normalize this distinction, typically rolling back the operation before retry. [InnoDB errors](https://dev.mysql.com/doc/refman/8.4/en/innodb-error-handling.html) Durable deployments require `innodb_flush_log_at_trx_commit=1` and, with binary logging, `sync_binlog=1`; hardware must honor flushes. Those settings do not guarantee immediate visibility or preservation on every replica/failover topology. [Durability settings](https://dev.mysql.com/doc/refman/8.4/en/innodb-parameters.html#sysvar_innodb_flush_log_at_trx_commit)

## Journal, external writers, and live queries

**Protocol inference:** a sequence-generated journal key is not automatically a safe resume cursor. Transaction A can reserve 10, B reserve and commit 11, a reader advance past 11, then A commit 10. PostgreSQL sequences are not transactional; InnoDB auto-increment allocation likewise is not a commit-order protocol. Use a transactionally serialized publication boundary, a proven native change-stream token, or partitioned progress tracking. Keep that mechanism adapter-private; global ordering has a concurrency cost. [PostgreSQL sequences](https://www.postgresql.org/docs/current/transaction-iso.html), [InnoDB allocation](https://dev.mysql.com/doc/refman/8.4/en/innodb-auto-increment-handling.html)

Notifications are wakeups, not ROM history. SQLite update hooks are connection-local and omit some changes; `data_version` detects other-connection commits only by comparing successive values on the same connection. PostgreSQL `NOTIFY` targets listeners; logical decoding provides persistent slots but may redeliver after crash. [SQLite hooks](https://www.sqlite.org/c3ref/update_hook.html), [data_version](https://www.sqlite.org/pragma.html#pragma_data_version), [NOTIFY](https://www.postgresql.org/docs/current/sql-notify.html), [logical decoding](https://www.postgresql.org/docs/current/logicaldecoding-explanation.html)

Separate cooperating external ROM writers from arbitrary SQL writers. The former obey the same receipt/revision/journal protocol. The latter can invalidate it: CDC can observe row changes but cannot reconstruct missing action intent or authorization. Either restrict bypass writes or explicitly support trigger/CDC integration and resync semantics. Live-query initialization must couple its result snapshot to journal progress; independently reading the result and then the latest cursor can miss changes.

## Query semantics and promises to reject

Core should define typed values, missing versus null, numeric ranges, timestamp precision, collation, comparison, and a total ordering with Resource identity as tie-breaker. SQLite affinity/coercion and heterogeneous storage-class sorting differ from rigidly typed expectations. PostgreSQL defaults nulls last ascending; MySQL defaults nulls first. Adapters must compile explicit equivalent semantics or reject the query. [SQLite types](https://www.sqlite.org/datatype3.html), [PostgreSQL order](https://www.postgresql.org/docs/current/queries-order.html), [MySQL nulls](https://dev.mysql.com/doc/refman/8.4/en/problems-with-null.html)

Use opaque, versioned query cursors bound to predicate/order and consistency context. Keyset pagination alone does not preserve a historical result across concurrent updates. Declare snapshot lifetime/expiry; negotiate index requirements and bounded scan fallback separately from semantic support.

Reject portable promises of unlimited transactions, universal historical snapshots, gapless IDs, arbitrary index-efficient queries, cross-shard atomicity without coordination, identical native SQL behavior, instant global replica visibility, lossless unbounded subscriptions, and exactly-once delivery to unrelated external systems. The useful portable guarantee is a precisely scoped operation with explicit capabilities and failure outcomes.
