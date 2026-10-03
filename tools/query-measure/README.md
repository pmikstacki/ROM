# Maintained query measurements

This unpublished tool measures complete public Runtime queries against SQLite.
It also provides a reusable commit workload for comparisons with the adapter before query indexes.
See the [measurement methodology](../../docs/research/query-measurement-methodology.md) for evidence limits.

## Run a small check

Use a fresh output directory. The tool rejects an existing directory.

```sh
cargo run -p rom-query-measure --bin rom-query-measure -- \
  --smoke --directory /tmp/rom-query-smoke --label smoke
```

## Measure query latency

Build without the `heap` feature. Stop other builds and measurement processes before the run.
Keep each raw JSONL file with its source revision and host information.

```sh
cargo run --release -p rom-query-measure --bin rom-query-measure -- \
  --sizes 128,1024,8192 --repetitions 11 --warmups 2 --seed 11 \
  --directory /tmp/rom-query-calibration --label calibration > calibration.jsonl
```

The default seed is `11`. Select and record a different seed before a held-out evaluation.
`--case NAME` selects one case. Case names are `selective_eq`, `common_eq`, `missing_eq`,
`narrow_range`, `broad_range`, `sorted_page`, `conjunction_selective_first`, and `conjunction_common_first`.

Each dataset runs automatic, reference, and forced-native modes through the same Runtime authority path.
Trials use all six mode permutations across repetitions, with an offset for each case and distribution.
Each record includes the requested mode and the actual strategy. Unsupported forced-native requests can use the reference strategy.

After each case's observed trials, a separate warmed Runtime runs the ordinary automatic production path.
These `production_query` records have no adapter counters. This pass helps assess instrumentation overhead, with its fixed later position disclosed.
The observed Runtime drains and drops before the production control acquires the same store.
The control also drains and drops before the next observed Runtime starts. Construction stays outside measured query intervals.

The tool compares complete ordered keys, revisions, and values outside the timer.
It records a digest for inspection, but equality does not rely on the digest.
The timer includes actor construction, query cloning, and final disclosure. It excludes comparison, formatting, and result destruction.
`storage_elapsed_ns` includes adapter metadata reads and planning. VM counters cover only the row materialization statement.

## Measure Rust allocations

Use a separate build and process. Heap builds require `--heap` and emit no latency samples.

```sh
cargo run --release -p rom-query-measure --features heap --bin rom-query-measure -- \
  --heap --sizes 128,1024,8192 --repetitions 1 --warmups 2 --seed 11 \
  --directory /tmp/rom-query-heap --label heap > heap.jsonl
```

Each query starts a new DHAT scope after setup and warm-up.
The result remains alive when the tool reads allocation counters. The profiler stops before comparison or reporting.
The peak includes tracked Rust allocations begun in that query scope. It excludes existing allocations and SQLite C allocations.
RSS records describe the process. Its high-water value includes setup and earlier datasets.

## Compare commit cost and file space

The write-only binary uses the same fixture and commit code without the observed-query API.
The command accepts a fresh database path, distribution, row count, and optional seed.

```sh
cargo run --release -p rom-query-measure --no-default-features \
  --bin rom-query-write-measure -- /tmp/rom-write.sqlite independent 1024 11
```

For a historical comparison, overlay this exact package and workspace dependency manifests onto the baseline source.
Build only the write binary with `--no-default-features`. Do not change durability settings between runs.
The binary rejects the `heap` feature.

The workload creates all rows, then updates up to 64 indexed fields and 64 non-indexed payload fields.
It does not measure deletion or receipt replay. The shared storage limits retain 32 journal rows and the default journal byte limit.
Write records include elapsed time and Linux process `write_bytes` and `cancelled_write_bytes` deltas when available.
These counters describe kernel-accounted storage writes, not physical SSD writes.
File lengths, page counts, and optional `dbstat` index bytes describe space. They are not write-amplification measurements.

## Fixture boundaries

Categories, activity flags, titles, and payloads use separate deterministic random streams.
The skewed distribution assigns approximately 90% of rows to `hot`.
The amount field equals the ascending Resource ID rank. This deliberate correlation is not an independent random distribution.
The Resource contains four scalar fields and one list payload. This workload does not represent every Resource shape or query plan.
