# Maintained query cost results

Date: 2026-10-03. Status: measurements, selector refinement, independent review and full local verification complete.

## Scope

These trials use the maintained Runtime, Resource codecs, authorization path and SQLite adapter.
They measure current code, not the earlier query prototype.
The baseline selector uses the fixed selectivity fractions introduced with maintained indexes in commit `9db4b2a`.

The [measurement harness](../../tools/query-measure/README.md) defines the workload and controls.
The [methodology review](query-measurement-methodology.md) explains its measurement limits.
The [source archive](evidence/query-cost-2026-10-03/calibration-source.tar.gz) preserves the calibration workspace sources.
The [archive checksum](evidence/query-cost-2026-10-03/calibration-source-archive.sha256) identifies that snapshot.

## Calibration evidence

The completed run contains 1,728 observed query samples and 576 ordinary production query samples.
All results matched the complete ordered reference views, including keys, revisions and projected values.
No result comparison is inside a measured interval.

The matrix has two distributions, three sizes, eight query cases, three observed modes and twelve repetitions.
The modes use all six permutations, with two warmups before samples.
Sizes are 128, 1,024 and 8,192 Resources. The calibration seed is 11.
Each Resource has four scalar fields and an unindexed list payload.
Amount equals the Resource ID rank; other fixture domains use separate mixed seeds.

- [Raw calibration records](evidence/query-cost-2026-10-03/calibration-v2.jsonl)
- [Calculated sample summaries](evidence/query-cost-2026-10-03/calibration-summary.json)
- [Build log](evidence/query-cost-2026-10-03/current-build-v2.log)
- [Compiler and host](evidence/query-cost-2026-10-03/environment.txt)

The host is shared. These are warm local observations, not isolated hardware performance guarantees.
Twelve samples do not establish a stable tail distribution. Their nearest-rank p95 equals their maximum.
The production control runs after the observed modes; use it as an instrumentation check, not an exact causal overhead estimate.

### Results at 8,192 Resources

Times below are medians in milliseconds for the full Runtime query.
The interval includes actor construction, query cloning and final disclosure. It excludes result comparison, formatting and destruction.

| Distribution | Case | Reference ms | Automatic ms | Forced native ms | Automatic decoded rows |
| --- | --- | ---: | ---: | ---: | ---: |
| independent | selective_eq | 11.192 | 0.057 | 0.075 | 1 |
| independent | common_eq | 13.360 | 3.885 | 3.945 | 543 |
| independent | broad_range | 45.950 | 51.615 | 51.850 | 8192 |
| independent | sorted_page | 14.863 | 20.608 | 20.873 | 8192 |
| independent | conjunction_common_first | 10.883 | 1.555 | 1.591 | 543 |
| independent | conjunction_selective_first | 10.907 | 0.077 | 0.095 | 8 |
| skewed | selective_eq | 11.624 | 0.082 | 0.092 | 1 |
| skewed | common_eq | 42.736 | 46.916 | 46.514 | 7376 |
| skewed | broad_range | 45.645 | 51.111 | 51.455 | 8192 |
| skewed | sorted_page | 14.902 | 20.658 | 20.548 | 8192 |
| skewed | conjunction_common_first | 11.149 | 15.403 | 15.385 | 7376 |
| skewed | conjunction_selective_first | 10.954 | 0.106 | 0.122 | 8 |

The selective cases support native execution. Broad reads show the cost of unnecessary index lookups and Row decoding.
On skewed data, predicate order changes candidate count from 8 to 7,376 for equivalent conjunctions.
The fixed fractions neither detect this skew nor select the most useful predicate.
A bounded covering-index probe is therefore justified for a follow-up experiment.
Its implementation must preserve complete candidates, current authorization and whole-kind admission limits.

## Write comparison

The valid comparison uses separate Cargo target directories for the current and pre-index builds.
The baseline source is commit `bedff7a`; the fixture, write workload, lockfile and workspace dependencies come from the current harness.
Both binaries use release mode without default features or the heap allocator.
Both use WAL, synchronous FULL and a 32-event journal bound. Durability is not disabled.
Four fresh processes per size and implementation alternate the execution order.

The first attempted comparison is invalid. A shared Cargo output directory left the historical binary at both copied paths.
Equal binary hashes and zero index pages in both outputs revealed the problem.
No timing or space result from that attempt is used here.
The runner now rejects equal binaries and verifies index presence with a separate eight-Resource preflight.

The valid trial completed all 24 fresh-process workloads and 72 measured write phases.
Each update phase changes 64 Resources. The seed phase creates the indicated number of Resources.

