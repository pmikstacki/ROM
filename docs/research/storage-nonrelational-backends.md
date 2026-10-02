# Non-relational backends for a database-independent ROM persistence contract

Research date: 2026-10-02. This note reviews the [persistence specification](../../openspec/changes/establish-rom/specs/persistence/spec.md) and [architecture](../../openspec/changes/establish-rom/design.md). Findings are documentation research, not tested adapter conformance or a production database selection. Vendor documentation is linked at each factual claim; proposed ROM behavior below is architectural inference.

## Contract boundary

The existing requirement—atomically persist resource state, revision, receipt, and events—is portable across these families when the transition is bounded. It should become a semantic command such as `commit_transition(expected_revision, action_identity, request_digest, replacement_or_tombstone, events)`. The core must not receive a transaction object, session, BSON document, DynamoDB expression, keyspace handle, or engine cursor. Resource remains the sole domain entity; receipts and journal entries are persistence records.

The core owns authorization, validation, canonical field meaning, action identity, event identity, revision rules, retention policy, and acceptable observable outcomes. The adapter owns physical encoding, indexes, conflict detection, transactions, durability settings, journal discovery, and translation into core-defined errors. A revision conflict requires the core to reconsider a state-dependent transition; an adapter retry must not silently recompute domain behavior against a different revision.

Mandatory durable behavior should include a coherent resource/revision read, conditional atomic commit, durable receipt resolution, bounded resumable journal scanning, and explicit retention gaps. Query-wide snapshots, historical reads, native notifications, and multi-resource atomicity are separately negotiated capabilities. Resource/event size and event-count limits are part of registration and admission, not surprises after partial writes. A supported adapter must meet the requested profile; capability negotiation must never silently weaken it.

## Family assessment

| Family | Can implement mandatory bounded commit? | Required constraints | Optional or unsupported promises |
|---|---|---|---|
| MongoDB | Yes, plausible with transactional records | Supported replica-set/sharded topology; explicit durable concerns; guarded revision and receipt uniqueness | Snapshot queries and change streams need their own configuration and retention contract |
| DynamoDB | Yes, plausible within one supported transaction scope | Conditional transactional writes; bounded item count/size; authoritative base-table reads | Arbitrary snapshot scans and strong GSI queries are unavailable natively |
| FoundationDB | Yes, plausible with conflict-protected keys | Short transactions, bounded affected data, durable receipt keys | Long-lived database snapshots exceed normal transaction lifetime; query/index layer is adapter work |
| RocksDB | Yes, plausible within one database instance | Transactional conflict checks, WAL and synchronous durable acknowledgment | Replicated availability and distributed transactions require additional infrastructure |

These are feasibility judgments, not claims that an adapter already exists.

### MongoDB

