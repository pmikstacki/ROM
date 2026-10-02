# Executed assessment: built-in action deduplication

Date: 2026-10-02. Branch `prototype/dedup-cache`, based on `90a87dc`. Throwaway probe only; no production code changed. Exact implementation, Cargo.lock, verifier and raw measurements are included.

## Answer

The exercised contracts work with ROM-owned supervision plus either pinned cache dependency. Single-flight alone removed duplicate durable attempts during overlapping bursts. A completed-outcome cache additionally removed sequential duplicate receipt lookups. Neither cache replaces durable identity/revision arbitration, input comparison, current authorization, or supervised ownership. The experiment supports keeping those responsibilities inside ROM while treating the cache library as an implementation detail.

The measurements do **not** establish a production default or zero overhead. Unique requests do all durable work in every mode and show substantial run-to-run variation, including slower cache-enabled samples. In mixed and burst disk-backed workloads, commit latency dominates and the throughput ranges overlap. Both libraries are viable candidates; no broad winner follows from this small probe. Use upstream dependencies first; there is no measured need to fork or vendor either crate.

## Verified behavior

`./verify` passed formatting, Clippy with `-D warnings`, **13 unit tests and one separate-process integration test**, and the ordinary-call demo. [Raw verification](results/verification.txt) is retained. Assertions inspect real receipt/event/resource counts, not merely equivalent return values.

Coverage includes all four modes' concurrent duplicate arbitration; cached/in-flight changed-input conflict; baseline's concurrent conflicting identities; caller-owner and follower cancellation; expiry and actual capacity eviction; normal runtime reopen; two independent runtimes sharing a database; separate executable process reopen; revoked authorization on cached admission and in-flight delivery; real successful commit followed by injected Unknown then retry; independent tenant/principal/operation/id scope; equal payload under distinct keys; admitted-job, waiting-caller and input bounds; and injected action panic with three joined waiters, flight cleanup and retry. Deterministic gates use semaphores/barriers rather than timing sleeps.

The demo reported two equal outcomes `{value: 1, revision: 1}` and one durable receipt, event and increment. The process test ran two fresh executables with empty private caches against one scratch SQLite file and confirmed the counts and revision remain one. This is a normal process exit/reopen, not an OS-crash or power-loss guarantee.

## Benchmarked path and environment

Rust/Cargo 1.99.0; optimized release build; NixOS `rom-dev`, Linux 6.6.94, AMD Ryzen 7 7700, four Tokio worker threads. Moka **0.12.16 sync** and quick_cache **0.7.0 sync**, verified using the current crates.io registry search and pinned exactly. SQLite uses bundled rusqlite **0.40.2**, WAL, synchronous FULL, one reused connection per runtime, and the same atomic state/revision/receipt/event transaction in every mode. There is no per-attempt connection-open distortion.

Two runs of the full matrix were retained. The initial default tempfile location is `/tmp` on **tmpfs**; [those results](results/benchmark-tmpfs.jsonl) characterize the memory-backed path only. The primary [ext4 results](results/benchmark-ext4.jsonl) reran the same workloads with `TMPDIR` in a verified ext4 directory under the workspace. Both comprise 144 samples: four modes × four workloads × three payload sizes × three repeated runs, 400 measured calls/sample. The three runs rotate mode order. Root coordinated the timed intervals so other prototype agents did not build or benchmark concurrently; background host activity was not controlled. [Environment details](results/environment.txt) and [all sample summaries](results/summary-ext4.md) are available.

Exact ext4 command after `cargo build --release --locked`:

```sh
mkdir -p results/scratch-ext4
TMPDIR="$PWD/results/scratch-ext4" ROM_BENCH_RUNS=3 ROM_BENCH_CALLS=400 \
  target/release/rom-dedup-cache-prototype bench > results/benchmark-ext4.jsonl
rmdir results/scratch-ext4
node summarize.mjs results/benchmark-ext4.jsonl > results/summary-ext4.md
```

Warm workloads seed 16 identities before measurement; cold workloads start with an empty runtime/cache/database, but do not flush OS caches. Mixed traffic is four hot calls per new call. Bursts align up to 32 callers per identity at a barrier; actual scheduler overlap determines joins versus cache hits. Every sample independently checks final receipt/event/value counts.

## Observed ext4 results

Throughput below is **median calls/s (minimum–maximum)** across three runs, for the 4096-byte payload. These are observed rates for this exact local probe, not a service SLO.