| Resources | Phase | Baseline median ms | Indexed median ms | Kernel write bytes ratio |
| ---: | --- | ---: | ---: | ---: |
| 128 | seed | 517.98 | 528.26 | 1.434 |
| 128 | indexed_update | 265.50 | 280.24 | 1.316 |
| 128 | payload_update | 263.52 | 261.80 | 1.079 |
| 1024 | seed | 5350.53 | 5791.27 | 1.561 |
| 1024 | indexed_update | 292.79 | 387.03 | 1.322 |
| 1024 | payload_update | 280.84 | 343.87 | 1.078 |
| 8192 | seed | 40513.90 | 49312.56 | 1.656 |
| 8192 | indexed_update | 313.74 | 339.07 | 1.302 |
| 8192 | payload_update | 318.83 | 362.93 | 1.076 |

At 8,192 Resources, the closed baseline database uses 13,398,016 bytes.
The indexed database uses 17,395,712 bytes, or 1.298 times that space.
The seed phase writes about 1.656 times the kernel-accounted bytes and takes about 1.217 times the median elapsed time.
These costs apply to four indexed scalar fields and the stated retention policy.
They do not establish a universal write penalty or physical SSD write amplification.
Timing varies between processes on this shared host; retain the individual samples, not only medians.

[Raw write records and per-process summaries](evidence/query-cost-2026-10-03/writes/summary.json) retain all four repetitions.
[Binary hashes](evidence/query-cost-2026-10-03/writes/binaries.sha256) and [execution order](evidence/query-cost-2026-10-03/writes/progress.log) identify the valid run.


## Baseline allocation measurements

A separate DHAT release build completed 48 query scopes at 1,024 Resources.
Each scope has one sample, with the result retained until counters are captured.
The Rust allocator sees allocations begun in that interval across process threads.
It excludes pre-existing allocations and SQLite C allocations. Peak tracked bytes are not total process memory.
The table below uses the skewed distribution, seed 11, with 1,024 Resources.

| Case | Mode | Allocations | Total allocated bytes | Peak tracked bytes |
| --- | --- | ---: | ---: | ---: |
| selective_eq | reference | 15590 | 2668482 | 1400074 |
| selective_eq | automatic | 272 | 24043 | 6511 |
| common_eq | reference | 136656 | 13635060 | 2091549 |
| common_eq | automatic | 137855 | 14226860 | 2091549 |
| conjunction_common_first | reference | 16394 | 2738116 | 1400307 |
| conjunction_common_first | automatic | 17595 | 3330056 | 1276982 |
| conjunction_selective_first | reference | 16394 | 2738116 | 1400307 |
| conjunction_selective_first | automatic | 1209 | 115834 | 18203 |

[Raw heap records](evidence/query-cost-2026-10-03/heap-calibration.jsonl) and [summaries](evidence/query-cost-2026-10-03/heap-calibration-summary.json) preserve the scopes.
These records contain no latency claim from the instrumented allocator.
The allocation results support reducing candidate materialization; they do not prove a universal memory bound.

## Bounded selector and held-out results

The accepted selector probes at most four distinct supported predicates in the admitted read transaction.
Two passes visit at most 16,456 index keys. This bound does not limit elapsed time or all SQLite instructions.
Exact counts identify selective candidates. Saturated counts retain conservative whole-kind costs.
Probe limits never apply to candidate materialization. Core authorization, residual filters, ordering and disclosure remain unchanged.

The validation seed is 203. Each implementation completed 1,728 observed queries and 576 ordinary production controls.
This validation changes the seed, not the query shapes, sizes or distribution families.
Every trial compared complete ordered views with its reference result outside the timer.
All 48 workload keys also have matching result counts and digests across the two implementations.
The cross-run digest comparison supplements the complete comparisons within each run; it does not replace them.

The table uses skewed data with 8,192 Resources. Times are full Runtime medians in microseconds.

| Case | Previous automatic µs | New automatic µs | Previous decoded rows | New decoded rows | New probe keys |
| --- | ---: | ---: | ---: | ---: | ---: |
| selective_eq | 76.797 | 100.097 | 1 | 1 | 1 |
| common_eq | 46647.615 | 43857.937 | 7361 | 8192 | 4114 |
| missing_eq | 52.611 | 58.076 | 0 | 0 | 0 |
| narrow_range | 113.397 | 123.657 | 8 | 8 | 8 |
| broad_range | 51530.676 | 46151.900 | 8192 | 8192 | 4114 |
| sorted_page | 21056.069 | 15561.629 | 8192 | 8192 | 4114 |
| conjunction_selective_first | 118.928 | 131.707 | 8 | 8 | 25 |
| conjunction_common_first | 15326.430 | 138.255 | 7361 | 8 | 25 |

