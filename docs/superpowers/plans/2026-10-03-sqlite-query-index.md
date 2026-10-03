# SQLite query index implementation plan

> For agentic workers: use parallel implementation with the separate file ownership below and an independent review before integration.

**Goal:** Execute maintained scalar queries through SQLite's planner, with atomic derived keys and bounded rebuild/recovery.

**Architecture:** A fixed scalar-key table and exact per-kind counters derive from the registered catalog and current Rows. The SQLite read transaction admits the kind, inspects its native plan, calls the shared selector and materializes an owned response. The core retains residual query and disclosure rules.

**Tech stack:** Existing Rust 1.99.0, rusqlite and ROM interfaces. No new dependencies.

**Spec:** [Maintained query strategies](../specs/2026-10-03-maintained-query-strategies-design.md) and [physical layout proposal](../../research/maintained-index-layout-design.md).

## Scope decisions

Use the deterministic all-scalar profile for this experimental release implementation. Each live Row has one entry per scalar field, including optional missing members.
Logical descriptors do not contain physical index settings. Rebuild uses the complete persisted catalog, including kinds omitted by the next Runtime.
The release measurements must assess write and space costs before the profile becomes a production recommendation.

Bump the coordinated native format from 6 to 7 and archive format from 4 to 5. redb keeps reference query execution with the same logical format contract.
Support explicit native upgrades from 6 and archive upgrades from 4. Preserve retry epochs and receipt origins from those epoch-aware sources.
Older sources retain their existing stricter epoch-zero validation. Never silently rewrite an existing source.

No native result LIMIT. A selected scalar predicate can produce a complete superset; core evaluates all remaining predicates and ordering.
An absent or unrecognized SQLite plan estimate selects reference. Execution failures propagate without a second read.
Start with one leading equality/absence/range predicate and parameterized BLOB keys. Queries without a usable predicate use reference.

## Native schema and interfaces

Private `index.rs` is a facade for `encoding`, `catalog`, `validation`, `query` and related focused modules.

- `query_keys(kind TEXT, field TEXT, id TEXT, encoded BLOB)` has primary key `(kind,id,field)` WITHOUT ROWID and index `query_keys_value(kind,field,encoded,id)`.
- `query_kinds(kind TEXT PRIMARY KEY, row_count INTEGER, row_bytes INTEGER, canonical_row_bytes INTEGER, live_count INTEGER, generation BLOB)` uses nonnegative checked i64 counters and an eight-byte unsigned generation.
- `query_profile(id INTEGER PRIMARY KEY CHECK(id=1), encoding_version INTEGER, profile_version INTEGER, store TEXT)` contains exactly one row; store identity is fresh for each publication.
- `index::initialize(&Connection) -> Result<()>` creates those structures and profile metadata in the caller's new-store transaction.
- `index::register(&Connection, &[Descriptor]) -> Result<()>` adds zero counters for newly registered kinds. Existing definitions must match and retain their counters.
- `index::replace(&Connection, Option<&Row>, &Row, Option<usize>, checkpoint: impl FnMut() -> Result<()>) -> Result<()>` updates keys/counters/generation in the caller's commit. The length argument is the old persisted text length; new Row text is canonical. Unchanged keys remain untouched. Each actual key or counter write invokes the supplied checkpoint.
- `index::rebuild(&Connection, &Snapshot, BackupLimits) -> Result<()>` derives keys/counters from current native Row text and supplied validated logical data before staged publication.
- `index::validate(&Connection, &mut Collector, BackupLimits) -> Result<()>` compares complete derived structures against authoritative rows/catalog. Charge physical records/bytes through a new `Collector::physical(bytes)` method so they share the complete native validation budget.
- `index::encode(&Shape, Option<&Value>) -> Result<Vec<u8>>` implements the scalar encoding in the proposal. Custom codecs do not run in this function.
- `index::query_read(&Connection, &StorageQuery, QueryBounds) -> Result<QueryRead>` runs inside one caller-owned read transaction.
- `persistence::snapshot_rows(&Connection, kind, max_rows, max_bytes) -> Result<Vec<Row>>` is the shared reference reader used by both Storage methods.

