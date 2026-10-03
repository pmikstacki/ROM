# Proposed integration of the query strategy selector

Date: 2026-10-03. This is a source review and design proposal, not an implemented selector.
The maintained baseline is `9ff10f72465341195232591a16a3e3f5ac60708d`, with the concurrent reference-upgrade demonstration changes.
This review changes no core or adapter behavior. It does not rerun the historical benchmarks.

## Evidence and present boundaries

The standalone selector is at `93d080510eaffde6c104e41451fccf5bb5abb1f9`, in `prototypes/query-planning/src/selector.rs`.
Its `choose` function compares Core and Native costs after checking plan equality, generation and exactness.
Its `PlannerAdapter` separates optional estimates from execution. An estimate failure permits fallback; an execution failure propagates.
SQLite owns the physical SQL plan. The selector does not force an index.
redb supplies only the reference path in that prototype.

The [selector results](query-selector-results.md) report seven selector tests and eight inherited conformance tests.
The [independent review](query-selector-independent-review.md) records 168 additional comparisons and fault checks.
These are fixed-schema experiments with synthetic visibility, session-local statistics and an in-sample cost model.
They do not establish maintained runtime authorization, durable index recovery or general cost accuracy.

The maintained seams are:

| Responsibility | Current source |
| --- | --- |
| Typed and live observations | [`crates/rom/src/query.rs`](../../crates/rom/src/query.rs) |
| Query and sort grants, normalization, scalar evaluation, selection | [`crates/rom/src/query_eval.rs`](../../crates/rom/src/query_eval.rs), especially `query_plan` and `select_rows` |
| Actor rechecks, commit gate and disclosure generation | [`crates/rom/src/execution.rs`](../../crates/rom/src/execution.rs), `Runtime::observe` |
| Wire query and anchor representation | [`crates/rom/src/query_spec.rs`](../../crates/rom/src/query_spec.rs) |
| Full-kind bounded storage read | [`crates/rom/src/persistence.rs`](../../crates/rom/src/persistence.rs), `Storage::snapshot` |
| SQLite read and atomic commit | [`crates/rom-sqlite/src/persistence.rs`](../../crates/rom-sqlite/src/persistence.rs) |
| redb read and atomic commit | [`crates/rom-redb/src/storage.rs`](../../crates/rom-redb/src/storage.rs), [`commit.rs`](../../crates/rom-redb/src/commit.rs) |
| Shared native query tests | [`tests/persistence/tests/query_planning.rs`](../../tests/persistence/tests/query_planning.rs) |

Both maintained adapters currently load a bounded kind snapshot. Core selects and sorts the result.
There is no maintained native query capability, secondary field-index catalog, cost adapter or plan cache.
The existing reference-edge indexes enforce integrity. They are not query indexes.

## Proposed integration seam

Keep one canonical plan and one semantic evaluator. Extract these responsibilities from `query_eval.rs` before adding a physical selector.
Typed, wire, projected and live queries should enter the same path.
Query and sort grants must remain before storage planning or access.

Add an optional storage query capability with a default reference implementation.
The capability should expose a coherent read session, or an equivalent single adapter operation.
Admission metadata, generation, index readiness, estimates and execution must refer to that same native snapshot.
Separate `row_count`, `epoch`, `estimate` and `execute` calls without a snapshot boundary are insufficient.
The runtime gate serializes one runtime. It does not make separate database connections share a snapshot.

Keep the core authority callbacks outside the physical cost model.
An adapter supplies exact candidates or ordered rows under an explicit supported-shape contract.
Core retains row authorization, field disclosure, final page selection and projected/typed disclosure.
Bind any capability result to the canonical plan, descriptor version, index generation and semantic version.
Unsupported operators or shapes use the reference path. Low estimated cost never grants semantic support.

