# Native strategy selection options

Date: 2026-10-03.

This document preserves the proposal before implementation. The [maintained results](maintained-query-cost-results.md) record the later two-pass selector and acceptance evidence.

Status: proposal from source review. No production selector change or new performance result is recorded here.
The baseline is commit `9db4b2a`, with the current test-support observation API under development in the same worktree.

## Recommendation

First, measure automatic, reference and permitted native execution on the same workloads.
If skew causes repeated losses, test a bounded candidate-count probe before adding persistent statistics.
Use the current generic `query_keys` table. Do not add tables for individual Resources.

The proposed first experiment checks one leading predicate.
If a different conjunct is clearly selective, a later experiment can compare a bounded number of predicates.
Keep the current physical-plan guard, full-kind admission and shared semantic gate.
No private probe limit may become a limit on the logical query result.

This recommendation minimizes new state and migration work. It does not establish that a probe will improve latency.
For very selective reads, the extra statement can cost more than the work it saves.

## Current implementation and its risks

[The planner](../../crates/rom-sqlite/src/index/planner.rs) estimates one eighth of the rows for equality and one half for a range.
These fractions are heuristics. They are not measured selectivity or SQLite statistics.
The planner selects the first usable equality filter, then the first usable comparison.
The remaining conditions stay in the core evaluator.

This can cause the following costs:

| Workload | Current risk |
| --- | --- |
| Common equality value | The estimate can understate candidate count. Native execution can fetch almost every Row. |
| Range that covers most values | The estimated half-kind scan can become almost a full scan plus indexed Row lookups. |
| Common first conjunct, rare second conjunct | Equivalent predicate orders can cause different materialization costs. |
| Few large matching Rows | Average byte estimates can understate decoder work, even when row counts are accurate. |
| Small kind | Metadata, EXPLAIN and statement setup can exceed the saved decode cost. |

These are cost risks. The residual evaluator and exact admission remain responsible for the existing query semantics.
The current [observation API](../../crates/rom-sqlite/src/query_observation.rs) reports materialization work only.
Its VM count excludes metadata reads and EXPLAIN. A future probe requires separate counters or an explicit expansion of that scope.

## Compare the options

| Option | Implementation and storage cost | Useful information | Main limitation |
| --- | --- | --- | --- |
| Keep fixed fractions | No new code or writes | Baseline for comparison | Misses skew and conjunction order. |
| Capped covering-index probe | One extra statement per tested predicate; no persistent state | Exact small candidate count, or a lower bound | Adds read work and can penalize already selective queries. |
| Exact persistent frequency counts | Atomic counter updates, validation, backup and maintenance changes | Exact equality count | Distinct values increase state. Range estimates need more work. |
| Persistent histograms | Bucket maintenance or scheduled rebuilds, freshness tracking | Approximate range and equality selectivity | Approximation and stale statistics require a fallback. |
| SQLite statistics | ANALYZE maintenance and a statistics interpretation boundary | Statistics for SQLite's own planner | Does not directly measure ROM decoding, authorization or residual evaluation. |
| Bounded sampling | Sample selection, estimator and bias tests | Approximate selectivity | An ordered prefix is not a representative sample. |
| Combine compatible bounds on one field | Extend SQL generation; no persistent statistics | A smaller complete candidate set for bounded ranges | Does not resolve competing fields or broad equalities. |

