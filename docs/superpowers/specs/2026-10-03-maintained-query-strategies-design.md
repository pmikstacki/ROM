# Maintained query strategies

Status: the owned core read seam and explicit read-policy contract are implemented.
SQLite scalar index integration and recovery have passed local verification.
See the [integration report](../../research/sqlite-query-index-results.md) for scope and evidence.
Cost coefficients remain uncalibrated; release measurements remain pending.
See the [core seam plan](../plans/2026-10-03-query-read-contract.md) for exact types and tests.

## Intent

Integrate the measured strategy selector into the maintained ROM query pipeline.
Keep database planning inside the adapter. The core selects a supported execution
strategy and retains the meaning of Resource queries, authorization and limits.
Typed, wire, projected and live observations use this same path.

The owner requires measured optimization, generic resource declarations and
cohesive modules. The design must not replace existing opaque policies with a
different implicit authorization model. It must not import a SQL engine into core.

## Alternatives and selected direction

| Approach | Assessment |
| --- | --- |
| Always use the core evaluator | Keep as the semantic reference and fallback. It does not satisfy indexed execution or measured selection. |
| Push predicates and limits into every backend | Reject. It can skip authorization errors, callback panics and matching rows hidden before a premature limit. |
| Coherent adapter operation plus explicit semantic gates | Select. Compare costs only between paths that can implement the same query contract. |

The standalone selector provides evidence for separating capability, freshness and
cost. Its synthetic policies and fixed table do not establish maintained semantics.
See [the integration review](../../research/maintained-selector-integration-review.md).

## Reference semantics

The existing opaque policy path is unchanged. Query/sort grants run before storage
access. Exact whole-kind row and serialized Row-byte limits apply before selection.
Tombstones count toward admission. Duplicate identities or wrong-kind rows fail.

For sorted queries, row-visible data must pass sort-field disclosure before the
predicate or anchor can exclude it. ID-order queries retain their earlier page
stop: callbacks after a full page are not evaluated. Callbacks can panic; therefore
skipping a nonmatching row is not automatically equivalent.

Custom Resource decoders are also executable code. A descriptor does not prove
that a decoder is total or free of side effects. Do not infer that property from
a derive marker, function address or previous successful values.

## Explicit read-selection contract

An optional actor-only `.read_policy(fn(&Actor) -> bool)` exposes an explicit optimization boundary.
Its input is `&Actor`, not `&Resource`. Its contract explicitly authorizes read
selection without decoding every candidate through a resource callback. Returned
values still use the normal projection, field disclosure and typed decode path.

This is a new explicit policy contract, not a reinterpretation of `.policy(...)`.
The reference evaluator and native path must implement the same new contract.
Old opaque policies remain supported through their existing evaluation order.
An actor-only denial is evaluated before adapter access, including an empty kind.

Track an explicit unconditional field-grant state from `allow_all_fields()`.
An arbitrary `field_policy(...)` clears that state. Sorted native selection needs
that proof. The initial native mode requires both uniform read and field grants;
mixed or fully opaque policies receive the full reference rows. A future policy
preflight is a separate design, not a callback executed under native locks.

A later `.policy(...)` clears the actor-only read mode. A later `.read_policy(...)`
replaces the read decision, while the ordinary policy still controls writes.
Direct reads and receipt disclosure use the same current read rule. Retained
receipt codecs, tombstone context and field projection remain independent checks.
For queries, actor-only denial precedes storage admission even for an empty kind.
This ordering is part of the new explicit contract, not a change to opaque policies.
Evaluate that callback once per query-selection attempt, after existing query
grants and normalization. Reuse its result for eligibility and selection; do not
invoke it per candidate. Observation retries begin a new attempt. Existing final
disclosure checks still apply to returned rows under the same current read rule.

The reference evaluator for this new mode skips the per-row Resource decode used
only to call the opaque row policy. With the explicit uniform field grant, it also
skips decoder/field-callback preflight for sort keys. This rule applies to both
reference and native execution of the new mode. Otherwise a decoder panic on an
excluded row would distinguish the two paths. Final returned-row decoding and
field projection remain common and mandatory. Mixed opaque field rules retain
their full reference preflight and cannot request native candidates initially.

## Coherent persistence boundary

Add one object-safe adapter-owned read operation to `Storage`. Its default
implementation wraps the existing bounded snapshot. A native implementation holds
one read transaction through admission, optional planning and materialization.
It returns owned Rows after it releases its connection guard.

The core supplies the normalized request, bounds and permitted selection mode.
The adapter calls a pure shared ROM selector with cost/capability evidence inside
that transaction. It invokes no application policy, actor gate or custom codec.
This avoids reentrant storage deadlocks and policy panics that poison a native mutex.
The core applies opaque policies and final disclosure after the adapter returns.

The operation binds:

