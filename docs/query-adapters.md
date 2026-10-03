# Adapter query contract

`Storage::query_read(&StorageQuery, QueryBounds)` is the persistence boundary for query selection.
Its default calls the existing bounded `snapshot` once and returns `QueryRead::Reference`.
Existing adapters retain their behavior without a native implementation.
The redb adapter uses this default. SQLite implements the scalar candidate profile below.

Core checks predicate and sort grants, normalizes operands through their actual codecs, and evaluates any explicit actor-only read rule first.
The request contains the normalized query, full descriptor, semantic version and permitted selection mode.
It contains no application callbacks or database handles.

## One coherent read

A native adapter must perform these steps inside one native read transaction:

1. Establish exact whole-kind admission counts, including tombstones and protected Row metadata.
2. Reject counts that exceed the row or byte limits.
3. Inspect optional plans for this request and snapshot.
4. Call `select_query_strategy` with those estimates.
5. Materialize the selected reference rows or complete native candidates.
6. Release native connection guards before returning owned rows.

An optional estimate failure selects reference execution before materialization.
After execution starts, propagate errors. Do not silently retry another executor.
Application policies, custom codecs and actor gates must not run under adapter connection locks.

`KindAdmission` separates persisted Row-text bytes from canonical serialized Row bytes.
Both totals must fit `QueryBounds::max_bytes`. Each count covers the whole kind, not only matching rows.
This preserves limits for older SQLite JSON text whose length differs from its canonical encoding.

## Native candidate profile

Version 1 permits complete scalar-predicate candidates only when core supplies `UniformReadAndFields`.
Core still evaluates residual predicates, moving anchors, ordering, page limits and final disclosure.
Do not apply a result LIMIT in the database before that evaluation.
The native path must preserve exact numeric, missing/null and UTF-8 ordering semantics from [the query contract](queries.md).

Return the exact `StorageQuery` in `ReadBinding` with the store identity, committed generation and supported profile/encoding versions.
Planning and materialization must use that same snapshot identity.
The store identity is an opaque nonempty string of at most 256 bytes; it is not a credential or client cursor.

Core rejects unexpected native mode, incorrect request bindings, unsupported versions, wrong-kind rows, duplicate IDs and inconsistent candidate counts or byte totals.
Whole-kind admission overflow returns `TooLarge`. These response errors do not trigger a fallback read.

Core cannot independently prove candidate completeness or a store generation from these fields.
Adapters remain trusted implementations of persistence. Their integrity validation and conformance tests must prove these obligations.
The test adapters that exercise this interface are not evidence of a production index implementation.

## Cost selection

`QueryEstimates` binds both costs to the exact request and active snapshot.
`complete_candidates` asserts the adapter's supported candidate semantics; it is not an authorization flag.
Use the same relative units for both costs. Calibrate them against the complete operation before making release performance claims.

`QueryCost` scores `startup + rows * per_row + bytes * per_byte` with checked arithmetic.
This is a comparison input, not a latency prediction supplied by ROM.
Cost overflow, missing estimates, unsupported scalar capability, stale bindings and ties select reference execution.
Only a matching permitted native path with a strictly lower score can win.

Database-specific plan inspection stays in the adapter. SQLite chooses its own physical plan; a key-value adapter describes its actual range-read capability.
Index maintenance, snapshot metadata integrity, rebuild and measured coefficients are separate adapter responsibilities.

## SQLite implementation

Format 7 maintains a key for every scalar field of every live Row, including optional missing fields.
The primary key groups memberships by Resource identity. A separate index groups scalar values by kind and field.
State, reference edges, derived keys, exact kind counters and generation change in the same commit.
An unchanged field key stays in place. Receipt replay and rejected writes do not change the generation.

The native reader selects one supported predicate and returns its complete candidate set.
Other predicates, sort order, moving anchors and result limits remain in core.
It uses bound BLOB values for exact scalar comparisons. It does not put a result LIMIT in SQL.

Plan inspection and execution use the same SQL and parameters inside one read transaction.
The first plan recognizer accepts specific output from the bundled SQLite 3.53.2 engine.
An unknown version or plan selects the reference path. Native execution errors propagate.
The current cost weights and selectivity fractions are heuristics. They are not measured latency estimates.
The release still needs skewed workloads, write amplification, allocations and memory measurements before tuning these weights.

Startup and backup validate all derived memberships and counters against authoritative data.
Validation also checks the native index layout. Physical entries count toward the complete native validation budget.
Logical archives omit derived keys; restore, migration, retention and format upgrade reconstruct them before publication.

For corrupt derived contents, keep the source offline and call `Sqlite::rebuild_indexes_from(source, destination, limits)`.
The destination must be fresh. The source must retain the known table inventory and valid authoritative records.
This operation discards corrupt derived contents and builds a validated replacement. It does not repair authoritative corruption.
Publication gives the replacement a fresh index identity and fences old journal cursors and work claims.
An interrupted stage is not published. The original source remains unchanged.