| Mode | Unique/cold | Hot/warm | Mixed/warm | Burst/cold |
|---|---:|---:|---:|---:|
| Baseline | 265 (244–285) | 41,831 (41,774–42,793) | 1,215 (1,144–1,242) | 6,785 (6,258–7,210) |
| Single-flight | 257 (155–273) | 41,409 (40,222–41,581) | 1,257 (1,169–1,391) | 7,783 (6,411–8,403) |
| Single-flight + Moka | 232 (173–274) | 2,375,311 (1,865,724–2,473,136) | 1,289 (1,133–1,383) | 7,435 (7,240–7,933) |
| Single-flight + quick_cache | 243 (186–274) | 2,502,424 (2,445,287–3,308,191) | 1,333 (1,206–1,360) | 7,430 (7,159–8,450) |

Mechanism evidence is clearer than rates. Across all ext4 samples:

| Workload | Baseline DB attempts | Single-flight DB attempts | Either cache DB attempts | New commits, every mode |
|---|---:|---:|---:|---:|
| Unique/cold | 400 | 400 | 400 | 400 |
| Hot/warm | 400 | 400 | 0 | 0 |
| Mixed/warm | 400 | 400 | 80 | 80 |
| Burst/cold | 400 | 13 | 13 | 13 |

Warm-up commits are excluded from measured counters. DB attempts mean entries into the durable transaction path, not individual SQL calls. Cold burst commit count is `ceil(400/32)`. Cache hits return a small scalar outcome with an in-memory authorization check and no database work; the very high hot-only rates depend on those conditions. A production authorization/database policy or larger output changes that path.

Paired unique-request elapsed overhead against the same-run baseline shows why no zero-cost claim is justified. Positive values are slower; negative values are faster. These are three samples each, not confidence intervals or evidence of a causal win.

| Payload | Mode | Run 0 / Run 1 / Run 2 elapsed overhead |
|---|---|---|
| 64 B | Single-flight | −3.4% / +20.4% / −0.2% |
| 64 B | Moka | −3.4% / −6.8% / +5.6% |
| 64 B | quick_cache | −5.1% / −4.6% / +9.6% |
| 4 KiB | Single-flight | +4.2% / +57.2% / +3.1% |
| 4 KiB | Moka | +4.2% / +40.8% / +14.4% |
| 4 KiB | quick_cache | +4.1% / +31.0% / +9.1% |
| 64 KiB | Single-flight | +0.5% / +4.0% / +0.8% |
| 64 KiB | Moka | +43.3% / +8.9% / −0.6% |
| 64 KiB | quick_cache | +4.2% / +13.5% / −2.8% |

The [complete table](results/summary-ext4.md) includes p50/p95/p99 per size/mode/workload. At 4 KiB, median p95 latency in microseconds for Baseline/Flight/Moka/Quick was unique `3709/6744/6884/6894`, hot `28.14/29.18/0.34/0.30`, mixed `3577/3641/3681/3661`, and burst `8912/7695/8846/9513`. The latency measurement starts after constructing the payload and, for bursts, after the barrier. Overall elapsed throughput includes payload construction and task setup. Quantiles are per sample; the summary reports the median of those quantiles.

CPU deltas ranged from 0–26 process clock ticks/sample (100 Hz); zero for a short sample is below counter resolution, not zero CPU cost. Observed process RSS snapshots ranged about 5,248–43,292 KiB across ext4 samples, with modes sharing one process and allocator history. These are neither isolated per-cache memory deltas nor peak RSS. Allocations were **not measured**.

## Boundaries of the conclusion

The private immutable runtime/database binding supplies cache ownership; storage-generation reset is not implemented or tested. Receipt retention is indefinite in the scratch schema, and shortening it would change the replay guarantee. Completed cache entries have an entry-count budget and common ROM TTL wrapper, not a byte-weighted hard cap; live jobs live in a separate bounded registry. The global registry mutex serializes cache access, so these results include wrapper contention and cannot rank standalone library lookup scalability. This is not a Moka native-async-initializer benchmark, and neither library's caller-owned initializer is used for durable commits.

Inputs are fully compared as typed values, including payload bytes. Cached values use Arc to avoid payload cloning on a hit, but the experiment is not a hashing benchmark or a constant-time equality design. Authorization is a synchronized local deny-set experiment, not a complete row-policy proof or a promise that authorization can always avoid reads. Revocation after the delivery check is not globally ordered with external policy changes.

Panic recovery covers a monitored action task before acquiring the SQLite lock. Arbitrary panic while holding the reused connection mutex may poison it. No attempt was made to prove power-loss behavior, arbitrary abort cleanup, large outcomes, remote database performance, multi-process coalescing, distributed coordination, weighted eviction, long-running-job deadlines, complete host-runtime shutdown or automatic DB-generation fencing. See the [README](README.md) for precise implementation and workload details.

Upstream references: [Moka 0.12.16](https://docs.rs/moka/0.12.16/moka/), [quick_cache 0.7.0](https://docs.rs/quick_cache/0.7.0/quick_cache/). The probe intentionally relies on ordinary supported APIs and the retained lockfile.
