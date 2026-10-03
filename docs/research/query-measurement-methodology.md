# Maintained query measurement methodology

Date: 2026-10-03. Review baseline: `9db4b2ab4bb090a940f452eb1039c77c9d1c8aab`.
Scope: primary-source review and recommendations for the [measurement plan](../superpowers/plans/2026-10-03-query-cost-measurements.md). This note contains no benchmark results.

## What each measurement establishes

| Measurement | Scope | Does not establish |
| --- | --- | --- |
| Public Runtime query elapsed time | Admission, scheduling, grants, normalization, adapter selection and reads, core selection and final disclosure within the timer | Network latency, concurrent-host throughput or cold storage latency |
| Rows and bytes decoded by the adapter | Materialized Resource rows and their stored JSON bytes | Every Resource codec call, SQLite cache traffic or physical I/O |
| SQLite statement VM steps | Instructions executed by the selected materialization statement | Metadata queries, EXPLAIN, Rust filtering, scheduler work or nanoseconds |
| Scoped DHAT statistics | Rust global-allocator activity while the profiler is active | Existing heap, SQLite C allocations, stacks, mappings or process RSS |
| Linux `VmRSS` / `VmHWM` | Current / historical peak process resident memory | Exact per-query live heap or a resettable per-trial peak |
| Database and WAL file lengths | Physical footprint at a stated lifecycle point | Total bytes written to storage, fsync cost or write amplification |

