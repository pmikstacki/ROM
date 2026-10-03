# Maintained query cost measurement plan

> For agentic workers: implement parallel tasks with separate ownership and an independent review before integration.

**Goal:** Measure maintained native/reference queries, writes and memory, then improve selection using that evidence.

**Architecture:** A non-published measurement binary exercises the real Runtime and SQLite adapter. Test-only adapter controls choose automatic, reference or permitted native execution. Identical Resources, policies, queries and databases establish semantic equivalence. Timing and heap profiling are separate runs.

**Tech stack:** Rust 1.99.0, existing Tokio/Rayon/rusqlite; optional DHAT in the measurement package only.

**Spec:** [Maintained query strategies](../specs/2026-10-03-maintained-query-strategies-design.md), release tasks 3.3 and 3.4.

## Constraints and alternatives

This is the existing release measurement work, authorized for autonomous parallel execution. It adds no new product policy.
Reuse the current isolated `release-query-index` worktree. Preserve historical experiments and evidence.
A synthetic evaluator or raw SQL microbenchmark alone cannot establish Runtime cost. Use full public Runtime queries, plus native counters.
Reference and native modes must retain the same uniform authorization contract. Test controls cannot bypass capability, admission or semantic checks.
Separate heap profiling from latency. SQLite C allocations are outside a Rust global allocator; report that limitation and process RSS separately.
Seed independent and skewed field distributions deterministically. Rotate execution order. Retain raw samples and exact source/build metadata.
Index write and space cost must be visible. Compare same indexed database query strategies and separately assess indexed versus pre-index bundle writes.
No timing assertion belongs in ordinary CI or the correctness suite. No universal speedup claim or production profile is implied.

## Review focus

1. A forced reference path must not retain different auth or a different logical result than native.
2. Instrumentation can distort timing; verify unprofiled and separately profiled measurements.
3. Broad predicates on skewed data can fool fixed selectivity fractions; include them and an independent validation set.
4. Full pipeline overhead includes metadata, planning, decode, core filtering and final disclosure.
5. Peak RSS includes native allocations and setup. Label phase scope rather than calling it precise Rust live heap.

## Tasks

### Adapter measurement controls — release_input_ergonomics

Own `crates/rom-sqlite` code and dedicated adapter tests. Do not alter production cost weights yet.

- [x] Add test-first `QueryExecution::{Automatic,Reference,Native}` and `Sqlite::query_read_observed(request,bounds,execution) -> Result<QueryObservation>` behind `test-support`.
- [x] `QueryObservation` contains `read: QueryRead` and `metrics: QueryMetrics`; metrics contain selected `strategy: QueryStrategy`, `decoded_rows: usize`, `decoded_bytes: usize`, `vm_steps: u64`.
- [x] Share production reads with instrumentation. Physical VM counters describe materialization, not unmeasured metadata/planning work.
- [x] Native forcing bypasses cost comparison only; it retains the shared capability/binding gate and rejects or falls back for unsupported plans.
- [x] Test controls preserve bounds, errors, opaque policy mode and normal default behavior. Keep facades and cohesive modules.

### Executable measurements — ste_specs

Own `tools/query-measure`, root Cargo.toml/Cargo.lock membership and optional measurement-only dependencies. Coordinate adapter API above.

- [x] Add a modular binary with deterministic independent and skewed Resource fixtures, uniform policy and complete public Runtime queries.
- [x] Provide setup plus warm repeated reads for selective equality, common equality, no match, narrow/broad ranges, sorted page and conjunction-order cases at multiple sizes.
- [x] Compare automatic/reference/native on identical data and assert complete result equality before collecting samples. Rotate strategy order.
- [x] Emit JSONL samples: workload, size, repetition, mode, actual strategy, elapsed ns, rows decoded, bytes decoded, materialization VM steps and result count.
- [x] Add a separate optional DHAT run for allocation count, total allocated bytes and peak tracked heap; capture Linux RSS with clear scope.
- [x] Include write and space observations using the real maintained commit pipeline. Do not disable durability for a benchmark.
- [x] Add an executable smoke command and tests for fixture distributions and exact result equality. Do not run large trials before coordinator reviews the harness.

### Methodology review and primary sources — ste_guides

Own `docs/research/query-measurement-methodology.md`. Review only; no shared code edits.

- [x] Verify allocator scope, SQLite plan/counter semantics, statistical limits and fixture bias against primary sources.
- [x] Critique the planned measurements and implementation as it appears. Suggest bounded changes that improve the comparison.

### Coordinator

- [x] Review the harness, run release-mode measurements and retain raw output with source, compiler and lockfile identity.
- [x] Compare automatic selection with both real alternatives. If the selector loses on a repeatable workload, test a bounded improvement before tuning.
- [x] Measure indexed versus pre-index write/space overhead without modifying authoritative stores. State any remaining measurement gap explicitly.
- [x] Run independent review, affected tests and the full verifier. Publish a report and update only completed release tasks.
