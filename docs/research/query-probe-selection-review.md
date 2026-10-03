# Bounded query probe selection review

Date: 2026-10-03. Scope: independent source review of the [implementation plan](../superpowers/plans/2026-10-03-query-probe-selection.md) and maintained SQLite planner. No builds or benchmarks were run for this review.

The [calibration report](maintained-query-cost-results.md) motivates evaluating a different selector. Its results describe the earlier selector. This review makes no performance claim for the bounded-probe implementation.

## Completeness and admission

Each supported predicate is a conjunct of the normalized request. Materializing all rows that satisfy any one such predicate gives a complete superset for core residual evaluation. Selecting a different conjunct changes the number of candidates, not which logical matches remain possible. The shared predicate representation supplies encoded parameters and presence-floor conditions to both probe and materialization SQL. This avoids two independent predicate implementations.

The probe's SQL limit applies only to counting covering-index entries. The materialization statement contains no limit, and core evaluation still owns all predicates, ordering, anchors and result limits. The forced-native broad-query regression requests a logical limit of one while expecting every native candidate, specifically testing this separation.

The query executor must finish whole-kind row, stored-byte and canonical-byte admission before invoking the planner. It must hold the same read transaction for metadata, probes and materialization. A selective probe cannot make an over-bound kind admissible. Final selection must continue through the shared request, snapshot, semantics and capability gate, including forced-native execution.

## Two-pass proof and work bounds

For a probe cap C, the SQL limit is C+1. Returning at most C entries after exhausting the cursor proves that count exact. Returning C+1 entries proves only a lower bound. Treating the latter as an exact estimate would incorrectly favor saturated predicates.

All first-pass probes use the same cap. If one returns an exact count, every saturated predicate has a strictly larger count. Selecting the smallest exact first-pass count is therefore sufficient; expanding saturated alternatives cannot improve it.

When every first-pass probe saturates, the second pass can discover a larger exact count. Once the best exact count is B, a later probe only needs to distinguish fewer than B matches from at least B. A cap of B−1, with SQL limit B, does this. If that cap is no larger than the already saturated first-pass cap, repeating the probe cannot improve the selected count and can be skipped. Ties do not require replacement.

At most four distinct encoded predicates consume slots. Unsupported predicates and duplicates do not consume additional slots. Later distinct predicates remain residual conditions. The two caps give at most 4 × (17 + 4097) = 16,456 returned probe keys and eight executed probe statements. The budget includes repeated keys between passes. It does not bound elapsed time, all SQLite instructions, internal B-tree work or EXPLAIN preparation.

For all-saturated candidates, a recognized complete materialization plan remains available, with the whole-kind count charged conservatively. This supports forced-native comparisons without pretending that saturation establishes exact cardinality. The 4096 expansion cap can miss useful candidates in larger kinds; that is an intentional bounded-work tradeoff, not a completeness loss.

## Cost and instrumentation

The model charges reference rows/bytes at 100/1 and native candidates/estimated bytes at 200/2, with startup charges 16 and 128. Candidate bytes use an upward-rounded whole-kind average. Checked integer arithmetic falls back when the estimate cannot be represented. These units are heuristics, not nanoseconds or a universal native/reference cost ratio.

Probe statements and returned keys are already incurred before final selection. Adding the same probe charge to both alternatives preserves their relative comparison. Charging only native would incorrectly make the chosen reference alternative appear to avoid work that already happened. Actual total adapter timing must still include probing when reference wins.

Probe counters must remain separate from materialization counters. Query-wide observed adapter timing includes EXPLAIN and planner work; the narrower probe execution timer excludes EXPLAIN. Ordinary production calls must not read an observation clock or SQLite status counters. A failed optional probe must not claim zero actual cost merely because no usable estimate was returned.

## Findings and final source review

The first source inspection found sound cap/exactness logic, shared predicate encoding and unbounded materialization SQL. The plan retains conservative fallback for unrecognized physical plans or optional probe failures. Execution errors after native selection must still propagate without a second reference read.

These findings were sent to the implementation owner while integration was in progress:

- Stop first-pass probes immediately after an exact zero count, as the plan requires. The initial implementation still probed later predicates.
- Avoid optional probes when the request already requires reference execution, while retaining the final shared eligibility gate. Add an observed opaque-request test with zero probe work.
- Failed probe attempts initially retained row/statement counts but omitted their elapsed time and VM counters. Account for partial work where possible, or label the counter limitation explicitly.

The final source inspection confirmed all three corrections. An exact zero stops the first-pass loop. The executor skips the planner for reference-only or unsupported-semantics requests without copying the full core eligibility algorithm. Probe cursor errors return through a scope that still samples elapsed time and VM work before discarding the optional count. Preparation and EXPLAIN failures remain outside those execution-only counters, as documented.

The completed [query executor](../../crates/rom-sqlite/src/index/query.rs) performs whole-kind admission before selection, carries accumulated probe metrics into reference responses, and passes forced-native estimates through the unchanged shared selector. Native materialization errors propagate directly. The caller's transaction encloses metadata, probes, plan recognition and final reads. The probe limit never enters candidate SQL.

The [probe implementation](../../crates/rom-sqlite/src/index/planner/probe.rs) enforces its remaining key budget before each statement. Both regular and observed automatic execution use the same count and selection logic. Only observed test-support calls read a clock or VM status. The updated harness emits separate probe rows, statements and VM steps; latency records additionally include probe elapsed time.

The planner split follows the repository's cohesion and DRY rules: orchestration, predicate encoding, physical probes and checked costs have separate responsibilities. Predicate encoding is shared between probe and materialization; the shared core selector remains the final eligibility authority. No implementation was added to a Rust module-root facade.

The owner reports 29 SQLite test-support unit tests and two public tests passing after observing the zero-count regression fail first. This independent review inspected the final source and tests without running builds. The tests include exact 16 versus saturated 17, early zero, opaque/forced-reference zero probe work, second-pass shrinkage, duplicate predicates, the first-four prefix, missing physical index fallback, medium selectivity and the full 16,456-key budget. They also check that a saturated forced-native query with a logical limit still materializes the full candidate set.

No remaining correctness blocker was found. This conclusion depends on the existing startup and transactional index-integrity guarantees; an estimate is not a fresh proof that an externally corrupted index contains every row. Optional future fault-injection coverage could exercise a cursor failure after partial probe progress and verify that fallback observations retain that work. Performance acceptance remains pending held-out full-pipeline measurements, especially probe overhead for narrow queries and conservative fallback on large or skewed kinds.
