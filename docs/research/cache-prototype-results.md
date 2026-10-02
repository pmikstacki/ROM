# Internal deduplication and incremental-read findings

Date: 2026-10-02. These are disposable experiments, not a released ROM cache. The [brief](cache-prototype-brief.md) defines the questions; [primary-source research](internal-cache-research.md) documents the candidate contracts. Resource authors still declare one resource: caching, supervision and dependency bookkeeping belong inside ROM.

## Recommendation

Keep durable atomic idempotency authoritative and use ROM-owned, bounded single-flight to join concurrent retries. A small completed-outcome cache is justified for workloads with repeated completed commands, subject to current authorization and an absolute validity horizon. Keep its implementation private. Both tested libraries can serve that role; the experiment does not establish a universal performance winner or a production default.

Do not use a cache library's initializer lifetime to supervise an action. Moka and quick_cache can retry initialization after cancellation; an already submitted database operation may still commit. The prototype therefore uses separate job supervision, caller limits and completed-entry eviction. A disconnect releases the caller, while admitted work retains its execution permit.

Use Salsa only as a candidate for expensive pure derived reads. It proves useful dependency reuse here, but introduces projection, freshness and lifetime obligations. It neither deduplicates mutations nor replaces resource persistence. A human should not maintain a second Salsa schema beside their resource definition: ROM must generate or centralize that bridge before adoption.

## Cache experiment

The `prototype/dedup-cache` branch, commit `6153d99`, compares durable-only, supervised single-flight, single-flight plus Moka 0.12.16, and single-flight plus quick_cache 0.7.0. Both cache APIs are **synchronous**; ROM owns all asynchronous supervision. The Moka `future` initializer discussed in the research is not the measured implementation.

All modes share actual SQLite transactions over resource counters, revisions, command receipts and events. A runtime reuses one connection; blocking database work runs off Tokio workers. Each mode checks typed input equality and preserves tenant/principal/operation/command identity. The benchmark measures store attempts and committed bundles, not just cache hits.

Tests exercise matching concurrent requests, conflicting input, owner cancellation, expiry and observed capacity eviction, independent runtimes sharing one database, cached/in-flight delivery authorization, a lost response after real commit, distinct identities with equal payload, admission bounds and injected worker panic. Failure to find an in-memory entry always returns to durable arbitration. An `Unknown` result never populates the completed-success cache.

The coordinator independently reran formatting, Clippy with warnings denied, **13 unit tests plus one separate-process integration test**, and the demo. The suite also cancels a follower while other callers remain, resolves three joined waiters after an injected panic, and checks a mismatched concurrent request through the durable-only baseline. The process test starts two successive executables against the same database and observes one receipt, event and revision; it is a normal exit/reopen test, not a hard crash.

### Measured results

The implementer ran **288 release samples**: 144 with the container's default tmpfs scratch and 144 with verified ext4 scratch. Each filesystem has three repetitions, rotating mode order, four workloads, three payload sizes (64 B, 4 KiB, 64 KiB), and 400 measured calls per sample. Warm seeding is excluded. All samples assert the expected durable receipt/event/resource counts. The coordinator inspected raw results and independently recomputed the summary; timing was not independently repeated.

These are **ext4 throughput ranges**, requests/second across three repetitions, for 4 KiB inputs:

| Workload | Durable-only | Single-flight | + Moka | + quick_cache |
| --- | ---: | ---: | ---: | ---: |
| Unique commands | 244–285 | 155–273 | 173–274 | 186–274 |
| Replays across 16 warm identities | 41,774–42,793 | 40,222–41,581 | 1,865,724–2,473,136 | 2,445,287–3,308,191 |
| 80% warm replays, 20% new commands | 1,144–1,242 | 1,169–1,391 | 1,133–1,383 | 1,206–1,360 |
| Cold duplicate bursts, up to 32 callers | 6,258–7,210 | 6,411–8,403 | 7,240–7,933 | 7,159–8,450 |

The mechanism is clearer than the timing ranking. For 400 requests, warm replays reduce durable attempts from **400 to 0** with either cache. Mixed traffic reduces them from **400 to 80**, but its disk-backed timing ranges overlap: avoiding receipt reads does not avoid the 80 new commits. Cold bursts reduce attempts from **400 to 13 with single-flight alone**. Adding a completed cache does not reduce that count further in these samples. New commits stay respectively 400, 0, 80 and 13 in every mode.