- Exact whole-kind row count and serialized Row-byte total for admission.
- A descriptor/index/commit generation identity bound to that same snapshot.
- A full reference read or a permitted complete native candidate set.
- Optional native capability and cost inspection for the exact normalized plan.
- Execution of that native plan inside the same transaction.

The core normalizes and checks query grants before entering the operation.
The adapter checks exact admission before optional planning. Returned admission
metadata remains bound to the same request and is checked again by core.
Estimate failure can select the reference path;
native execution failure propagates. It must not retry a different executor and
hide a storage error. Stale or mismatched estimates cannot select native execution.

The public adapter types contain no HTTP types or database handles. They include
an explicit semantic contract version. Native requests contain only the normalized
supported query and shape information. The adapter receives no authority to make
policy decisions. No API lets application code mutate query indexes independently.

The interface to implement is:

```text
Storage::query_read(&self, request: &StorageQuery, bounds: QueryBounds)
    -> Result<QueryRead>

StorageQuery: normalized QuerySpec, Descriptor and semantic version,
              SelectionMode::{ReferenceOnly, UniformReadAndFields}
QueryBounds: max_rows, max_bytes
QueryRead: Reference { rows: Vec<Row> }
         | NativeCandidates { rows: Vec<Row>, admission: KindAdmission,
                              binding: Box<ReadBinding> }
```

`KindAdmission` contains exact row count, persisted Row-text bytes and canonical
serialized Row bytes. Both byte totals preserve the existing SQLite and core
limits, including noncanonical older JSON text. `ReadBinding` contains the exact
request identity, schema/profile/encoding versions, store identity and committed
generation from the same read transaction. It is not a client cursor or auth token.

The first native profile materializes a complete scalar-predicate candidate set
without a backend result LIMIT. Core evaluates residual predicates, anchors and
ordering, then truncates. Whole-kind admission bounds the materialization. Native
LIMIT or policy-aware chunk refills need separate proof and are not implied here.

An opaque definition always requests `ReferenceOnly`. The adapter must honor it
regardless of its estimates. Native candidates with mismatched request binding,
wrong-kind rows, duplicates or exceeded bounds fail instead of triggering fallback.

## Physical indexes and compatibility

Use adapter-owned physical index declarations separate from Resource descriptor
equality. Prefer a fixed scalar-entry schema over per-resource SQL DDL. Index
entries derive from current rows and preserve missing, null and value distinctions.
Exact scalar encodings must preserve the core's unsigned, signed, finite-float,
UTF-8 byte order and ID tie rules. Unsupported shapes select the reference path.

Maintain entries, row/byte counters and mutation generation in the same native
transaction as the Resource bundle. Replay and rejected writes change none of
them. Compare old and new keys to avoid rewriting unchanged index entries.
Derived index state is not a replacement for authoritative rows or receipts.

Use explicit native format compatibility for new tables and metadata. Rebuild
derived entries from validated rows during staged restore and schema migration.
Validate the complete derived state before publication or indexed startup. An
interrupted or invalid index cannot be marked ready. Keep native publication and
recovery in the existing maintenance protocol.

SQLite's planner chooses its access path. Use the same SQL and bindings for plan
inspection and execution. Unknown planner output disables the estimate. A redb
adapter must describe its actual capability; it must not pretend to have a SQL
planner. Its reference path remains valid while any separate native index path
must pass the same contract tests.

## Verification and measurements

Differential tests compare complete results and exact errors with selection forced
to core or native under the same read-policy contract. Cover typed, wire, projected
and live observations, current authority changes, anchors, scalar extremes,
missing/null, small limits, oversized kinds, tombstones and corrupt index state.
Include a custom decoder that panics only on excluded rows, an actor-only callback
that counts its invocations, explicit denial on empty/oversized kinds and builder
overrides that clear each optimization proof. Compare reference and native phases
under the new contract as well as the unchanged opaque-policy oracle.

Test commit rollback, unknown acknowledgement, replay, reopen, two database
connections, interrupted rebuild, migration, retention and backup/restore.
Compare derived entries and exact counters against fresh authoritative derivation.

Measure file-backed selective, broad, no-match, skewed and write-heavy workloads.
Include full admission, policy and planner overhead in end-to-end timings. Record
allocations/memory, decoded bytes, native work counters and write amplification
separately. Rotate trial order and retain raw samples with source/toolchain metadata.
Choose a strategy only from these supported paths; do not promise a universal win.

## Remaining design work

The core seam fixes the adapter types, actor-only policy API and scalar eligibility gate.
SQLite uses a fixed all-scalar profile with per-Resource primary membership keys and a scalar-value secondary index.
Format 7 maintains keys and exact counters atomically. Fresh-destination maintenance rebuilds them from authoritative records.
The remaining optimization work is measured selection, write and memory cost assessment, and the final independent integration review.
