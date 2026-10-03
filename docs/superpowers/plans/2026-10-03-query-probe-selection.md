# Bounded query probe selection implementation plan

> **For agentic workers:** Use `superpowers:executing-plans` and TDD for this assigned implementation. The coordinator owns integration and performance runs.

**Goal:** Choose a selective supported conjunct from bounded same-transaction index probes, and prefer reference reads for broad candidates.

**Architecture:** Probe the existing covering index without decoding Resource rows. Compare up to four distinct supported predicates in two passes. Keep shared admission, physical-plan recognition, binding, and residual evaluation unchanged.

**Tech stack:** Existing Rust and rusqlite dependencies; SQLite 3.53.2 physical-plan profile.

**Spec:** [Native selection options](../../research/native-selection-options.md), informed by the maintained Runtime calibration described below.

## Evidence and constraints

The coordinator reported these baseline medians from calibration seed 11:

- At 1024 skewed rows, common-first conjunction execution decoded 918 rows and took 2039 µs. Selective-first decoded 8 rows and took 114 µs.
- At 8192 skewed rows, common-first decoded 7376 rows and took 15403 µs. Selective-first decoded 8 rows and took 106 µs.
- At 8192 rows, the broad automatic case took 51111 µs, versus 45645 µs for reference execution.

Raw calibration is at `rom-dev:/var/tmp/rom-query-cost-20261003/calibration-v2.jsonl`.
These are coordinator-reported measurements, not independent reproductions by this implementer.
They justify testing a bounded selection change. They do not establish its performance before implementation and held-out measurement.

Preserve the existing Resource fixture, authorization, whole-kind admission, storage durability, index format, and result semantics.
Probe limits apply only to key counting. Never limit native candidate materialization with a probe limit.
Do not run builds while the coordinator's write measurements are active.
Observe failing tests before implementation. Do not commit; the coordinator owns integration.
Keep roots as facades, implementations cohesive, and predicate encoding shared by probe and materialization SQL.

## Bounded algorithm

Collect at most four distinct supported predicates from filters followed by comparisons.
Deduplicate equivalent encoded predicate plans before consuming a probe slot.
Queries with more eligible predicates retain this bounded first-four policy; later predicates remain residual conditions.

For each selected predicate, count up to `min(16, whole_kind_rows / 2) + 1` covering-index entries.
The count is exact only when fewer than the bound are returned. Otherwise it is a lower bound.
If any first-pass count is exact, select the smallest exact count. A saturated first-pass predicate cannot beat it.

If all first-pass probes saturate, expand each remaining probe to at most `min(whole_kind_rows / 2, 4096) + 1` entries.
After an exact count is available, reduce subsequent caps to at most `best_exact_count - 1`.
A zero exact count ends further probing.
The explicit worst-case budget is `4 * (17 + 4097) = 16456` visited keys per query.
This bounds returned probe keys, not elapsed time or all SQLite instructions.

Use `SELECT 1 FROM query_keys ... LIMIT ?` and count returned entries without Resource decoding.
The probe and materialization statements share encoded parameters and presence-floor logic.
Require the known covering-index plan for a probe before executing it.
Require the existing materialization plan before selecting native execution.
Unknown plans or optional probe failures retain conservative fallback behavior.

If every count saturates, retain a recognized materialization plan with conservative whole-kind cost.
This lets the test-support forced-native mode run the full native alternative without treating a broad estimate as a missing plan.

## Cost model and observation

Use reference charges of 100 per row and 1 per canonical byte.
Use native charges of 200 per candidate row and 2 per estimated candidate byte.
Estimate candidate bytes from the whole-kind average, with checked arithmetic and rounding upward.
For a saturated candidate, charge the full kind rather than treating a lower bound as an exact count.
Keep reference startup at 16 and native startup at 128.
Add the same charge for executed probe attempts and visited keys to both alternatives, because this work is already incurred.
Charge 64 units per attempted probe statement and 1 unit per visited key.
These units are conservative comparison weights, not measured nanoseconds. Held-out evaluation decides whether the change is acceptable.

Keep materialization counters unchanged. Add separate test-support counters for probe statements, visited keys, VM steps, and execution nanoseconds.
The fields are `probe_statements`, `probe_rows`, `probe_vm_steps`, and `probe_elapsed_ns`.
Probe elapsed time excludes EXPLAIN; the existing adapter elapsed timer includes all planning and probe work.
Ordinary production queries must not read SQLite status counters or a measurement clock.

## File boundaries and interfaces

