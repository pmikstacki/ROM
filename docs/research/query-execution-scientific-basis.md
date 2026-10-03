# Three execution strategies with one Resource contract

Research date: 2026-10-03. Owner requested three functional approaches, scientific
grounding and a case-by-case comparison. This note supplies hypotheses and an
evaluation contract, not benchmark results.

## Evidence and its limits

**Volcano (Graefe and McKenna, ICDE 1993)** separates logical query meaning from
physical execution plans and uses equivalence rules and costs to choose an
implementation. Its generated optimizer still plans queries at runtime. For
ROM, the useful distinction is a canonical query contract versus an adapter's
execution strategy; generated authoring helpers do not settle the latter.
This is foundational work, not a new 2026 library recommendation.
[Original paper](https://15721.courses.cs.cmu.edu/spring2023/papers/16-optimizer1/graefe-icde1993.pdf)

**Cascades (Graefe, IEEE Data Engineering Bulletin 1995)** makes physical
properties and costs explicit and explores equivalent expressions. Its examples
include required ordering. ROM can adopt the small principle that an optimized
path must meet the required order, rather than copying a full optimizer or
assuming all plans are interchangeable merely because they filter the same rows.
[Original paper](https://15799.courses.cs.cmu.edu/spring2025/papers/05-cascades/graefe-ieee1995.pdf)

**DBSP (Budiu et al., PVLDB 2023)** formalizes incremental computation of query
results from database changes, with composition rules. The paper also notes that
incremental plans retain state and cannot be switched as casually as ad-hoc plans.
For ROM this motivates a later live-view experiment comparing recomputation with
maintained deltas. It does not show that incrementalizing arbitrary Rust policies
or adopting a new engine is necessary for today's filter/sort slice.
[Original paper](https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf)

The current literature scan also found **Enzyme (Yadav et al., 2026)** on
incremental maintenance for data engineering. Only its abstract and publication
metadata were inspected here. It is a follow-up reading candidate, not evidence
for ROM performance or a tested dependency.
[Author preprint, revised April 2026](https://arxiv.org/abs/2603.27775)

These papers establish useful distinctions. They do not prove that ROM preserves
authorization, exact codecs or commit/event invariants; local tests must do that.

## Existing optimizers rather than starting from scratch

SQLite already estimates competing plans using indexes and statistics. ROM's
SQLite adapter should provide exact, optimizable SQL and let that engine choose
its local access path. Its planner does not, however, model the Rust codec,
residual authorization or the cost of fetching data into ROM.
[SQLite optimizer overview](https://www.sqlite.org/optoverview.html)

Apache DataFusion provides a Rust query engine and optimizer with logical and
physical optimization passes. It is a candidate for a later integration trial,
especially broader analytical queries, rather than an adopted dependency here.
Mapping ROM's exact scalar/presence/policy rules, compile footprint and startup
cost requires measurement before replacing the small evaluator.
[Official optimizer guide](https://datafusion.apache.org/library-user-guide/query-optimizer.html)

`egg` supplies equality-saturation machinery and extraction according to a cost
function. It does not supply ROM's rewrite laws, statistics or cost model. A
trial would need to demonstrate that this machinery improves selection beyond a
small explicit strategy table, without changing semantics or unbounded planning.
[Official tutorial](https://docs.rs/egg/latest/egg/tutorials/_01_background/index.html)

Neither DataFusion nor egg was compiled or benchmarked in this experiment. The
current recommendation is to reuse the database planner locally and keep ROM's
cross-boundary selection small until measured workloads justify another engine.

## Functional variants

All variants accept the same normalized query, dataset and policy, and compare
with independently specified expected results. A Resource mutation still uses
the authoritative commit path; query acceleration does not own mutation semantics.

| Variant | Execution | Hypothesis to test | Cost that must be included |
| --- | --- | --- | --- |
| A: core evaluator | Read bounded candidates, evaluate predicates/current policy, sort and limit in core | A small, portable reference path may be sufficient for small collections | All read/decoded rows, bytes and sorting |
| B: exact pushdown | Adapter applies exact predicates without a field index; core applies residual policy before final limit | Reduced decoding can help even while the database scans | Database VM work as well as returned candidates; residual-policy refill |
| C: indexed execution | The same predicates use canonical indexed keys with exact order support where available | Selective/read-heavy cases can reduce database work materially | Index construction/storage, mutation maintenance, and cases whose ordering still needs sorting |

Use real SQLite for all three physical variants and a redb/reference path for
cross-model semantic checks. A plan using a filter index plus a temporary sort
must be called that; it is not automatically an indexed keyset seek. Do not force
the same execution algorithm onto redb merely to make the table symmetric.

Separately compare **generated**, **hybrid typed** and **runtime descriptor**
authoring against the same normalized representation and executor. These are
three ways of constructing a query, not three different database algorithms.

## Shared acceptance cases

- Exact membership/order for small, selective, broad, empty and mostly-denied
  results; authorization runs before the client result limit.
- Exact unsigned extrema, missing versus null, canonical custom values, Unicode
  order and duplicate sort values with Resource ID as the final tie-breaker.
- Moving continuation after insert/delete/update, with documented repeats or
  omissions and no claim of a retained snapshot.
- Common rejection of malformed input and bounded-work exhaustion; an optimized
  implementation cannot silently truncate or approximate the answer.
- Mutation/query/reopen checks with the same committed Resource and event history
  where the harness integrates ROM. If it only uses synthetic rows, label this
  case untested rather than infer commit correctness from query equivalence.

Negative controls deliberately apply limit before row authorization, collapse
missing/null or convert exact u64 through floating point. Each should violate an
expected-result assertion. This checks that the suite can detect the relevant
mistake rather than merely exercising code.

## Choosing an approach

Correctness is an admission gate. Among passing variants, report repeated latency
distributions, construction/normalization time, candidate/decoded counts, database
VM work, build/code size and index/write cost separately. Publish the dataset,
versions, commands and raw samples. Avoid a universal winner from one workload:
read selectivity, collection size and write rate can change the useful strategy.

The intended outcome is one pleasant Resource/query API and internal selection of
an exact execution path, with a portable bounded evaluator as fallback. Required
persistence semantics remain mandatory for every adapter. A measured index win
does not authorize weakening idempotency, authorization or committed events.
