# Query planning experiment

Disposable Rust experiment, not a maintained ROM API, adapter format, authorization implementation, or general optimizer. It separates **three authoring profiles** (build-generated resource helpers, generic typed builder, descriptor/runtime wire compilation) from **three SQLite execution profiles** (raw snapshot plus evaluator, translated parameterized SQL without a secondary index, translated SQL with an index). A persisted redb scan supplies a fourth reference path. All compare complete returned rows with the same oracle.

The fixed schema supports unsigned amount `ge`, text equality, note equality (missing/null/value), conjunction, multiple sort fields and ascending ID tie-breaks. The generated build script emits Inventory and Ticket helpers. All authoring paths normalize to `Plan`; this is not generated SQL. Runtime timing starts from a parsed JSON Value and includes its clone/deserialization/lookup; it excludes JSON text parsing. Native typed inputs and wire values have different starting representations.

The executor stores exact u64 as eight big-endian bytes in SQLite BLOBs; it never coerces these to REAL or signed INTEGER. Text uses binary UTF-8 order; no locale collation or Unicode normalization. Missing, null and value have explicit ranks. The SQL translator binds values and expands moving anchors lexicographically. The prototype does not validate anchor binding to a kind/schema/query; that remains a maintained-core requirement.

Visibility is a synthetic persisted boolean, with 10% of fixture rows visible. SQL filters/orders candidates, then Rust checks visibility before the result limit. A deliberately wrong pre-policy limit is a negative control. The separate `authorize` sort fixture is not called by the executors and does not establish ROM authority correctness. Query operand trimming illustrates canonicalization only; this is not a full field codec or normalized-write test. Clients can construct Plan directly: this API is intentionally not a security boundary.

`Sqlite::snapshot` reads all bounded rows with no policy or query evaluation, then `oracle` applies those once. `Sqlite::query` executes the translated query. `Read` exposes candidates, serialized decoded bytes and SQLite VM steps; `explain` supplies SQLite's own access-path explanation. This is the interface for a separate strategy-selection experiment. Candidate budget bounds decoded SQL output, **not** database scan/sort work; the database may do substantial work before returning a row. The redb path bounds the full table scan.

Run in the native Rust environment, keeping build/database artifacts outside small tmpfs:

```sh
cd prototypes/query-planning
export CARGO_TARGET_DIR=/var/tmp/rom-query-probe-target
export CARGO_BUILD_JOBS=2
export QUERY_PROBE_SCRATCH=/var/tmp/rom-query-probe-data
cargo fmt --check
cargo test --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
cargo run --release --locked --offline -- all > results/timings.csv 2> results/execution.log
node build-cost.mjs
node diagnostics.mjs
```

The test scratch filenames belong exclusively to this experiment; journal fixtures are recreated. Redb reopen tests open the existing file without reinserting rows and verify a committed modification. SQLite tests compare a synthetic transaction (row update plus journal insert), repeated queries, events, and actual file reopen under all three strategies. These are not ROM Resource/action/receipt integration tests. Timing writes uses in-memory SQLite transactions and includes journal insertion/index maintenance; it measures no durable fsync latency.

See [results/report.md](results/report.md) for measured evidence and limits. External approach research is in the separate research branch's `docs/research/query-planning-research.md`; this prototype chooses no library for maintained ROM.
