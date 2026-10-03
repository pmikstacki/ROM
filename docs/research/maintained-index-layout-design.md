# Proposed physical indexes for maintained queries

Date: 2026-10-03. Status: design proposal, not implemented behavior.
Baseline: `383b03e`, worktree `release-query-index`.
This report reviews maintained source and primary SQLite documentation.
It does not run benchmarks or workspace tests. Earlier measured results remain
in [query-selector-results.md](query-selector-results.md).

## Recommendation

Use a fixed normalized table of scalar field keys in SQLite. Generate its entries
from registered Resource descriptors and validated current Rows. Keep the existing
`resources` table authoritative. Keep one canonical query evaluator in core.
SQLite selects physical access paths; ROM selects between complete, semantically
compatible execution strategies.

For the first maintained version, use a deterministic physical profile:
one index entry for each scalar field of each live Row, including optional missing
fields. Do not add public per-field index configuration yet. Store the profile and
encoding version in adapter metadata. This keeps backup reconstruction deterministic
and avoids adding index choices to logical Descriptor equality.

This recommendation prioritizes generic coverage and a small lifecycle contract.
It has real write and space costs. Measure those costs before making this profile
an unconditional production default. A future sparse host-selected profile needs
its own persisted catalog, readiness state and restore contract.

## Maintained facts that constrain the design

The maintained [query evaluator](../../crates/rom/src/query_eval.rs) admits a full
kind before selecting a page. The admission limit counts tombstones and the full
serialized Row, including protected metadata. A selective index must not bypass it.
The evaluator supports at most 32 predicates and four order fields.

For field ordering, core checks sort-field read permission on every live row that
passes row authorization. That check happens before predicate and anchor filtering.
A row excluded by a native predicate can therefore still cause `Denied` today.
An indexed candidate set alone cannot reproduce that result.

The [SQLite commit path](../../crates/rom-sqlite/src/persistence.rs) already has one
immediate transaction for Row, references, receipt, journal and effects. Index
maintenance belongs in that transaction. Receipt replay returns before mutation.
The [native snapshot reader](../../crates/rom-sqlite/src/snapshot.rs) validates an
exact table/index inventory. New structures require an explicit format transition.

The [integration review](maintained-selector-integration-review.md) provides the
full authorization, admission, cursor and failure-equivalence requirements.
Those requirements remain binding for this proposal.

## Layout alternatives

| Layout | Benefit | Cost and semantic risk | Recommendation |
| --- | --- | --- | --- |
| Normalized field-key rows | Fixed DDL for arbitrary Resources; exact Rust encoding; ordinary SQLite indexes | One entry per indexed scalar; extra joins or probes for multiple fields; larger write transactions | First maintained implementation |
| Generated expression indexes over JSON | SQLite maintains entries automatically; useful for known fixed expressions | Exact expression matching; JSON numeric conversion; per-field DDL/catalog; custom deterministic encoder must be registered consistently | Keep as a later measured specialization |
| Per-Resource tables with named columns | Familiar SQL statistics, covering and composite indexes | Dynamic DDL, column migrations, identifier mapping, cross-profile maintenance; duplicates logical schema machinery | Too much new lifecycle work for this stage |