Initially, support SQLite plan inspection and a redb reference adapter.
Add redb index execution only with its own exact key encoding and recovery tests.
Do not invent a SQL planner interface for redb or import the prototype's fixed `amount` schema.
Use one SQL construction routine for EXPLAIN and execution, including actual bindings and bounds.
Unknown SQLite versions or plan nodes should disable the estimate, not reject a valid reference query.

## Required result and error equivalence

The reference result includes errors, authorization order and limits, not only returned IDs.
The [maintained query review](maintained-query-independent-review.md) already records these boundaries.

- Before any storage access, reject denied predicate or sort grants, including an empty kind.
- For field-sorted queries, examine every row-visible live row for ordered-field read grants before predicate or anchor filtering.
  A visible row with a protected sort field produces `Denied`, even when the predicate excludes that row.
  Native predicate pushdown alone would omit that error and change the contract.
- Row-hidden rows do not require sort-field disclosure. A native preflight cannot deny merely because such a row exists.
- Apply the result limit after row policy and final ordering. An SQL limit before policy can produce a short or wrong page.
- Preserve the ID-order fast path. It currently stops row policy evaluation when the page is full, after full snapshot admission and duplicate checks.
- Preserve missing versus null, exact `u64` values, signed extremes, finite floats, bytewise string order and ascending ID ties.
  Preserve structural equality and custom codec normalization. Generic SQLite JSON casts do not establish these guarantees.
- Preserve anchor binding to normalized predicates, order, kind and schema version. Anchors remain moving boundaries, not snapshot tokens.
- Preserve current actor rechecks and live invalidation. A cached plan or result cannot cache an authorization decision.
- If selected execution fails, propagate the failure. Do not retry a different strategy and hide a storage or integrity error.

The first sorted native path therefore needs a complete authorization preflight in the same snapshot.
It can skip that preflight only with a separately designed, explicit proof that equivalent authorization is independent of omitted rows.
`allow_all_fields()` currently installs callbacks; it does not retain such a capability proof.
Do not infer one from a callback address or observed successful calls.
The preflight may dominate latency. Measure that cost before claiming a useful optimization.

## Admission and cost metadata

Maintain the current whole-kind row and byte limits under every strategy.
These limits include persisted tombstone Rows, not only live matching rows.
A selective query with a small result limit must still reject an oversized kind.
Statistics cannot authorize admission. Fresh and stale estimates must produce the same admission result.

If metadata replaces the full admission scan, maintain exact row counts and serialized full-Row byte totals atomically with each mutation.
Include create, replacement, deletion, tombstone purge and maintenance reconstruction.
Use checked arithmetic and verify metadata against authoritative rows at startup or explicit validation.
Preserve existing adapter accounting and core checks for invalid scope and duplicate identities.
Do not substitute payload bytes, index bytes or estimated result size for the documented Row-byte budget.

Keep cost estimates separate from exact admission counters.
Cache keys need canonical plan, descriptor/index versions and a durable mutation generation, or an explicitly ephemeral session boundary.
Invalidate on schema migration, index replacement, restore and relevant committed writes.
Unknown, stale, mismatched or overflowing estimates cannot select an unproven path.
Never reuse cached row visibility across actors or authority changes.

The benchmark reports executor-only VM steps and candidates; selected latency includes admission and EXPLAIN.
Candidate bounds do not limit SQLite internal scan or sort work.
A future wall-clock, VM-step or streaming budget needs an explicit common contract and cancellation tests.
It must not silently replace the present bounded-kind contract.

## Index lifecycle and recovery

Declare supported field indexes through a separate physical index contract.
Do not silently change logical Resource descriptor equality merely to tune a backend.
Index entries should derive from current rows, including exact presence/null tags and scalar encodings.
Historical receipts, events and work payloads are not current query entries.

Update index entries and exact admission counters in the same native transaction as the Resource bundle.
Use differences between old and new entries; a payload-only change should not rewrite unchanged index entries.
Receipt replay, rejected mutations and transaction rollback must leave indexes and generations unchanged.
Add interruption checkpoints for each actual write and exercise unknown acknowledgement after commit.