- `crates/rom-sqlite/src/index/planner.rs`: selection orchestration and `NativePlan` export to the query executor.
- `crates/rom-sqlite/src/index/planner/predicate.rs`: shared typed predicate encoding and SQL construction.
- `crates/rom-sqlite/src/index/planner/probe.rs`: capped probes, physical-plan recognition, and probe work accounting.
- `crates/rom-sqlite/src/index/planner/cost.rs`: checked cost construction.
- `crates/rom-sqlite/src/index/planner/tests.rs`: real SQLite selection and budget regressions.
- `crates/rom-sqlite/src/index/query.rs`: adapt the selection seam; preserve forced-cost override and execution gates.
- `crates/rom-sqlite/src/query_observation.rs`: separate probe observation fields.
- `tools/query-measure/src/run.rs`: emit the new probe counters without changing fixture or equality checks.

The selection seam returns a selected plan plus estimates, or no permitted plan, together with accumulated probe metrics.
The query executor passes the existing request binding and admitted whole-kind row/byte counters.
Probe accounting remains available when automatic selection chooses reference execution.

## Task 1: Selection and bounded work

- [x] Add real SQLite regressions for broad skew choosing reference, both conjunction orders choosing the 8-row candidate, and full candidate materialization under forced native.
- [x] Add exact-versus-saturated boundary tests, medium selectivity above 128 matches, duplicate predicates, more than four predicates, and unknown-plan fallback.
- [x] Run the focused tests after the coordinator releases the build pause. Record the intended RED result.
- [x] Implement the shared predicate representation, bounded two-pass probe, conservative cost model, and query seam.
- [x] Run focused planner and existing native query tests. Preserve encoding/absence tests independently of changed automatic cost decisions.

## Task 2: Observable probe cost and parity

- [x] Add tests for separate probe work counters, zero probe work under forced reference, and ordinary/observed automatic result parity.
- [x] Record RED for missing instrumentation, then implement counter collection only in observed calls.
- [x] Emit probe counters in measurement JSONL and state their scopes.
- [x] Run default-feature and test-support SQLite tests, harness smoke tests, formatting, and Clippy.

## Implementation verification

The first behavior run had four intended failures and three passing controls.
Common-first and duplicate-predicate queries returned 921 candidates instead of 8.
The broad-result and bounded-prefix tests also failed because the previous selector chose native execution.
The separate instrumentation RED run reported missing probe fields.

After implementation, the SQLite test-support run passed 29 unit tests and two public observation tests.
The no-default-feature run passed 23 unit tests. The measurement harness passed five tests, including complete result equality.
The tests cover an exact first-pass count of 16, a saturated first pass followed by an exact count of 17, and the 16,456-key budget.
They also verify the reduced 1,058-key second-pass workload after an exact 512-row candidate is known.

Review added an empty-candidate early-stop regression, which failed with two probes before the fix and passed with one probe afterward.
Reference-only selection and unsupported semantics now skip probes, while the full shared selector remains the final native eligibility gate.
Public observation tests reject a whole-kind byte bound of 1 in every mode, even when the logical limit is 1.

Formatting and Clippy passed for all targets with all features, and for SQLite with no default features.
The final Clippy run followed equivalent iterator and conditional cleanup.
Its log is `/var/tmp/rom-maintained-query-probe-20261003-clippy.log` on the host.
The implementation was then frozen for the coordinator's release build and held-out measurements.
No commits or large performance trials were performed by this implementer.

An independent source review found no remaining blocker: [probe selection review](../../research/query-probe-selection-review.md).
The coordinator still owns the final integrated verifier and performance acceptance.

## Review focus and acceptance

The tests must distinguish exact probe counts from saturated lower bounds.
They must catch a private probe limit leaking into materialization, duplicate predicates consuming slots, or a broad first predicate hiding a selective later predicate.
They must retain missing/null range behavior and conservative fallback when the recognized physical plan is unavailable.
Ordinary reads must retain their authority, admission, binding, and error order.

The coordinator runs the full verifier, independent review, and release-mode held-out seed 203 trials.
Compare narrow-query overhead, broad-result fallback cost, both conjunction orders, and separate probe work.
Do not mark the release or selector improvement complete from implementation tests alone.

## Coordinator acceptance

The seed-203 validation, baseline comparison and separate heap trial are complete.
The [maintained results](../../research/maintained-query-cost-results.md) include improvements, selective-query overhead and all measurement limits.
Full local verification and the reference demo passed after the final harness input guard.
Independent specification and quality reviews found no unresolved blocker.
Release tasks 3.3 and 3.4 are complete; stage-four operations and release work remain open.