SQLite considers an expression index when the query uses the corresponding
expression, rather than proving arbitrary algebraic equivalence. Index expressions
also require deterministic functions. A custom encoder can make an expression
index exact, but it adds a function-registration and versioning contract.
[SQLite expression indexes](https://www.sqlite.org/expridx.html).

Generated columns inherit SQL affinity rules. VIRTUAL columns calculate on read;
STORED columns calculate on write. SQLite cannot add a STORED generated column
with `ALTER TABLE ADD COLUMN`. These constraints make dynamic named-column layouts
more expensive to maintain.
[SQLite generated columns](https://www.sqlite.org/gencol.html).

## Proposed native structures

Names are illustrative and remain private to the adapter.

```sql
CREATE TABLE query_keys (
    kind TEXT NOT NULL COLLATE BINARY,
    field TEXT NOT NULL COLLATE BINARY,
    id TEXT NOT NULL COLLATE BINARY,
    encoded BLOB NOT NULL CHECK(typeof(encoded) = 'blob'),
    PRIMARY KEY(kind, field, id)
) WITHOUT ROWID;
CREATE INDEX query_keys_value ON query_keys(kind, field, encoded, id);

CREATE TABLE query_kinds (
    kind TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
    row_count INTEGER NOT NULL CHECK(row_count >= 0),
    row_bytes INTEGER NOT NULL CHECK(row_bytes >= 0),
    canonical_row_bytes INTEGER NOT NULL CHECK(canonical_row_bytes >= 0),
    live_count INTEGER NOT NULL CHECK(live_count >= 0),
    generation BLOB NOT NULL CHECK(length(generation) = 8)
) WITHOUT ROWID;

CREATE TABLE query_profile (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    encoding_version INTEGER NOT NULL,
    profile_version INTEGER NOT NULL
);
```

`query_keys` has one primary entry per `(kind, field, id)` and a secondary index
for range access. The primary key prevents duplicate memberships. A value update
changes the secondary index through SQLite's transaction machinery.
`WITHOUT ROWID` avoids an unrelated integer key for this composite-key table.
Its benefit must still be measured for large text keys; it is not universally
smaller or faster.
[SQLite WITHOUT ROWID](https://www.sqlite.org/withoutrowid.html).

`row_bytes` counts persisted `resources.data` UTF-8 bytes.
`canonical_row_bytes` counts the full Row serialized by core. The existing SQLite
and core paths enforce these separately. Native JSON with different whitespace,
escapes or number spelling can make the totals differ. Track both unless a separate
compatibility decision requires canonical native text at startup. New writes and
restored Rows are already serialized canonically.

Keep counts within checked signed 64-bit storage limits. Reject overflow rather
than converting to floating point. Store the monotonic generation as eight-byte
unsigned data, decode in Rust and increment with checked arithmetic. Generation
exhaustion is an explicit error. A new publication gets a fresh store identity,
so a restored generation cannot validate an old cached plan.

Derive the field inventory from the complete persisted descriptor catalog, not
only types registered by the latest Runtime. Otherwise an omitted kind can lose
index coverage after reopen. Scalar means the existing evaluator's recursively
unwrapped Optional/Nullable shape; List and Map values remain on the reference
path. String-shaped custom codecs use their already normalized stored value.
An index encoder must not invent an additional normalization rule.

Keep profile metadata separate from Resource metadata. This profile indexes
`Row.value` only. Tombstones, historical receipts, journal Rows and protected
`deletion_authorization` values must not produce live query entries.

## Exact scalar encoding

Use one Rust encoding routine for writes, rebuilding, query bindings and anchors.
Treat the following bytes as a versioned proposal, not an external wire format.
Each field has a fixed registered shape. Comparisons never mix different fields
or shapes in one key range.

| Input | Proposed key | Required behavior |
| --- | --- | --- |
| Missing optional member | `00` | Less than null; one explicit entry for a missing member |
| Present null | `01` | Distinct from missing; less than present scalar values |
| Bool | `02` followed by `00` or `01` | False before true |
| `u64` | `02` followed by eight-byte big-endian value | Exact through `u64::MAX` |
| `i64` | `02` followed by big-endian bits XOR `0x8000000000000000` | Signed order, including `i64::MIN` |
| Finite `f64` | `02` followed by sortable IEEE bits | Normalize both zeros to positive zero; complement negative bits, flip the sign bit for nonnegative values; big-endian result |
| String, enum, reference | `02` followed by UTF-8 bytes | Bytewise order, including empty strings, NUL and prefixes |

For floats, use the same `Value::as_f64` conversion as core, reject nonfinite
values, and test subnormals and adjacent representable values. Do not substitute
`total_cmp`: core treats negative and positive zero as equal.

Source inspection confirms a separate equality boundary. Pinned
`serde_json 1.0.151` compares integer and floating Number variants as unequal.
The maintained `query_eval::equal` instead compares scalar F64 values through
`compare_value` and `as_f64`: scalar `1` and `1.0` therefore compare equal.
List/Map equality still uses structural `Value` equality, so the same substitution
is not valid inside containers. Plan and anchor structural comparisons are also
representation-sensitive. Keep residual evaluation and complete canonical plan
identity; an equal index key is not proof of an equal serialized plan or anchor.
This is a source-derived conclusion, not a new executed experiment.
[Maintained evaluator](../../crates/rom/src/query_eval.rs),
[pinned serde_json Number source](https://docs.rs/serde_json/1.0.151/src/serde_json/number.rs.html).
 This encoding must
match both scalar equality and scalar ordering.

SQLite INTEGER is signed; REAL is binary64. Numeric affinity can convert an
out-of-range integer string to REAL. Therefore JSON extraction plus a numeric cast
cannot establish ROM's exact unsigned-integer semantics. Bind index keys as BLOBs.
SQLite compares BLOBs bytewise, which fits the proposed ordering.
[SQLite storage classes and comparison](https://www.sqlite.org/datatype3.html).

Do not concatenate multiple variable-length field keys without a tuple encoding.
The first version uses separate indexed rows. Core can sort multiple order keys;
SQLite can also sort explicit joined key columns. An eventual composite-key codec
needs escaping/length boundaries and separate tests.

For ordinary `Ne(value)`, exclude missing entries. A present null can match a
non-null `Ne`, as it does in core. Ordered comparisons exclude null and missing.
`Eq(absent)` and `Ne(absent)` are separate presence tests. Descending field order
reverses missing/null/value ordering, but the final ID tie remains ascending.
A reversed `(encoded,id)` scan alone does not satisfy that descending contract.

Structural List/Map equality keeps the existing evaluator. Do not call a hash
collision or a JSON-string comparison exact structural equality.

## Query session requirements

Use one adapter-owned operation with a coherent read transaction. The operation
performs exact admission, obtains physical metadata, applies a pure ROM strategy
selector, and materializes either complete reference Rows or eligible native
candidates. It then releases the transaction and native mutex before returning
to the common evaluator.

The ordering must be explicit:

1. Before storage access, core checks actor authority and query/sort grants,
   runs field-codec normalization, and determines native eligibility. Any explicit
   row-independent read grant is evaluated here, outside native locks.
2. The adapter starts one read snapshot. It obtains exact counters, versions and
   generation, inspects optional planner information, then selects and executes
   the eligible physical path. Only ROM-owned pure operations run in this scope.
3. The adapter fully materializes the bounded result and drops all native guards.
4. Core runs opaque row/field policies and the common evaluator. It keeps existing
   actor/disclosure-generation rechecks around the observation.

Do not run user authorization or codec callbacks while holding the adapter mutex
or transaction. A callback can call `Storage::load` on the same adapter and
re-enter the mutex. A callback panic under that mutex can poison it and change
later failure behavior. A generic session callback that receives arbitrary user
code is therefore the wrong boundary.

Do not expose unrelated unbound `count`, `estimate` and `execute` operations.
Do not require a rusqlite Transaction to be Send or expose SQLite types in core.
Keep native execution in the existing tracked blocking operation. Do not await
application futures or network traffic while retaining the native snapshot.

The root design owns the public interface. It must express these facts:

| Fact | Why core needs it |
| --- | --- |
| Exact full-kind row and byte counts | Preserve `TooLarge` before selection |
| Canonical plan identity and semantic version | Estimates and results belong to the requested operation |
| Descriptor and physical profile versions | Reject mismatched scalar encodings or incomplete coverage |
| Same-snapshot generation/store identity | Prevent stale or restored-plan reuse |
| Complete candidate coverage and ordering contract | Distinguish exact filtering, superset filtering and final ordering |
| Explicit native eligibility established before storage | Prevent physical filtering from hiding opaque policy behavior |
| Complete reference materialization for opaque policies | Preserve row policy and sort-field errors without callbacks under locks |
| Actual execution failure | Propagate failure without switching strategies |

Opaque policies always use the complete reference path in this stage. Existing
`allow_all_fields()` callbacks do not themselves establish a proof that native
filtering is safe. Native eligibility needs an explicit contract covering every
row and field permission that filtering or sorting would omit. An unsuccessful
or absent proof chooses reference execution; it does not reject the query.

For field ordering, current core semantics check sort-field grants on all
row-visible live Rows before predicate and anchor filtering. That full-kind cost
remains for opaque policies. Materialize the snapshot under the adapter lock,
then perform the current authorization/evaluation sequence outside it. Do not
recommend a user-code authorization preflight inside the transaction.

For ID order, callbacks currently run before predicates. A skipped row can execute
a stateful callback or panic. Keep the current ID-order callback sequence and its
page stopping point for opaque policies. Do not evaluate policies once during
preflight and again during selection: the second call can change behavior.

On an explicitly eligible native path, start with one driving scalar predicate
and bounded candidate materialization. Fetch candidate Rows in ascending ID order,
release native guards, and run the common evaluator. Residual predicates and
structural equality stay in core. A superset is valid only if core checks residual
terms and skipped Rows cannot affect errors under the eligibility contract.

Do not apply a logical output LIMIT before all remaining membership, policy and
ordering checks. For this first API, materialize complete bounded candidates in
the adapter operation. Internal SQL chunks may use a continuation key within the
same transaction, but core must not call policies between chunks while a native
guard remains live. Streaming policy-driven page refill needs a separate lifecycle
design; it is not part of this proposal.

## Atomic maintenance and validation

For every changed Row, derive old/new key maps once. Compare entries and update
only changed fields. A payload change unrelated to indexed values should not
rewrite all index entries. Serialize the new full Row once and reuse those bytes
for persistence and byte accounting.

In the existing immediate transaction, update authoritative Row, key differences,
reference edges, exact counters and generation, then the rest of the bundle.
The transaction commits them together. Counter deltas preserve both native stored-text and canonical full-Row
lengths, not payload or index lengths. Creation increments the persisted Row count;
delete replaces a live Row with a tombstone and does not reduce that count.
Tombstone purge reduces it during maintenance reconstruction.

Receipts, effects and pending-work-only updates do not change the query generation.
Unchanged bundles, receipt replay, rejected transitions and rolled-back commits
must leave keys and counters unchanged. An acknowledgement lost after commit must
leave the full committed index state, and retry must not change it again.

At open, derive expected keys/counters from validated authoritative Rows and compare
with physical state. Check both directions: no missing keys and no orphan/duplicate
keys. Reject discrepancies as corruption. Do not silently repair them during open.
Use a bounded merge comparison or bounded collections under explicit validation
limits. Include derived entry count/key bytes in those limits; Resource row limits
alone do not bound index amplification.

All supported writers must use this bundle path. Startup validation does not prove
that an unrelated raw SQL writer cannot corrupt keys after open. State that topology
limit explicitly. Detecting arbitrary post-open tampering without an index scan
requires another integrity contract; candidate verification alone cannot detect
missing memberships.

Reuse this derivation routine for registration, maintenance and corruption checks.
Put implementation in cohesive `query_keys`, `query_metadata`, `query_read` and
`query_planner` modules if those responsibilities remain substantial. Keep `lib.rs`
a facade. Avoid a second implementation of scalar comparison in each adapter.

## Native formats, backup and rebuild

Current native format is 6 and the archive is format 4 with storage marker 6.
An old writer must not open a database whose query indexes it will not maintain.
Bump the native format for this change. Ordinary open must reject the old marker;
an explicit fresh-destination upgrade rebuilds keys and counters from current Rows.

The repository currently shares `rom_backup::STORAGE_FORMAT` across adapters.
For this stage, keep that convention rather than quietly introducing incompatible
per-adapter numbering: allocate the next native marker and update both adapters'
acceptance/upgrade tables. An archive marker transition should be explicit too.
Use named constants and version-specific compatibility tests; do not scatter new
number literals. The exact assigned versions belong to the implementation plan.

The logical archive need not contain query entries or counters. Under the proposed
deterministic profile, reconstruct both from descriptors and current Rows. Old
receipts, replay versions, retry epochs, references, work payloads and external blob
boundaries remain intact. A future configurable sparse profile must be included
in archive metadata or explicitly supplied by the host on restore.

All restore, schema migration, native upgrade and retention outputs use the same
builder. Validate the rebuilt structures before the existing Stage publication.
A failed or interrupted build cannot expose a partially indexed destination.
Retain source immutability and the existing rollback-journal rejection rule.
No online build is proposed for this stage. Add it only with a separate catch-up
and atomic readiness contract.

## SQLite planner bridge

Use one SQL builder for EXPLAIN and execution, with identical predicates, bindings
and batch bounds. Bind kind, field, key and ID values; never interpolate Resource
names as SQL identifiers. SQLite may choose a scan, range search or temporary sort.
Do not issue `INDEXED BY` as the selector's optimization decision.

EXPLAIN QUERY PLAN is a diagnostic interface whose output can change. Keep parsing
inside a version-pinned adapter with a strict node allowlist. Unknown versions or
nodes return no estimate. They do not make a valid query fail or establish exact
candidate coverage. Explain failure before selection permits the reference path;
selected execution failure does not.
[SQLite EXPLAIN QUERY PLAN](https://www.sqlite.org/eqp.html).

A first estimate can distinguish indexed search, scan and temporary sort. It does
not have reliable selectivity statistics for arbitrary Resource distributions.
Do not copy the prototype's uniform-range assumption as a production fact. Use
conservative estimates and report unknown costs. Prefer a reference strategy when
there is no demonstrated benefit after admission and authorization costs.

Use prepared-statement status counters to record full-scan steps, sort operations
and VM steps. VM step counts are undefined past their documented signed-32-bit
range; represent that as unavailable rather than a small wrapped measurement.
These are measurements, not admission proofs.
[SQLite statement counters](https://www.sqlite.org/c3ref/c_stmtstatus_counter.html).

`sqlite3_stmt_scanstatus` supplies loop/visit information and planner estimates,
but requires `SQLITE_ENABLE_STMT_SCANSTATUS`. Do not assume the bundled build enables
it. A later optional diagnostics build can compare estimated and observed visits.
Its estimate is rows per loop, not total query latency.
[SQLite scan status](https://www.sqlite.org/c3ref/stmt_scanstatus.html),
[scan status quantities](https://www.sqlite.org/c3ref/c_scanstat_est.html).

If `ANALYZE` or `PRAGMA optimize` is added, run it as a host maintenance operation,
not during a read-only query session. Statistics can change the physical choice,
never the semantic capability proof. Budget its work and invalidate planner caches
when statistics change.
[SQLite optimization maintenance](https://www.sqlite.org/pragma.html#pragma_optimize).

## Bounds and expected costs

SQLite defaults include 2,000 columns and 32,766 host parameters on current
versions; joins are limited to 64 tables. Builds can lower several limits. The
normalized layout uses a fixed small column set and one or a few joins rather
than one join per predicate. ROM's current 32-predicate bound does not itself
protect a naive SQL generator from large expression, parameter or work costs.
Inspect the actual bundled runtime limits. Bind values and cap generated SQL size.
[SQLite implementation limits](https://www.sqlite.org/limits.html).

Let N be live Rows and F eligible scalar fields. The default profile produces
approximately N × F logical memberships plus B-tree overhead. Wide strings repeat
key material. A changed indexed field updates its membership and secondary index;
an unchanged field should do neither. Full-kind counters add a small write per
changed Row. Indexed reads still require Row lookups and decoding.

A selective scalar predicate can reduce Row reads for ID-ordered queries.
Opaque-policy queries retain full reference materialization; field sorting also
retains O(N) authorization work under today's policy contract.
Broad predicates, low-selectivity bool fields, many scalar fields, large text and
write-heavy applications can lose overall. These are hypotheses to measure, not
performance claims.

Keep full-kind admission limits identical across strategies. Add separate bounded
internal buffers for keys/candidates/SQL. A candidate LIMIT does not bound SQLite
VM work or temporary sorting. If runtime cancellation or VM budgets are added,
define their shared error/lifecycle contract and use a progress mechanism that
cannot interrupt unrelated concurrent work on another session.

## Acceptance experiments before promotion

- Differential tests force reference/native execution for full Rows and exact
  errors across typed, projected, wire and live queries.
- Cover absent/null, all scalar extremes, positive/negative zero, text prefixes,
  embedded NUL, Unicode and custom codec normalization. Include multi-key mixed
  directions and ascending ID ties.
- Use hidden rows, visible protected-sort rows excluded by filters or anchors,
  revoked actors, no matches, tombstones and tiny output limits on oversized kinds.
- Corrupt keys, counters, profile versions and extra native objects. Reopen must
  fail before indexed reads. Test omitted kinds and two independent connections.
- Inject failure and process exit at actual key/counter writes and commit. Verify
  rollback, lost acknowledgement, replay, upgrade, migration, retention and restore.
- Measure file-backed selective, broad and no-match queries with skewed holdout
  data. Record whole-operation latency, preflight/decode cost, SQL VM work, index
  size, peak memory and commit latency. Rotate strategy order and retain raw data.
- Compare the default all-scalar profile against no indexes and a bounded sparse
  experimental profile. Do not promote all-scalar indexing solely from read speed.

The first integration is complete only when a second unrelated Resource uses this
path without custom tables, SQL, controllers or index code from its author.