Both conjunction orders now select eight candidates in this workload.
Broad cases select reference execution and avoid native row lookups, although the probes still add work.
For independent data at the same size, the common predicate matches 547 rows and retains native execution.
Its automatic median is 4,139.965 µs, against 13,508.561 µs for reference execution.

The change has costs. The selective equality case increases from 76.797 to 100.097 µs in the separate validation runs.
Within the new run, the worst automatic-to-best-alternative median ratio is 1.202.
That case is the sorted page on 128 skewed Resources: 400.353 µs automatic versus 333.134 µs reference.
These observations do not establish causal precision on a shared host or an optimal strategy for every query.
They support bounded probes because the measured skew and predicate-order losses decrease substantially without a semantic change.

- [Previous selector validation records](evidence/query-cost-2026-10-03/heldout-baseline.jsonl) and [summary](evidence/query-cost-2026-10-03/heldout-baseline-summary.json)
- [New selector validation records](evidence/query-cost-2026-10-03/heldout-tuned.jsonl) and [summary](evidence/query-cost-2026-10-03/heldout-tuned-summary.json)
- [New selector source archive](evidence/query-cost-2026-10-03/tuned-source.tar.gz) and [checksum](evidence/query-cost-2026-10-03/tuned-source-archive.sha256)

## Allocation comparison after selection changes

The new selector completed another 48 DHAT scopes with the same seed, size and heap protocol as the baseline.
This table compares automatic execution on skewed data, seed 11, with 1,024 Resources.

| Case | Previous allocations | New allocations | Previous total bytes | New total bytes | Previous peak bytes | New peak bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| selective_eq | 272 | 284 | 24043 | 25111 | 6511 | 6437 |
| common_eq | 137855 | 136711 | 14226860 | 13638422 | 2091549 | 2091549 |
| broad_range | 153749 | 150701 | 15740290 | 14824664 | 2297608 | 2297608 |
| sorted_page | 23283 | 20235 | 4860196 | 3944570 | 1591792 | 1591792 |
| conjunction_selective_first | 1209 | 1238 | 115834 | 118003 | 18203 | 18203 |
| conjunction_common_first | 17595 | 1238 | 3330056 | 118003 | 1276982 | 18203 |

Peak values cover new Rust allocations in one query scope, with its result still alive.
SQLite C allocations and pre-existing allocations remain excluded. Each cell represents one scope, not a statistical distribution.
[New heap records](evidence/query-cost-2026-10-03/heap-tuned.jsonl) and [summary](evidence/query-cost-2026-10-03/heap-tuned-summary.json) retain all cases.

## Reproduction and verification

Use the archived workspace for each measured version. Give each workspace its own Cargo target directory.
Build the latency binary with default features and without `heap`.
Run each binary with the following arguments and a different fresh directory:

```sh
--sizes 128,1024,8192 --repetitions 12 --warmups 2 --seed 203 \
  --directory FRESH_DIRECTORY --label RUN_LABEL
```

For calibration, use seed 11. For allocation trials, build with `--features heap` in a separate target directory.
Use these arguments for each heap binary:

```sh
--heap --sizes 1024 --repetitions 1 --warmups 1 --seed 11 \
  --directory FRESH_DIRECTORY --label RUN_LABEL
```

Use `scripts/prepare-query-write-baseline` to create the historical write workspace.
Use `scripts/measure-query-write-comparison` with separate baseline and indexed binaries.
Use `scripts/summarize-query-measurements.mjs` to recalculate query and heap summaries from JSONL records.
[Binary hashes](evidence/query-cost-2026-10-03/binaries.sha256) identify the measured executables.

The final harness rejects duplicate dataset sizes before it creates any databases.
This input guard was added after the source archives and measured binaries were frozen.
All preserved trials use distinct sizes. The guard does not change their execution or the production selector.
Its regression failed before the fix, then passed with the default and heap feature sets.

The final full `./scripts/check` run passed after that guard.
The reference `./demo/verify` run passed on SQLite and redb with the same production changes.
The [verification record](evidence/query-cost-2026-10-03/verification.md) identifies commands, source hashes and logs.
Independent [specification](measured-query-final-spec-review.md) and [quality](measured-query-final-quality-review.md) reviews found no unresolved blocker.
Those reviews distinguish source inspection from executed checks.

Release tasks 3.3 and 3.4 are complete. The operations and release tasks in stage four remain open.
The result preserves the Resource contract; it does not establish full release readiness.