The first two scopes follow the planned instrumentation boundary. They must be checked against the final harness and adapter implementation.
SQLite defines VM steps as a prepared-statement work proxy. Counts above 2,147,483,647 are undefined; widening the result to `u64` does not remove that limit. A counter reset returns the old value and resets subsequent counting. Capture each sample before statement reuse, or explicitly reset at its boundary. Sources: [SQLite statement counters](https://www.sqlite.org/c3ref/c_stmtstatus_counter.html), [statement status API](https://sqlite.org/c3ref/stmt_status.html).

EXPLAIN QUERY PLAN describes access paths and nested loops. It is not an elapsed-time measurement or an exact candidate-count estimate. Its text format can change between SQLite releases. Record the SQLite version and retain raw plans for inspected workloads. Unknown plans should retain the reference fallback. [SQLite EXPLAIN QUERY PLAN](https://www.sqlite.org/eqp.html).

## DHAT 0.3.3: scope and interpretation

The verified documentation identifies version 0.3.3. A `Profiler` starts profiling on construction and stops on drop. Only one profiler may run at once. `HeapStats::get()` requires an active heap profiler; it need not use testing mode. `ProfilerBuilder::testing()` suppresses the usual profile output on drop. Sources: [Profiler](https://docs.rs/dhat/0.3.3/dhat/struct.Profiler.html), [HeapStats](https://docs.rs/dhat/0.3.3/dhat/struct.HeapStats.html), [ProfilerBuilder](https://docs.rs/dhat/0.3.3/dhat/struct.ProfilerBuilder.html).

Record `total_blocks`, `total_bytes`, `max_bytes`, `curr_blocks` and `curr_bytes`. `max_blocks` is the block count when tracked bytes reached their peak; it is not a separate peak block count. Sample these statistics before dropping the profiler. [DHAT HeapStats definitions](https://docs.rs/dhat/0.3.3/dhat/struct.HeapStats.html).

A profiler started after setup cannot measure the whole live process heap. Allocations made before its scope are ignored when retained or freed. A reallocation during the scope becomes a tracked allocation. The global profiler can observe allocations from other Rust threads. Label a scoped peak as **peak tracked Rust bytes allocated during the query scope**. [DHAT profiling scope](https://docs.rs/dhat/0.3.3/dhat/#running), [pinned 0.3.3 source](https://github.com/nnethercote/dhat-rs/blob/0.3.3/src/lib.rs).

Recommended sequence:

1. Build the Runtime, populate the database and warm all query modes before profiling.
2. Construct the profiler immediately before the query attempt.
3. Run one complete public query and await its result.
4. Sample statistics while the result remains alive; label this boundary explicitly.
5. Drop the result consistently, then sample current bytes if retained-memory analysis is needed.
6. Drop the profiler before formatting JSON, reading `/proc`, or writing report files.

Use separate release binaries for timing and profiling. Enabling the DHAT allocator changes allocation cost even when no profiler is active. Do not mix profiled timings with the latency dataset. DHAT recommends release builds and notes potentially substantial profiling overhead. Preserve line debug information for useful allocation traces. [DHAT configuration and setup](https://docs.rs/dhat/0.3.3/dhat/#configuration-profiling-and-testing).

For strongest isolation, run one workload and strategy per heap-profile process. A scoped profiler in a sequential harness is also useful, with the stated cross-scope limitations. Its peak must not be computed by subtracting two cumulative high-water marks. Quiesce unrelated background tasks before each scope. These are measurement recommendations, not guarantees supplied by DHAT.

## Rust, SQLite and RSS are different memory views

Rust's `System` allocator uses platform allocation functions, but direct system allocations can bypass the chosen Rust global allocator. SQLite normally uses C `malloc`, `realloc` and `free`, unless configured with another allocator. Therefore DHAT's Rust wrapper does not, by itself, count SQLite's internal allocations. No SQLite allocator redirection was found in the maintained adapter during this review. Sources: [Rust System allocator](https://doc.rust-lang.org/std/alloc/struct.System.html), [SQLite memory allocation](https://sqlite.org/malloc.html).

Keep RSS outside timed and profiled regions. Record it before and after a phase, with the process identifier and phase name. Prefer the unprofiled binary for process-memory comparisons; a profiling process also contains the profiler and its bookkeeping. `VmHWM` includes earlier setup and trials. It cannot establish that a later strategy caused the recorded peak. Linux also documents asynchronous RSS accounting; `/proc/PID/smaps` provides a more precise but slower snapshot. Do not describe a `VmHWM` difference as exact query allocation. [Linux proc documentation](https://docs.kernel.org/filesystems/proc.html).

SQLite exposes separate memory-status interfaces. Adding those could narrow the native-memory gap later, but RSS and Rust heap data should remain distinct even then. [SQLite memory status](https://sqlite.org/malloc.html#memory_status).

## Real-pipeline comparison

Use the same indexed database, descriptors, Runtime limits, actor and uniform read/field policy for automatic, reference and permitted-native modes. Compare the entire ordered result, including keys, revisions and values, before recording samples. A matching result count is insufficient. Unsupported forced-native requests must report their actual strategy or explicit rejection.

Time from immediately before the public query call through its completed result. Specify whether query cloning and result destruction are inside this interval. Keep validation, JSON formatting, metric extraction and stdout writes outside it. Use identical boundaries for all modes. Also compare the normal production path with observed automatic mode to estimate instrumentation overhead.

Report full-pipeline elapsed time beside materialization counters. Fewer decoded rows or VM steps can explain a result, but cannot substitute for elapsed time. Normalization, metadata and planner overhead can dominate small or empty queries. A sorted page can return one row while materializing many candidates; retain both counts.

Use warm-up samples that are discarded. Rotate strategy order within each workload/size/repetition block. Across larger runs, use a recorded deterministic shuffle or all six strategy permutations to reduce carry-over bias. Do not equate many repetitions in one warmed process with independent host runs. Randomized designs and interleaved repetitions are established ways to reduce order confounding. Sources: [NIST randomized designs](https://www.itl.nist.gov/div898/handbook/pri/section3/pri331.htm), [Google Benchmark interleaving and warm-up guidance](https://google.github.io/benchmark/user_guide.html#random-interleaving).

Run release measurements without concurrent builds, other benchmark processes or maintenance workloads. Record CPU, kernel, container configuration, thread counts, compiler, flags, allocator, SQLite version, Cargo.lock identity, source revision and dirty diff. Label results as warm-cache measurements unless a separate cold-cache experiment controls that condition.

## Fixtures and independent validation

The following recommendations address the current selector's fixed selectivity assumptions. They are proposed ROM experiments, not claims about production workload distributions.

| Axis | Required cases |
| --- | --- |
| Scale | Tiny kinds where planning dominates; larger kinds within declared whole-kind limits |
| Selectivity | No match, rare equality, common equality, narrow range, broad range |
| Distribution | Independent fields; heavily skewed popular values; deliberately correlated fields as a separate scenario |
| Query shape | Sorted small page, full results, both orders of the same conjunction, moving anchor |
| Representation | Narrow and wider payloads; missing/null; variable string lengths |
| Mutation | Create, indexed-field change, non-indexed-field change, deletion and exact receipt replay |

Do not derive every field from the same remainder of the row ID. That can make supposedly independent predicates deterministically correlated. Use reproducible, separately seeded streams or a domain-separated deterministic generator. Record the generator version and seed. Check marginal frequencies and joint counts in fixture tests; naming a dataset “independent” is not evidence of independence.

Choose calibration and held-out seeds before inspecting timings. Use additional held-out sizes, selectivities or distributions, rather than only a different row order of the same dataset. Freeze a selector change before running the held-out set. Record any later tuning as another experiment with new validation data.

Include a conjunction whose first predicate is broad and whose second predicate is selective, then reverse their order. The existing leading-predicate strategy can produce very different candidate counts for these equivalent requests. Record the selected predicate or raw plan, candidate count and full logical match count outside the measurement window. Final page size alone conceals this problem.

Compare automatic elapsed time with both alternatives per workload block. Report median and spread, sample count and repeated-process variability. Preserve outliers and raw samples. A small sample does not support a stable tail-percentile claim. Treat noisy ties as unresolved. Report selection loss relative to the faster alternative separately from any weighted workload aggregate; an arbitrary average can hide severe skew regressions.

Before changing cost weights, inspect which cost component explains repeatable losses. Candidate-count estimation, predicate choice and planner startup can require different fixes. Any extra probe or statistics maintenance must be included in automatic-mode timing and write costs. Do not calibrate weights against materialization-only counters while claiming complete Runtime optimization.

## Writes and space

Read-mode comparisons on one indexed database do not measure index maintenance overhead. Measure the same real commit sequence against the maintained pre-index baseline and current implementation in separate isolated stores. Keep durability, receipt/work semantics, payloads and limits equal. Preserve baseline binaries or revisions and raw output.

Separate setup, initial load, replacements, deletes and replays. Record exact successful operations and checkpoint state. File bytes observed with an active WAL are not comparable with a checkpointed main file alone. Include existing WAL bytes, or checkpoint both stores consistently before a separate settled-size sample. Do not turn off durability to obtain cleaner timings. File-size differences alone are space evidence, not device-write amplification.

Linux `write_bytes` attributes storage-layer writes at page-dirtying time; `cancelled_write_bytes` records writes prevented by truncation. Report both process-counter deltas separately. They do not establish physical SSD writes, filesystem-wide traffic or device write amplification. Read counters outside the timer and keep report output outside the sampled interval. [Linux process I/O accounting](https://docs.kernel.org/filesystems/proc.html#proc-pid-io-display-the-io-accounting-fields).

## Harness review status

The first implementation review inspected [fixture.rs](../../tools/query-measure/src/fixture.rs), [write.rs](../../tools/query-measure/src/write.rs) and their tests while the query runner was still being written. The package is unpublished, its library root is a facade, and its write workload calls the public Runtime API. These are source-inspection findings, not measurement results. The review also applies the repository's Beskid quality rules: keep roots as facades, put implementation in cohesive modules, and share fixture and protocol rules instead of copying them into benchmark-only implementations.

The review found and the author corrected a fixture issue: the initial generator mixed seed + index + domain constant. Thus, random fields for seed 12 at row i exactly matched seed 11 at row i+1. Adjacent seeds mostly shifted the same dataset. The current source mixes the seed and field domain before adding the index. A regression compares adjacent seeds against shifted calibration rows. The author reported observing this test fail before the correction and pass afterward; this review confirmed the source change without rerunning it.

The adapter and observed wrapper share the production read path. Forced native mode changes only cost preference; it still uses the shared capability, binding and plan checks. The wrapper measures the complete adapter call, including its transaction, and rejects a second unconsumed observation in a sequential trial. Materialization counters remain separate. The write workload currently covers seed, indexed-field update and payload update; it does not yet measure deletion or receipt replay.

Fixture tests now check hot-value frequencies across amount quartiles and category/active combinations. These are useful sanity checks, not a statistical proof of independence. Amount deliberately equals the row ID rank and must remain labeled as correlated with ID. The completed runner and heap scope were then reviewed in source. The review findings below describe implementation boundaries, not reproduced timing or allocation results. No heavy benchmark was run for this review.


### Runner and heap scope review

The runner compares complete ordered `ProjectedView` values with the reference result outside the measurement window. It also checks warm-up results and each recorded trial. Limited queries obtain their full logical match count with an untimed query that removes the limit. The result digest supplements these comparisons; it does not replace them.

The query timer starts before actor construction and query cloning, then ends after the public Runtime call returns. Result destruction occurs later. The adapter timer encloses the transaction-owning observed read. Its elapsed time excludes the wrapper's subsequent metrics-lock bookkeeping, while the public query timer includes that work. These are useful nested boundaries, but their difference is not solely the core evaluator's cost.

The heap profiler starts before the same query attempt. `HeapStats::get()` runs with the result alive. Consuming `Profile::finish` drops the profiler before comparison, JSON serialization, digest calculation or RSS sampling. Its current-byte sample includes tracked allocations still live at that boundary, not only the result. The settings gate separates query latency runs from heap-enabled builds. Previously constructed fixtures and the retained expected result remain outside tracked allocation totals.

The modules have cohesive responsibilities for fixture generation, workload definitions, settings, observations, memory, space and execution. The library root remains a facade. Storage delegation preserves real Runtime authorization and decoding; no separate benchmark evaluator was introduced. The write workload is shared with the standalone pre-index comparison binary.

The author applied the final review corrections, and the review confirmed them in source:

- The runner cycles through all six mode permutations, shifted by case and distribution. The initially implemented three cyclic rotations balanced position but did not cover both directions of carry-over. Use repetition counts divisible by six for exact balance within each workload.
- The run header explicitly states that query cloning and actor construction are timed, while result destruction is excluded. The heap scope encloses the same query attempt.
- Correct the standalone write binary's heap label: enabling the optional dependency does not install the allocator in that binary. The author resolved this by rejecting heap-enabled write runs before opening a store; the review confirmed that guard in source.

The runner now adds a direct Runtime automatic-query control without the observation wrapper. It uses the same store, limits, Resource definition, actor and query, verifies complete result equality, and warms this path before timing it. This control always follows the observed trials for that workload. It can expose a large instrumentation difference, but fixed order and a separate Runtime prevent interpreting the difference as an exact causal overhead estimate. Repeat with balanced control order if that distinction affects the selector conclusion. No timing measurements were reproduced in this review.

Capture revision, compiler, SQLite version, lockfile and dirty diff with the external run record; the current harness label alone is not that provenance. The author reported five focused tests passing after the corrections. This review inspected their source and did not independently rerun them.

## Baseline artifact validation lesson

During coordinator validation, the baseline and indexed builds shared a Cargo target directory. Both captured executable paths contained the same historical executable: their hashes matched, and both generated databases reported zero query-index page bytes. The coordinator stopped and invalidated that run. It supplies no comparative write-cost evidence. This account is coordinator-reported evidence; the independent review did not reproduce the failed build.

Use fresh, separate target directories for historical and current builds, and preserve each executable before the next build. Source labels and intended Cargo commands cannot establish which executable actually ran. The revised [comparison script](../../scripts/measure-query-write-comparison) rejects byte-identical executable files, records their hashes, then runs an eight-row database preflight for each. It requires zero query-index bytes for the baseline and a positive count for the indexed binary. Missing or unavailable index-size evidence fails the preflight. This combines artifact identity with observed database behavior; different hashes alone would not prove the intended implementation difference.

The [baseline preparation script](../../scripts/prepare-query-write-baseline) archives the chosen historical commit, then overlays the current harness, workspace manifest and lockfile. Its provenance file records that combination and hashes the overlaid files. This is a historical adapter built with a common current workload and dependency lock, not a claim to reproduce the historical release binary. Prepare it from frozen harness sources and retain that provenance with both build commands and executable hashes.

The comparison script runs each measurement in a fresh process and fresh database. Its four write repetitions alternate the pair order, with baseline first twice and indexed first twice. The review confirmed the revised loop before the valid comparison run. These remain descriptive samples. The coordinator's twelve query repetitions cover the six query-mode permutations twice per workload; that is separate from the write script's four process repetitions.

## Summary script review

The [summary script](../../scripts/summarize-query-measurements.mjs) sorts each metric and uses the central value for odd sample counts, or the mean of the two central values for even counts. Independent synthetic checks passed for both cases and for twelve-sample nearest-rank p95. Shell syntax checks for both preparation/comparison scripts and the JavaScript syntax check also passed. No builds or comparative benchmarks were run during this review.

At twelve samples, nearest-rank p95 selects the maximum. Preserve its explicit estimator name and sample count; it is not evidence of a stable workload tail. The summary groups files with matching run label, record type, distribution, seed, size, case and requested mode. It records the set of actual strategies, but combines those files' samples and does not retain process identity. Use the original JSONL files for process-level variability or paired analyses. Keep source revisions under distinct labels, and do not supply a raw file twice. These reporting limits do not invalidate the median calculation.
