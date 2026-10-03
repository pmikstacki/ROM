# Small strategy selector with database-owned physical plans

This standalone extension builds on the frozen three-strategy query experiment.
It adds a generic `PlannerAdapter` and a pure `choose` function; no DataFusion or
egg dependency is introduced. SQLite remains responsible for its SQL access path.
The selector chooses only Core versus Native and never emits `INDEXED BY`.

SQLite's adapter inspects the exact LIMIT-bearing query with EXPLAIN, recognizes
a strict set of nodes for pinned SQLite 3.53.2, and estimates scan/decode/sort work.
Unknown nodes or versions return no estimate. redb's adapter supplies the same
core path with no native query-planner estimate. Missing, stale, mismatched,
non-exact or overflowing estimates cannot authorize a different semantic query.
Execution errors propagate; optional planning errors can use the reference path.

Statistics are collected at fixture creation. Uniform amount distribution is a
heuristic, not a fact inferred by the selector. Fixed cost weights are relative
units chosen using the earlier workload observations, not calibrated nanoseconds.
This is an in-sample test, not proof of learned or generally optimal planning.
Wrapped SQLite mutation advances a session-local generation. Statistics must be
discarded on reopen; external writers, persisted cost caches, multiwriter fencing
and schema/index mutation are not implemented by this fixture.

## Uniform admission and correctness

A reviewer reproduced a defect in the first draft: fresh indexed estimates could
return a selective result within a ten-candidate bound, while stale estimates
selected a core snapshot that exceeded the same bound. The corrected selector
checks actual collection row count before planning, preserving the existing MVP
bounded-kind profile for every strategy. It does not use estimated row count for
admission. This permits exact differential testing of both results and capacity
errors. A future streaming/candidate-budget profile needs its own common contract.

This admission count is extra work. Its latency is included in selected timings;
`vm_steps` and candidate counts are **executor-only** and exclude COUNT/EXPLAIN.
The benchmark has no byte, wall-clock or total VM-work cancellation contract.
Production admission should reuse atomically maintained metadata where justified,
rather than silently weakening the common limit to win a benchmark.

Seven selector tests pass, in addition to eight inherited conformance tests.
An independent reviewer ran 168 additional query comparisons and fault adapters:
authorization precedes adapter access, planning failure falls back, execution
failure does not silently retry, and actual mutation invalidates old estimates.
Synthetic boolean row visibility and one sort grant are not maintained ROM auth.
The transactional journal is this fixture's journal, not ROM's receipt pipeline.

## Reproduction

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-selector-target CARGO_BUILD_JOBS=2
cargo test --locked --manifest-path prototypes/query-planning/Cargo.toml
cargo clippy --locked --manifest-path prototypes/query-planning/Cargo.toml --all-targets -- -D warnings
cargo run --release --locked --manifest-path prototypes/query-planning/Cargo.toml --bin selector > prototypes/query-planning/results/selector.csv
```

The added binary means the original query benchmark must now specify
`--bin rom-query-planning-prototype` explicitly. The original prototype branch is
unchanged. `selector.csv` contains 594 observations: 3 sizes × 2 index profiles ×
3 cases × 11 samples × 3 modes. Every result is checked against the oracle outside
the timer; execution order rotates. Core/native baselines skip outer normalization,
authorization, admission and planner inspection; the selected mode includes them.
The difference is full selector-path overhead, not pure optimizer CPU time.

By median execution time the selector chose the faster raw path in all 18 fixture
cases. It is still slower than directly using that known best path: on 100k rows,
selected/raw native was 96.4/73.3 µs for indexed selective and 62.1/35.2 µs for
indexed no-match. At 1k indexed no-match it was 15.3/6.9 µs. For unindexed broad
100k rows it selected core at 30.70 ms instead of native 52.74 ms. These values
show both the benefit and the setup overhead; they are not portable forecasts.

Host load, caches and frequency are uncontrolled. Small sub-one regret ratios
between independent sample medians reflect noise, not an impossible speedup over
the executor the selected path invokes. No allocation/RSS data, realistic query
distribution, plan cache, write-heavy adaptation or production implementation
cost is measured. Next evaluate bounded generation-keyed plan reuse and skewed
holdout workloads before promotion.