Unique-request overhead is **not proven negligible**. Paired elapsed-time differences across payloads/runs ranged from −3.4% to +57.2% for single-flight, −6.8% to +43.3% for Moka, and −5.1% to +31.0% for quick_cache. Sequential samples on a shared host, disk synchronization and only three repetitions prevent attributing these differences solely to cache code. No acceptable overhead budget was preselected. These observations cannot justify an always-on production default or a claim of zero cost.

quick_cache leads the tiny hot-loop samples, but those complete in fractions of a millisecond, return two scalar fields and use in-memory auth; most other ranges overlap. This is insufficient to reject Moka or select a universal winner. Neither cache's native initializer or expiry engine is compared: ROM supplies supervision and expiry checks for both.

The [raw results, environment, summary script and assessment](https://github.com/pmikstacki/ROM/tree/prototype/dedup-cache/prototypes/dedup-cache) retain p50/p95/p99, attempts, commits, joins, hits, process CPU ticks and before/after RSS. CPU ticks are coarse; allocations and peak memory were not measured. No competing agent build ran during the timed sections, but the host was not CPU-pinned or isolated from unrelated activity. SQLite WAL with `synchronous=FULL` on ext4 is not a power-loss certification.

### Boundaries

- Authorization is an in-process principal-denial control, not a production row/field policy implementation. A current ownership or permission check may still need a database read on every hit.
- The request comparison uses whole typed input equality; durable encoding is JSON in fixed struct field order. It does not benchmark a cryptographic canonical fingerprint or arbitrary resource codecs.
- Completed caches use entry counts and bounded input sizes, not a measured weighted-memory policy. Process RSS snapshots are not peak memory or allocation attribution.
- The wrapper has one global flight mutex and one connection mutex per runtime. Results characterize this complete prototype, not unconstrained cache-library scalability.
- Durable receipts are retained indefinitely. Cache TTL is checked explicitly; storage-generation reset and finite durable identity horizons remain untested integration requirements.
- Panic injection occurs before the SQLite lock/transaction; it does not establish recovery from arbitrary poisoned locks or allocation failure. No power-loss claim follows from process recovery.
- No multi-host deployment, HTTP, real identity provider or combined notification-outbox pipeline is measured.

## Salsa experiment

Source and full assessment: [prototype/salsa-reads](https://github.com/pmikstacki/ROM/tree/prototype/salsa-reads/prototypes/salsa-reads), commit `5af9ca6`. Salsa 0.28.5 receives committed in-memory task snapshots with separately tracked owner, completion, title, collection membership and actor policy. Plain recomputation supplies an independent output oracle.

The coordinator independently reran formatting, Clippy with warnings denied, **eight tests**, and the demo on Rust/Cargo 1.99.0 in `rom-dev`.

| Operation on three tasks | Additional membership / list / count body executions |
| --- | --- |
| First query | 3 / 1 / 1 |
| 1,000 unchanged ID/count reads | 0 / 0 / 0 |
| Change an unused title | 0 / 0 / 0 |
| Reopen one visible task | 1 / 1 / 1 |
| Change a hidden task's owner while it remains hidden | 1 / 0 / 0 |

Policy revocation, actor changes and insertion/deletion/reinsertion produced the expected results after applying the new authoritative snapshot. A deliberately incorrect query reads an external permission flag without tracking it: after revocation, it returns its stale authorized result. The test passes by reproducing that bug. Dependency completeness is therefore part of ROM's contract, not something Salsa can infer from arbitrary Rust side effects.

This is a concrete read adapter, not a generic resource registry. Applying a full snapshot is O(n), changed collection queries can scan the collection, and inputs persist until the Salsa database is dropped. There is no measured memory plateau, cancellation stress, distributed invalidation or timing comparison. Avoided query bodies are evidence of work reuse, not a measured end-to-end speedup.

## What carries into the reusable core

1. Keep job ownership independent of the requesting future and the cache. Reject mismatched input before joining; bound jobs, callers and accepted bytes separately.
2. Cache only confirmed, minimally disclosive outcomes. Check present authorization at admission and disclosure; preserve durable identity and generation horizons.
3. Keep one resource descriptor authoritative. If incremental reads are adopted, derive their projection and invalidation metadata from that same contract.
4. Validate performance on a representative integrated resource/action path before fixing a default. Retain cache-disabled controls and include unique traffic, policy reads, churn, response serialization and deployment storage.

No vendoring or fork is warranted by these results. Normal private dependencies preserve the ability to optimize later without exposing cache selection to application authors. Production retention, policy freshness and shutdown integration remain explicit work; completing these experiments does not mark them implemented.