The index validator must reject missing/extra/wrong entries and metadata. Bound loops before adding expected entries; do not allocate an unbounded complete derived map.
Store generation is monotonic within one kind; replay and failed transactions do not change it. Fresh rebuild resets generation under a fresh store identity.
Schema checks use canonical descriptor equality, not declaration field order.

## Review focus

1. Noncanonical old Row text needs separate raw and canonical byte totals.
2. A failed write, rejected revision or replay must not change keys/counters/generation.
3. Missing/null, u64 extremes, signed extremes, negative zero and UTF-8 prefixes must match core.
4. An interrupted rebuild must not publish a usable but incomplete index.
5. An epoch-aware source must retain nonzero retry boundaries during native/archive upgrade.

## Tasks and ownership

### Index catalog and integrity (agent: ste_specs)

Own `crates/rom-sqlite/src/index/catalog.rs`, `validation.rs`, related helper modules and private tests. Coordinator owns the facade.

- [x] Write failing native tests for registration, key deltas, metadata and corrupt membership.
- [x] Implement initialize/register/replace/rebuild/validate with checked bounds and complete catalog coverage.
- [x] Keep all writes inside the supplied transaction; validate every rebuilt structure before publication.

### Encoding and native query (agent: release_input_ergonomics)

Own `crates/rom-sqlite/src/index/encoding.rs`, `query.rs`, planner helpers and private tests.

- [x] Write failing encoding tests for scalar edge cases and native/reference candidate behavior.
- [x] Implement exact BLOB keys and a parameterized leading predicate; other predicates/order remain residual.
- [x] Inspect EXPLAIN QUERY PLAN for the same SQL/bindings. Use the shared selector with matched snapshot/request, checked relative costs and no forced index hints.
- [x] Admit exact whole-kind counters first, then execute once. Report measurements as pending.

### Format compatibility (agent: ste_guides)

Own `rom-backup`, `rom-redb`, their relevant existing format tests and documentation. Do not edit SQLite files.

- [x] Write failing tests for archive-4/storage-6 upgrade with nonzero retry epochs.
- [x] Bump formats; add `upgrade_v4_archive` and a shared `upgrade_current_snapshot` validator retaining epochs/origins.
- [x] Extend native redb upgrade/migration to format 6; validate pre-6 retry epochs only.
- [x] Add public `Collector::physical(bytes)` for one derived native record with checked combined limits.

### SQLite lifecycle integration (coordinator)

Own `index.rs`, `lib.rs`, `store.rs`, `persistence.rs`, `snapshot.rs`, `maintenance.rs`, `upgrade.rs`, `migration.rs`, rebuild API and new integration tests.

- [x] Add failing tests for changed/create/delete/replay, native selection and corrupt index startup.
- [x] Initialize the new schema; maintain keys in the atomic bundle; add coherent query_read and shared snapshot reader.
- [x] Extend old source inventories and format-6 upgrades without discarding retry epochs.
- [x] Rebuild during restore/migration/retention. Add explicit fresh-destination rebuild for valid authoritative data with corrupt derived contents.
- [x] Preserve source files, validate publication and test interruption, reopen, backups and byte limits.

### Integration and release evidence (coordinator with independent review)

- [x] Run all affected tests, demo upgrade/recovery and full local checks.
- [x] Record actual native versus reference evidence and unresolved performance work.
- [x] Review query and maintenance invariants before integration; keep full release goal open.

## Completion evidence

The [integration report](../../research/sqlite-query-index-results.md) records executed tests, review corrections and remaining measurement work.
Native format 7, archive 5 and explicit rebuild are implemented. The full release goal remains open.
