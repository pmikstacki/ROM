# Throwaway built-in deduplication fast path

Question: can ROM own bounded action supervision and safely use an ordinary cache dependency to avoid repeated durable action attempts? This executable experiment compares baseline, ROM single-flight, single-flight plus **Moka 0.12.16 sync**, and single-flight plus **quick_cache 0.7.0 sync**. It does not create a cache-provider API or select a production default.

Run from this directory inside the persistent `rom-dev` NixOS container:

```sh
./verify
cargo run --locked
cargo build --release --locked
mkdir -p results/scratch-ext4
TMPDIR="$PWD/results/scratch-ext4" ROM_BENCH_RUNS=3 ROM_BENCH_CALLS=400 \
  target/release/rom-dedup-cache-prototype bench > results/benchmark-ext4.jsonl
rmdir results/scratch-ext4
```

Host invocation example:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/dedup-cache/prototypes/dedup-cache && ./verify'
```

`verify` runs formatting, Clippy with warnings denied, tests, and the ordinary-call demo; it never runs the benchmark. The integration test starts two separate executable processes against the same disposable database. `persist-demo /absolute/scratch.sqlite` intentionally retains that file; all other runs remove their scratch database directories.

## What is shared

Every mode uses the same typed action, authorization checks, admitted-job supervision, SQLite connection lifetime, and durable transaction. Each runtime owns one reused connection; independent runtimes use independent connections. SQLite WAL and synchronous FULL are configured. A transaction starts IMMEDIATE, checks the unique receipt identity, compares canonical typed input, reads current resource revision, conditionally updates state, inserts receipt and event, and commits atomically. A durable replay returns the original outcome. Benchmark DB attempts count entry into this durable path; actual commits count new action transactions, not read-transaction close or individual SQL statements. Each workload also asserts receipt count, event count, and aggregate resource value.

Key scope is `(tenant, principal, versioned operation, id)`. Target, amount and payload belong to typed input: changing any under the same identity conflicts. Equal payload under different identities applies independently. The runtime database binding is immutable and its private cache is newly constructed per runtime. There is no database-generation reset experiment or finite durable-receipt retention policy.

The resource-facing call is just `runtime.call(key, input).await`. Mode selection is private experiment construction. Moka and quick_cache are ordinary pinned dependencies, not forks or vendored source. Neither dependency owns an action initializer. ROM's separate flight map compares typed inputs before joining, and the supervisor retains the action when a requesting task is aborted. Completed cache entries contain only confirmed committed outcomes in `Arc`s. Expired/evicted entries fall back to the receipt transaction. A real post-commit injected Unknown is not cached as success; retry resolves the receipt. Separate runtime instances cannot coalesce with each other, but the database arbitrates their commits.

Admission bounds distinct jobs and all active callers separately (`bound` and `bound * 32`). Identifiers/target are at most 256 bytes each, payload at most 1 MiB. Overload is an immediate Busy result; no unbounded queued admission. Live jobs are never evicted by cache pressure or expiry. Cache capacity is an entry-count policy, not a byte-weighted hard memory budget. Moka's maintenance is asynchronous/best-effort. TTL uses one ROM-owned monotonic wrapper for both libraries; tests advance its offset rather than sleep. This measures the libraries' synchronous storage paths, not Moka's native asynchronous initializer or native TTL scheduler.

## Measurements

See [ASSESSMENT.md](ASSESSMENT.md), [raw ext4 results](results/benchmark-ext4.jsonl), [raw tmpfs results](results/benchmark-tmpfs.jsonl), [environment](results/environment.txt), and [verification](results/verification.txt). The initial tempfile default was `/tmp`, which is **tmpfs** here; those results are retained and explicitly separated from the rerun with verified ext4 scratch files.

Each filesystem has 144 samples: 3 runs × 3 payload sizes (64, 4096, 65536 bytes) × 4 workloads × 4 modes, 400 measured calls/sample. Mode order rotates by run. Workloads are sequential unique/cold; sequential hot/warm across 16 seeded identities; sequential mixed/warm with one new identity every fifth call; and cold synchronized bursts of up to 32 callers per identity. Warm seeding is outside timing/counters. Each sample creates a new runtime, cache and database; `cold` refers to those, not an OS page-cache flush. Burst overlap is scheduler-dependent and actual joins/hits/DB attempts are recorded. Barrier alignment does not force an artificial handler delay.

Overall elapsed time includes payload creation and burst task setup. Per-call latency begins after payload creation and after each burst barrier. Successful call completion is timed through delivery, with per-call p50/p95/p99 and overall throughput. CPU is process utime+stime delta in `/proc/self/stat` clock ticks (100 Hz on this host), so very short samples can round to zero. RSS is process resident KiB before/after each sample, not isolated cache memory or peak RSS. All modes run in the same process; allocator retention and background cleanup can carry across samples. Allocations were **not measured**. No library lookup-only or canonical-hashing benchmark is claimed: equality compares complete typed inputs, including payload bytes, and durable serialization happens inside the transaction.

## Limits

This is an executable model of one local action path, not production ROM integration. A global flight-map mutex also covers cache hits and serializes wrapper access. Four async workers and SQLite write serialization shape burst results. Cached outcomes are small scalar structs; large outcome costs were not explored. There is no connection pool, remote database, distributed supervisor, fairness policy, graceful host-runtime shutdown API, OS crash/power-loss test, durable receipt pruning, replica lag or storage rebinding. Runtime shutdown can end in-memory tasks; authoritative durable receipts remain necessary.

Authorization is a synchronized in-memory deny set checked at admission and delivery. This demonstrates revocation checks, not complete row-level policy or a claim that production authorization avoids a DB read. There is no coordinated external-auth freshness proof. Panic injection occurs before acquiring the SQLite mutex and proves monitored action-task cleanup with actual joined waiters. A panic while holding that mutex may poison it; arbitrary internal panics/OOM/abort are not covered. The prototype's fault injection and test gates are built into the same executable, inactive during measurements.

The tests use explicit semaphore/channel/barrier handshakes and assert real SQLite side effects. Timeouts only bound a hung handshake; sleep races are not used. The separate-process case exits normally after commit and checks reopening, not a hard crash. The selected cache dependencies are current registry releases verified with `cargo search`; full dependency resolution is preserved in Cargo.lock.
