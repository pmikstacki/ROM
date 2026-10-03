# Optimizer adoption: implementation cost versus measured benefit

Date: 2026-10-03. This assessment combines inspected APIs and the current query
experiment. Relative implementation costs are engineering estimates, not measured
person-days. DataFusion and egg have not been built or benchmarked for ROM.

## Recommendation

Use the existing database planner for local physical execution, retain one exact
query representation/evaluator, and add a small internal strategy selector only
when passing alternatives exist. Prioritize canonical indexes for measured hot
queries. Do not build a general optimizer or add an analytical engine solely to
choose between a scan and an indexed filter.

This recommendation preserves the owner's contract: application authors declare
Resources and queries once. Every conforming implementation returns the same
meaning; optimization does not reinterpret fields, authorization or events.

| Option | Work ROM still owns | Relative adoption cost | Benefit supported now |
| --- | --- | --- | --- |
| Existing SQLite planner, exact bound SQL and relevant indexes | Canonical scalar/presence representation; index declaration, atomic maintenance and rebuild; auth before final limit | Lowest incremental dependency cost; index lifecycle is still real implementation work | Standalone SQLite experiment shows indexed and unindexed plans with identical results and different measured VM work/latency |
| Small ROM strategy selector | Validated alternatives, coarse statistics/cost hints, bounded fallback, explainable choice | Low-to-medium once alternatives exist; no new query-engine dependency required | Can avoid unconditional pushdown; automatic selection itself has not yet been benchmarked |
| DataFusion | Mapping ROM values into Arrow/expressions, provider and execution boundary, exact missing/null/custom codecs, policy placement, memory/lifecycle integration | Higher than retaining current drivers/evaluator; modular crates may reduce scope | Rich optimizer exists; no demonstrated ROM speedup or integration-cost measurement |
| egg | ROM expression language, valid rewrite rules, cost/statistics model, extraction limits and executor | High semantic/design burden for the current narrow query profile | General equivalence/extraction machinery exists; no demonstrated ROM speedup |

SQLite estimates local CPU/I/O costs among plans and uses available statistics.
It cannot account for arbitrary native ROM authorization or codec work performed
after rows leave the database. This motivates a small outer choice, not a second
SQL optimizer inside core.
[SQLite optimizer](https://www.sqlite.org/optoverview.html)

DataFusion's documented integration uses table providers/execution plans and Arrow
data; its optimizer can be reused modularly. Its ready-made rules do not remove
the need to map ROM's domain and security semantics correctly.
[Provider integration](https://datafusion.apache.org/library-user-guide/custom-table-providers.html),
[Optimizer guide](https://datafusion.apache.org/library-user-guide/query-optimizer.html)

egg extracts an expression according to a supplied cost function. The framework
does not infer our valid rewrites or measure adapter costs automatically.
[Extractor API](https://docs.rs/egg/latest/egg/struct.Extractor.html)

## Smallest useful adoption sequence

1. Freeze shared scalar filters, deterministic ordering and moving continuation
   semantics. Use the same normalization and policy path for typed/wire/live API.
2. Keep the bounded evaluator as the reference implementation for all adapters.
3. Promote a measured indexed strategy only with atomic index maintenance,
   version/rebuild behavior and differential tests against that evaluator.
4. Add conservative internal selection using demonstrated properties. Prefer a
   valid fallback when estimates are absent; do not pretend stale statistics prove
   an optimal plan. Report chosen path and work in diagnostics.
5. Revisit DataFusion when joins/aggregations/reporting workloads exist. Revisit
   egg only if rewrite-space complexity exceeds simple explicit planning rules.

## Evidence gates and limitations

The three physical prototypes share normalized queries; the authoring comparison
is separate. Their final report must include fairness corrections from review,
raw repeated samples and index/write overhead before numerical rankings are
treated as final. An in-memory SQLite benchmark is not a production disk-I/O
forecast; a synthetic journal is not the maintained ROM commit pipeline.

No line-count estimate establishes development time. No fixed speedup is promised
for small collections, broad queries, frequent writes or different policies. The
adoption decision should be revisited when a real application workload disagrees
with this experiment.