Use a transaction containing the guarded resource update, unique action receipt, and journal inserts. MongoDB supports atomic transactions spanning documents and collections. Configure snapshot read concern when a coherent multi-record view is required and majority write concern for the documented majority-committed snapshot guarantee. Local reads across shards can observe only part of a committed transaction; atomic writes do not make arbitrary outside reads a coherent bundle. [MongoDB transactions](https://www.mongodb.com/docs/manual/core/transactions/)

Standalone deployments lack multi-document transactions, and the default transaction lifetime is below one minute. Treat topology, lifetime, storage engine, and journaling configuration as adapter admission requirements. Embedding all history in one ever-growing resource document is not a general replacement for a retained journal. [Production considerations](https://www.mongodb.com/docs/manual/core/transactions-production-consideration/)

Commit uncertainty is a distinct state: MongoDB distinguishes retrying a whole transiently failed transaction from retrying a commit with `UnknownTransactionCommitResult`. Driver retry behavior stays private to the adapter; ROM still needs a durable application receipt across process restarts. [Driver transaction error handling](https://www.mongodb.com/docs/drivers/node/current/crud/transactions/)

### DynamoDB

Build `TransactWriteItems` with a condition on the resource update, a conditionally absent receipt, and event records. The service limits it to 100 distinct items, 4 MB aggregate, one account and Region; an individual item cannot exceed 400 KB. A condition must be attached to the resource update rather than a second operation on that same item. Physical indexes and bookkeeping also consume the adapter's transaction budget. [TransactWriteItems](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_TransactWriteItems.html)

Native request-token idempotency lasts ten minutes. Keep a ROM receipt with the action identity and canonical request digest for the advertised retry horizon. Transaction changes propagate gradually to GSIs and streams; records belonging to one transaction may interleave. `TransactGetItems` provides a coherent read of a bounded, identified item set. [Transaction behavior](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/transaction-apis.html)

Successful writes are durably persisted, but default reads are eventual. Use strong base-table reads for authoritative revision and receipt lookup; GSIs and streams support eventual reads only. This does not establish cross-Region transaction atomicity. [Read consistency](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.ReadConsistency.html)

Even a strongly consistent `Scan` lacks snapshot isolation. Therefore a paginated query cannot claim one database snapshot without an additional versioned representation or another proven protocol. [Scan API](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_Scan.html)

### FoundationDB

Normal reads participate in strictly serializable conflict detection; snapshot reads omit those conflicts. Protect both expected revision and receipt absence with normal reads or explicit conflict ranges. The adapter must not use snapshot reads for conditions and assume commit will validate them. Transactions normally cannot run longer than five seconds. [Developer guide](https://apple.github.io/foundationdb/developer-guide.html)

Transactions are limited to 10,000,000 bytes of affected data; keys to 10,000 bytes and values to 100,000 bytes. A resource may need chunking, but its chunks, receipt, and events must still fit one atomic transition. Oversize transitions are rejected, not split into independently committed fragments. [Known limitations](https://apple.github.io/foundationdb/known-limitations.html)

Versionstamped keys can order journal records by transaction version with an additional intra-transaction component. Watches only indicate value changes and may miss a change that returns to the starting value; use them to wake journal readers, never as durable events. These primitives remain adapter details. [API: versionstamps and watches](https://apple.github.io/foundationdb/api-python.html)

### RocksDB

An atomic `WriteBatch` alone does not protect a preceding revision read from concurrent writers. Use transaction conflict checks, including `GetForUpdate` for revision and receipt absence. A read snapshot chooses a consistent version; it does not by itself establish every necessary commit condition. [Transactions](https://github.com/facebook/rocksdb/wiki/Transactions)

For a durable profile, keep WAL enabled and acknowledge only after the required synchronous persistence. Default asynchronous writes can be lost on machine crash; disabling WAL can lose writes even on process crash. RocksDB provides snapshots and ordered iteration, useful for resource reads and journal scans. The adapter must bound snapshot lifetimes and distinguish process recovery from disk-loss recovery. [Basic operations](https://github.com/facebook/rocksdb/wiki/Basic-Operations)

## Outcomes, replay, and live reads

Return core-owned outcomes distinguishing committed, revision conflict, identity/request mismatch, definitely not committed, and unknown. Receipt lookup should distinguish found, absent at this read, expired, and unavailable. Absence is not proof that an in-flight attempt cannot commit later. FoundationDB explicitly distinguishes `commit_unknown_result`, which is no longer in flight, from timeout/cancellation errors that may still commit later. That vendor distinction illustrates why a universal timeout-equals-rollback rule is unsound. [Unknown results and cancellation](https://apple.github.io/foundationdb/developer-guide.html#the-commit-unknown-result-error)

Proposed retry rule: resolve or resubmit the same identity through the same atomic receipt-absence guard. Store the original outcome and request digest; reject a different request under that identity. Define receipt retention independently from journal retention. Deleting a receipt may permit an old action to execute again, so unlimited idempotence requires retained identity evidence, while bounded idempotence requires an explicit expiry contract.

The durable journal needs per-resource `(revision, event_ordinal)` ordering, stable event identities, tombstone events, replay after reconnect, and a retained lower bound. A cursor is an opaque ROM token scoped to adapter generation, journal partition, and filter contract. It must not be a timestamp high-water mark that skips a delayed writer with an earlier timestamp. Discovering pending events across resources also needs a crash-safe index or fair partition enumeration; per-resource history alone is insufficient for a global worker.

MongoDB resumes change streams only while sufficient oplog history remains. DynamoDB Streams retain records for up to 24 hours. Therefore neither native feed alone supplies an arbitrarily retained ROM journal; persist journal records in the commit and use feeds as optional wakeups. [MongoDB change streams](https://www.mongodb.com/docs/manual/changestreams/), [DynamoDB Streams](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Streams.html)

Live reads should reuse the same declared query and authorization semantics, with bounded invalidation and refresh. Startup requires a proven snapshot/change boundary or a reconciliation protocol covering concurrent changes. Calling a non-snapshot scan a snapshot does not close that race. Reconnection and cursor expiry must cause explicit resynchronization; coalesced live results and replayable committed events remain different products.

## Query semantics and negative capabilities

Define semantic field predicates, presence/null behavior, exact numeric comparison, ordering, projection, and pagination independently of SQL or vendor query languages. MongoDB's equality-to-null predicate also matches a missing field, so translating ROM's explicit-null test directly would change meaning. [Null queries](https://www.mongodb.com/docs/v8.0/tutorial/query-for-null-fields/)

DynamoDB `Query` requires partition-key equality, orders by sort key, and applies its 1 MB limit before filters. An empty filtered page may still have continuation. Arbitrary cross-resource ordering requires a different access path, not a renamed native `Query`. [Key conditions and pagination](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Query.KeyConditionExpressions.html)

For ordered key-value stores, the adapter builds typed indexes and preserves ROM comparison semantics. Index readiness, supported predicate combinations, stable ID tie-breaking, cost limits, and page consistency must be advertised. A full scan is acceptable only under an explicit bounded execution policy; unsupported combinations must fail clearly. Optional multi-resource atomicity needs explicit scope and limits and must not enter the mandatory single-resource action contract.

Redis is a useful negative control: execution-time errors inside `MULTI/EXEC` do not roll back other commands, and the default AOF policy can lose roughly a second of writes. A generic transaction wrapper therefore does not prove ROM conformance. Any Redis design needs its own bounded encoding, failure proof, and durability profile. [Transactions](https://redis.io/docs/latest/develop/using-commands/transactions/), [Persistence](https://redis.io/docs/latest/management/persistence/)

## Proposed failure tests

1. Race two actions against one revision: exactly one transition wins; its state, receipt, and complete event set agree.
2. Kill after durable commit but before response; retry after restart and beyond native idempotency windows. Obtain the original result without another revision or event.
3. Delay a timed-out commit, observe receipt absence, then retry concurrently. At most one identity may commit; absence must not be reported as final rollback.
4. Inject failure between every physical write and during durable flush. Recovery exposes either the complete transition or none; no durable success precedes required persistence.
5. Exceed size/event/index budgets by one unit. Reject without partial state; repeat with deletion and recreation of the same resource ID.
6. Replay across page boundaries, concurrent writers, partition changes, restart, and retention expiry. No retained event is skipped; duplicates retain stable identities; gaps are explicit.
7. Change filter membership, ordering, deletion, and authorization during live startup. Verify refresh converges and never exposes unauthorized fields.
8. Compare missing, null, decimal boundaries, equal sort keys, empty filtered pages, and unsupported predicates across adapters. Reject unsupported snapshot or multi-resource requests instead of degrading them.