Choose an explicit persisted index inventory and native-format compatibility rule.
SQLite currently validates a fixed object inventory in [`snapshot.rs`](../../crates/rom-sqlite/src/snapshot.rs).
Adding tables or indexes without updating maintenance validation would make valid stores fail to open.
The redb inventory and reconstruction are in [`maintenance.rs`](../../crates/rom-redb/src/maintenance.rs).

Prefer rebuilding derived indexes from validated current rows during staged restore or migration.
Publish only after row/index/counter consistency checks succeed.
Do not silently serve from an incomplete index after interruption.
If online rebuild is later supported, define how its snapshot and subsequent writes meet before the index becomes ready.
An incomplete index must remain unavailable to selection.

The logical [`Snapshot`](../../crates/rom-backup/src/model.rs) currently contains descriptors and authoritative reference edges, not query indexes.
Decide whether index declarations belong in host configuration or a versioned archive field.
Derived entries need not be archived, but restore must reproduce the declared usable state or select the reference path explicitly.
Preserve retry epochs, receipt identities, reaction work, journal generations and external blob boundaries during all reconstruction.
Reuse staged publication from existing adapter maintenance; do not build another publication protocol for indexes.

## Tests and measured acceptance

Extend the existing shared query tests with forced Core and forced Native modes available only to tests.
Compare complete Rows and exact errors across typed, wire, projected and live APIs.
Include a predicate excluding every protected-sort row, an anchor beyond those rows, empty kinds and denied standalone grants.
Include both row and byte overflow with tiny output limits, tombstones, stale estimates and failed optional planning.
Add revoked actors during observation, duplicate/tampered index entries, unknown plan nodes and execution failures without fallback.

For each adapter, test commit rollback, lost acknowledgement, replay, reopen, interrupted rebuild, backup/restore, schema migration and retention.
Check exact index contents and admission metadata against a fresh derivation from current rows.
Test two connections so admission and execution cannot accidentally depend on one runtime gate.

Benchmark file-backed adapters under the maintained policies and receipt pipeline.
Include indexed/unindexed, selective/broad/no-match, skewed holdout data, protected fields and write-heavy cases.
Record end-to-end latency, admission/EXPLAIN overhead, candidates, VM work where available, decoded bytes, memory and write amplification separately.
Rotate execution order and retain raw samples, source, compiler and lockfile evidence.
The prototype's 18 in-sample cases justify further evaluation; they do not establish a production speedup.

## Separate core cohesion recommendations

These refactors are proposed cleanup work. They are not prerequisites for changing query semantics.
Follow [`docs/quality.md`](../quality.md): facade roots, stable exports, cohesive modules and no duplicated contracts.

| Existing source responsibility | Proposed boundary |
| --- | --- |
| `resource.rs`: `Field`, scalar/container codecs, `FiniteF64`, `ResourceRef`, `Input` | Field/input codec module; retain public exports |
| `resource.rs`: `FieldRef` and typed `Query` construction | Typed query authoring module beside canonical query definitions |
| `resource.rs`: `Definition`, action erasure and `Registered` implementation | Registration module; keep typed declarations and erased policy adaptation together |
| `resource.rs`: `Command` and `Snapshot` | Typed command/result module |
| `execution.rs`: `Builder` registration and `build` | Runtime composition module |
| `execution.rs`: lifecycle, tracked I/O, status and shutdown | Runtime lifecycle module; preserve drop-before-drain ordering |
| `execution.rs`: actor authority and current disclosure | Authority module shared by reads, writes and receipt replay |
| `execution.rs`: invocation and `run` | Command execution module; keep transaction ordering visible in one implementation |

Do not split `run` at an arbitrary line count.
Its receipt lookup, current authorization, exact replay codec, preparation, race recheck and commit form one ordered contract.
Extract a shared helper only when both branches have the same guards and guarantees.
Keep backend-specific atomic transactions in their adapters.
After a structural change, run affected behavior tests, downstream consumers, doctests and the full local verifier.
