# Adapter query contract

`Storage::query_read(&StorageQuery, QueryBounds)` is the persistence boundary for query selection.
Its default calls the existing bounded `snapshot` once and returns `QueryRead::Reference`.
Existing adapters retain their behavior without a native implementation.
The SQLite and redb adapters currently use this default.

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
Use the same calibrated relative units for both costs.

`QueryCost` scores `startup + rows * per_row + bytes * per_byte` with checked arithmetic.
This is a comparison input, not a latency prediction supplied by ROM.
Cost overflow, missing estimates, unsupported scalar capability, stale bindings and ties select reference execution.
Only a matching permitted native path with a strictly lower score can win.

Database-specific plan inspection stays in the adapter. SQLite chooses its own physical plan; a key-value adapter describes its actual range-read capability.
Index maintenance, snapshot metadata integrity, rebuild and measured coefficients are separate adapter responsibilities.
