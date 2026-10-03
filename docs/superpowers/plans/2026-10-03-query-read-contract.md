# Query read contract implementation plan

> For agentic workers: use the subagent-driven development workflow for the independent tasks below.

**Goal:** Connect maintained query selection to an owned persistence operation and a pure strategy selector.

**Architecture:** Core grants and normalizes queries before adapter access. Adapters return owned reference rows or a complete candidate set from one coherent read. Core retains residual selection and disclosure.

**Tech stack:** Rust 1.99.0, existing ROM contracts and test adapters. No new dependencies.

**Spec:** [Maintained query strategies](../specs/2026-10-03-maintained-query-strategies-design.md).

## Constraints and scope

Keep opaque policy ordering, whole-kind admission and public API compatibility.
Keep `lib.rs` and `mod.rs` as facades. Put protocol, selection and response checks in named modules.
Native index storage, transactional index maintenance and measurements remain subsequent stage-three work; this change must not mark them complete.

## Interfaces

- `Definition::read_policy(fn(&Actor) -> bool)` replaces only read authorization. A later `policy(...)` clears it.
- Internal `Registered::uniform_read(&Actor) -> Option<bool>` evaluates the explicit read rule; `uniform_fields() -> bool` reports the explicit field grant.
- `Storage::query_read(&StorageQuery, QueryBounds) -> Result<QueryRead>` defaults to one bounded `snapshot` call.
- `StorageQuery` contains `spec: QuerySpec`, `descriptor: Descriptor`, `semantics: u32`, `selection: SelectionMode`.
- `SelectionMode` has `ReferenceOnly` and `UniformReadAndFields`.
- `QueryBounds` contains `max_rows` and `max_bytes`; `KindAdmission` contains `rows`, `persisted_bytes` and `canonical_bytes` (all `usize`).
- `QueryRead` has `Reference { rows }` and `NativeCandidates { rows, admission, binding: Box<ReadBinding> }`.
- `ReadBinding` contains the exact `request: StorageQuery` and `snapshot: QuerySnapshot`.
- `QuerySnapshot` contains `store: String`, `generation: u64`, `profile_version: u32`, `encoding_version: u32`.
- Version 1 identifies the query semantics and scalar candidate profile/encoding. A store identity must be nonempty and at most 256 bytes.
- `QueryCost` contains `startup`, `rows`, `per_row`, `bytes`, `per_byte` (`u64`); checked score is startup + rows*per_row + bytes*per_byte.
- `QueryEstimates` contains `binding`, `complete_candidates: bool`, `reference: QueryCost`, `native: QueryCost`.
- `select_query_strategy(&StorageQuery, &QuerySnapshot, Option<&QueryEstimates>) -> QueryStrategy` returns `Reference` or `NativeCandidates`.
- Internal `validate_query_read(&StorageQuery, QueryBounds, QueryRead) -> Result<Vec<Row>>` validates admission, binding, kind, duplicates and candidate budgets; it returns rows in ID order.

The adapter supplies snapshot identity from its own active read transaction. The selector compares estimates against that exact identity.
Core can validate response consistency, but cannot independently prove a store generation or candidate completeness from a response.
Conformance tests and native integrity checks must establish those adapter obligations.
Estimates only select execution; they never establish admission or authority.
Optional invalid estimates select reference. Invalid responses and execution errors fail without a second read.

Implementation ruling: box the native response binding to keep the default reference variant compact.
Clippy identified the unboxed enum as 416 bytes versus a 24-byte reference payload.
The native path pays one allocation; the reference path does not.
The cost estimate still owns its binding directly because it is not an enum payload.

## Review focus

1. Opaque callbacks on excluded rows must retain their previous behavior.
2. Read denial must precede adapter access even for empty or oversized kinds.
3. Final disclosure must recheck current authority after candidate selection.
4. Adapter omissions cannot be certified by metadata alone; distinguish consistency checks from trusted completeness.
5. Planner failure can fall back before execution, but execution failure must not trigger a second read.

## Tasks and ownership

### 1. Explicit read-selection policy (agent: ste_specs)

Files: `resource/definition.rs`, separate private definition tests.

- [x] Write and run failing tests for read/write independence, policy override order and clearing field proof.
- [x] Implement the fluent method and internal proof methods. Keep field disclosure decoding unchanged.
- [x] Run core tests and all-feature Clippy.

### 2. Owned adapter protocol and selector (agent: release_input_ergonomics)

Files: `query_storage.rs` facade and private modules, `lib.rs` exports, `persistence.rs` default method.

- [x] Write failing tests for default snapshot, stale/wrong request estimates, opaque mode, unsupported shapes/versions and overflow.
- [x] Implement protocol, checked cost selection and owned-result validation with focused negative tests.
- [x] Preserve reference validation order and error categories. Reject native mode upgrades, bad bindings and impossible admission counts.
- [x] Run core tests and all-feature Clippy.

### 3. Shared runtime integration (coordinator)

File: `query_eval.rs`.

- [x] Run integration tests from task 4 and confirm they expose the absent runtime path.
- [x] Evaluate uniform read once after normalization; deny before storage if false.
- [x] Request native eligibility only with uniform read and fields. Validate the owned response before evaluation.
- [x] Skip row-policy decoding for explicit uniform reads and sort-field preflight only for uniform read plus fields.
- [x] Retain residual predicates, anchors, ordering, page limit and final disclosure for all paths.

### 4. Runtime conformance and review (agent: ste_guides)

File: `tests/persistence/tests/query_read.rs`.

- [x] Add real-storage wrapper tests for legacy fallback, uniform/native equivalence, builder overrides and current read authority.
- [x] Add fault responses for wrong binding, kind, duplicates, bounds and unauthorized native mode; verify no fallback after failure.
- [x] Test callback timing and excluded-row decoder behavior; keep existing opaque oracle tests.
- [x] Independently review the combined implementation after coordinator integration.

### 5. Integration evidence (coordinator)

- [x] Update OpenSpec scenarios and public author/adapter documentation.
- [x] Run full `./scripts/check`; record source, compiler, command and limitations.
- [x] Integrate verified changes without marking native indexes or release complete.
