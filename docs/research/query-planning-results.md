# Recorded experiment, 2026-10-03

Environment: x86_64 NixOS container, rustc 1.99.0 (b940084d7 2026-09-28), cargo 1.99.0, rusqlite 0.40.2 with bundled SQLite 3.53.2, redb 4.3.0, serde 1.0.229, serde_json 1.0.151. Exact dependency graph: Cargo.lock. Commands and raw samples: `prototypes/query-planning/README.md` on the immutable prototype branch. Release optimization; two build jobs; no LTO setting. Host activity, CPU frequency and page caches were not isolated.

Eight conformance tests cover shared canonical authoring, invalid runtime fields, exact high unsigned values, missing/null, Unicode/NUL/text ties, moving continuation, policy-before-limit and scan-budget failure, standalone sort denial, true persisted redb reopen, and synthetic SQLite mutation/journal/reopen invariance. Negative controls demonstrate lossy REAL casts, conflated JSON missing/null, and incorrect SQL LIMIT before visibility. Compile-negative probes reject a string passed to the generated and hybrid unsigned operand with E0308. These tests demonstrate this fixture's semantics, not every operator/codec/authorization combination in maintained ROM.

Prototype source: `b7cfdb1907bdc30be455fd98f7e3b958c5ce87ce`, branch `codex/prototype-query-planning`. The [independent review](query-probe-independent-review.md) reran conformance, checked the reported medians and exercised 3,888 additional query/anchor vectors. Raw CSV and diagnostic logs remain beside the prototype source.

## Construction and rebuilds

31 samples, 10,000 constructions per sample, rotating profile order, black_box inputs/results. Median per normalized plan: generated **56.783 ns**, hybrid **59.322 ns**, runtime-from-Value **597.511 ns**. The typed profiles are close; runtime includes representation conversion and validation. This is not evidence that all runtime compilation is ten times slower, nor that generation is needed for execution speed.

Five rebuild samples/profile rotate order; dependencies remain compiled. Application clean median: generated **1.274197 s**, hybrid **1.253473 s**, runtime **1.451255 s**. Main-only edit median: **0.298345 / 0.299378 / 0.299284 s**. Release binary sizes: **4,053,064 / 4,053,040 / 4,114,496 bytes**. Features alter authoring only: every binary still contains database benchmark machinery. No cold dependency build, compiler peak memory or production macro expansion was measured.

## Query execution

Nine samples per case/profile at 1k, 10k and 100k rows, rotating execution order. All results assert against the oracle outside the timer. Timers include statement preparation, candidate decoding, policy evaluation, output allocation and instrumentation. Table below gives 100k-row median milliseconds; complete samples are in timings.csv.

| Case | Indexed SQLite | Unindexed SQL pushdown | SQLite raw snapshot + evaluator | redb full scan + evaluator |
|---|---:|---:|---:|---:|
| Selective amount range, amount/title order | 0.131 | 2.375 | 30.818 | 24.953 |
| Broad amount range, amount/title order | 0.168 | 52.704 | 31.550 | 25.937 |
| Selective range, title order | 0.147 | 2.316 | 30.188 | 24.658 |
| No match | 0.088 | 2.258 | 30.389 | 24.759 |

EXPLAIN confirms real indexed range searches; unindexed paths scan and use temporary sort structures. The index supports amount ordering but title-only ordering still uses a temporary sort. Indexed wins these deliberately compatible cases; unindexed broad pushdown loses to the reference snapshot. This supports case-dependent planning, not a universal SQL-pushdown or backend winner.

For selective range, indexed/unindexed both decode 100 rows (8,632 serialized bytes), but execute **1,112 vs 302,010 SQLite VM steps**. For broad range, they decode 200 candidates to find 20 visible rows but execute **2,208 vs 1,501,608 steps**. No-match decodes zero candidates yet unindexed executes **300,010 steps** (indexed 14). Raw snapshot decodes 100,000 rows (8,625,037 bytes), with 1,000,009 VM steps. Candidate/output counts cannot serve as database work budgets.

SQLite databases are in memory; redb databases are files with uncontrolled cache state. SQLite bytes are counted by serializing decoded rows; redb counts stored JSON bytes. This accounting adds work asymmetrically. Serialized bytes are not allocation counts, retained memory, RSS, disk bytes, or peak working set. No allocation or memory measurement was performed. Data has synthetic repeated titles and fixed visibility; results do not establish production selectivity or policy costs.

## Writes and consistency

31 samples of 100 in-memory commits, alternating indexed/unindexed order, each updating amount and inserting one journal record. Median per commit: unindexed **3,530.71 ns**, indexed **5,017.04 ns**. The fixture index adds roughly 42% here. After every batch, queries equal the oracle; the two journals contain the same 3,100 entries. File-backed conformance separately checks reopen. There is no throughput, multiwriter, durable-write-latency, recovery-injection or maintained-ROM journal claim.

## Decision supported

Use one normalized query representation and evaluator with generated or typed ergonomic helpers as equivalent front doors. Keep scalar semantics and authorization fixed across physical paths. An adapter may translate proven exact predicates/order and ask its database for a plan; it must not assume pushdown is cheaper or apply the output limit before policy. Collect separately observable database work where possible. Retain a bounded fallback for unsupported optimization. Index design and cost estimation need additional workloads; this experiment does not justify a full optimizer or a new maintained storage format.