SQLite can use a covering index without fetching the table payload. This makes a key-only probe plausible.
Equality prefixes and a following range also match the current physical key order.
These facts support the experiment, but do not measure its benefit. [SQLite optimizer overview](https://www.sqlite.org/optoverview.html#covering_indexes).

ANALYZE stores statistics for SQLite's planner. Approximate ANALYZE can limit analysis work, but is less informative than a full analysis.
Neither form directly supplies a stable ROM-level cost API. [SQLite ANALYZE](https://www.sqlite.org/lang_analyze.html).

## Proposed bounded probe

Build the probe from the same typed predicate representation as the materialization SQL.
Bind the Resource kind, field, encoded value and probe cap as parameters.
Only the operator comes from ROM's closed comparison enum.
Never interpolate a descriptor name into SQL syntax.

Example equality probe:

```sql
SELECT count(*)
FROM (
    SELECT 1
    FROM query_keys
    WHERE kind = ? AND field = ? AND encoded = ?
    LIMIT ?
);
```

For a range, preserve the current encoded presence floor:

```sql
SELECT count(*)
FROM (
    SELECT 1
    FROM query_keys
    WHERE kind = ? AND field = ? AND encoded < ? AND encoded >= ?
    LIMIT ?
);
```

The last range parameter is the existing present-value prefix, `[2]`.
Missing and null keys must not enter ordinary range candidates.
The range operator and all other encoding rules remain unchanged.

Let `B` be the maximum count that the experiment needs to distinguish.
Bind a checked `B + 1`, converted to SQLite's signed integer range.
If the probe returns at most `B`, that result is the exact candidate count in this snapshot.
If it returns `B + 1`, the candidate count exceeds `B`; its exact value remains unknown.
Do not record the lower bound as an exact count.

The inner LIMIT bounds the number of selected probe rows. An outer `count(*)` alone would not bound the scan.
SQLite's SELECT documentation defines this row limit. The execution plan still determines the physical work needed to produce those rows.
[SQLite SELECT](https://www.sqlite.org/lang_select.html#the_limit_clause).

Before the probe, require the recognized covering-index search for its exact SQL and bindings.
Retain the separate recognition of the final materialization plan.
EXPLAIN QUERY PLAN text is not a stable application interface, so unknown output must preserve the existing fallback.
[SQLite EXPLAIN QUERY PLAN](https://www.sqlite.org/eqp.html).

This is a row-work bound, not a hard elapsed-time or VM-instruction guarantee.
B-tree seeks, planning and storage stalls also cost work.
If a later requirement needs a hard execution budget, evaluate SQLite's progress handler separately.
It can interrupt work, but adds connection-scoped control and error handling.
[SQLite progress handler](https://www.sqlite.org/c3ref/progress_handler.html).

## Snapshot and semantic requirements

1. Perform exact full-kind admission before any probe. Include tombstones and both existing byte counters.
2. Use the same adapter-owned read transaction for counters, probe, plan inspection and materialization.
3. Bind every estimate to the request, store identity, generation and profile versions.
4. Run no application policy or codec callback while the native guard is held.
5. Select native only for the existing uniform read-and-field contract.
6. Materialize the full candidate superset after selection. Do not reuse a truncated probe as the result.
7. Release native guards before core decoding, residual evaluation and disclosure checks.
8. Propagate selected materialization failures. Preserve the existing optional-planning fallback rules.

A probe count can improve the cost estimate. It cannot prove logical result count, authorization or candidate completeness.
Other conjuncts, sorting, moving anchors and final limits remain in the shared evaluator.

## Include the probe in the decision

Compare complete estimated alternatives, not only Row fetch cost:

```text
reference_total = metadata + reference_materialization + common_evaluation
native_total    = metadata + plan_inspection + probe
                  + candidate_materialization + residual_evaluation
```

A crossover threshold depends on the measured workload and Row sizes.
Do not use the current fractional estimates as evidence for that threshold.
A bounded probe can distinguish narrow from broad predicates without finding an exact large count.
However, a broad-result fallback then pays for both the probe and the reference read.

For each probe, report its rows or lower bound, VM steps and elapsed time separately.
SQLite's VM-step counter counts instructions executed by that statement. It is not elapsed time or I/O cost.
[SQLite statement counters](https://www.sqlite.org/c3ref/c_stmtstatus_counter.html).

A scan-status estimate is another source of information, but requires SQLite's scan-status build support.
It would introduce a build dependency and another planner interpretation boundary.
Do not add it before the simpler baseline identifies a need.
[SQLite statement scan status](https://www.sqlite.org/c3ref/stmt_scanstatus.html).

## Alternative predicate choice

The first bounded experiment should retain one leading predicate, so its cost is easy to isolate.
If conjunction-order tests show a substantial loss, compare a small, explicit maximum number of eligible predicates.
Deduplicate identical predicate probes and enforce one total probe-work budget.

For an exact small count, compare measured native cost with the reference alternative.
For a saturated probe, retain only the lower bound.
If all candidates saturate the budget, use the reference path.
No probe may relax admission or authorize a different execution mode.

A later trial can combine lower and upper bounds for the same scalar field.
That change can reduce candidates without extra statistics, but needs exact null, missing and endpoint tests.
Keep all original predicates in the core evaluator even when the adapter combines compatible bounds.

## Why not sample first

The first matching index entries cannot estimate the total matching population.
A prefix of all ordered keys is also biased toward one end of the value distribution.
Randomized seeks and stratification would need separate assumptions, tests and cost accounting.
For this initial decision, a bounded exact-count probe has a smaller correctness and maintenance surface.
This is an engineering inference, not a result from a sampling experiment.

## Acceptance experiment

Use the existing maintained Runtime harness with unchanged Resources and policy definitions.
Include independent and skewed values, common and rare equalities, empty results, narrow and broad ranges, and both conjunction orders.
Include varying Row sizes and a validation dataset that was not used to choose weights.

Compare the existing automatic selector, the proposed probe selector, forced reference and permitted native execution.
Assert complete result equality before timing. Count probe work separately from materialization.
Measure p50 and tail latency across repeated runs, while reporting sample count and execution order.
Run heap profiling separately. Report write and space observations unchanged by this read-only proposal.

Adopt the probe only if repeatable gains justify its selective-query overhead and added maintenance.
Otherwise retain the simpler selector and record the unsupported optimization claim explicitly.
