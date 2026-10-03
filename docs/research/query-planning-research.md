# Schema-driven filtering, sorting and backend query planning

Research date: 2026-10-03. ROM source inspected at `64240cf99e4632378e62db91f3bf151f2d306e6f`.
This note owns research and proposed experiments only. No query prototype,
third-party crate, benchmark or database translation was implemented or executed
for this report. Source inspection and official documentation are the evidence.
The owner accepts **moving views** and requests better filters/sorting and real
comparative experiments. Aggregates, reporting and joins are outside this step;
compensation is not part of the query work.

## Recommendation and decision status

Keep one canonical, schema-validated ROM query representation for typed Rust,
wire and live queries. Compare generated convenience methods, generic typed
field methods and direct runtime AST construction against that same evaluator.
A provisional **hybrid** is most compatible with the existing Resource model:
derive emits field bindings, ordinary Rust traits expose applicable operations,
and runtime compilation resolves the accepted descriptor/codecs and current
permissions before an adapter translates an authorized plan. This is an
engineering recommendation, not a measured winner or an owner-selected crate.

Treat authoring strategy, SQL construction, and physical execution as separate
experimental variables. Generating Rust methods does not itself create indexes,
remove a scan, or authorize a row. A fast SQL renderer does not establish exact
numeric comparison, privacy, or database planner behavior.

## What ROM actually does today

- [QuerySpec](../../crates/rom/src/query_spec.rs) stores conjunctions of equality
  predicates, optional `after_id`, and a result limit. Each predicate passes
  descriptor lookup, query permission, shape validation and its actual field
  codec before storage access. The selector then obtains a bounded whole-kind
  snapshot, checks row identity/byte limits, sorts IDs, applies row policy and
  predicates, and takes the requested limit.
- [Resource/Field](../../crates/rom/src/resource.rs) already supplies typed
  `FieldRef<R,T>`, `Query<R>::all`, descriptors and `normalize_field`.
  [Derive](../../crates/rom-derive/src/lib.rs) generates field bindings and codec
  dispatch from one field declaration. `FieldRef::new` is public for handwritten
  definitions: even typed helpers cannot replace runtime validation of a field's
  accepted name and type.
- [Presence](../../crates/rom/src/patch.rs) distinguishes an absent field from
  `Option<T>`'s explicit null. Finite floats, signed and unsigned integers, custom
  canonical codecs, references and container shapes already exist. Their meaning
  must survive a richer query path.
- [Storage](../../crates/rom/src/persistence.rs) has bounded `snapshot`, not a
  query-plan interface. [SQLite](../../crates/rom-sqlite/src/lib.rs) stores a
  serialized complete `Row` in `resources.data` and selects a kind in ID order;
  application fields live below `value`. [redb](../../crates/rom-redb/src/lib.rs)
  scans a `(kind,id)` key range and decodes row JSON. Neither implementation has
  maintained secondary field indexes.
- [observe](../../crates/rom/src/execution.rs) runs under supervised I/O and the
  commit gate, rechecks current authority and validates the handoff generation.
  [Projection](../../crates/rom/src/projection.rs) separately applies field
  disclosure rules. [Documented query bounds](../queries.md) explicitly say that
  page limits do not make an oversized candidate snapshot fit.

These are observed implementation properties, not benchmark results.

## Relevant first-party query approaches

Documentation versions below identify the pages inspected, not dependencies
selected for ROM. Moving `latest`/`2.3.x` pages can change after this review.

| Approach | Checked behavior | Implication for ROM |
| --- | --- | --- |
| SeaQuery, API docs 1.0.2 | Builds expression/condition ASTs at runtime and renders dialect SQL plus bound values. Identifier derives are available. Its documented backends include SQLite, PostgreSQL and MySQL. `build` returns SQL and values separately; `to_string` embeds values for display. [Official crate documentation](https://docs.rs/sea-query/1.0.2/sea_query/) | A candidate **adapter implementation**, not the core schema or policy model. It could reduce SQL construction work without importing a complete ORM. Generated identifiers do not prove operand compatibility with ROM codecs. Never send displayed SQL with literal user values as the execution path. |
| Diesel, official 2.3.x docs showing 2.3.13 | Typed query DSL composes filters and ordering. `into_boxed` preserves one type across conditional construction, with a specified backend. A boxed query cannot be reused across different backends. [QueryDsl](https://docs.diesel.rs/2.3.x/diesel/query_dsl/trait.QueryDsl.html#method.into_boxed), [application composition guide](https://diesel.rs/guides/composing-applications/) | Useful comparison for compile-time operand diagnostics and dynamic composition costs. Adopting a SQL-table DSL as ROM's public contract would impose another schema vocabulary and exclude redb. Do not equate typed construction with precomputed SQL or measured speed. |
| SeaORM, official 2.0.x documentation | Generated `COLUMN` members expose type-specific methods, while the older `Column` enum accepts broader inputs. Select builders compose `filter` and `order_by_*`; cursor examples include multiple columns. [Select and typed columns](https://www.sea-ql.org/SeaORM/docs/basic-crud/select/) | Direct precedent for generated typed field conveniences around dynamic queries. Copy the ergonomic lesson; do not replace Resources with a second entity/active-model hierarchy merely to obtain filtering. |
| SQLx, API docs 0.9.0 | `query!` requires literal SQL and a build-time schema connection or offline metadata, and binds checking to a database type. Runtime `QueryBuilder` uses `push_bind` for values; raw `push` does not sanitize input and constructed SQL is not syntax-checked by that builder. [query! requirements](https://docs.rs/sqlx/0.9.0/sqlx/macro.query.html), [QueryBuilder](https://docs.rs/sqlx/0.9.0/sqlx/struct.QueryBuilder.html) | Fixed generated SQL variants and runtime query construction are different profiles. Arbitrary dynamic filter trees cannot all become one checked literal. An adapter may use bound runtime construction, but ROM still owns validation and semantics. No need to replace the existing rusqlite driver just to conduct the comparison. |

SeaQuery also exposes null-placement ordering methods; these are renderer
features, not a proof that ROM's missing/null semantics are preserved.
[OrderedStatement](https://docs.rs/sea-query/1.0.2/sea_query/query/trait.OrderedStatement.html)
SeaORM's cursor interface is a useful keyset example, but its existence does not
make a mutable ROM sort field a snapshot or immutable traversal key.
[Cursor API](https://docs.rs/sea-orm/latest/sea_orm/struct.Cursor.html)

## Semantic contract to freeze before comparing implementations

The following are proposed defaults to test, not changes to the current release.

| Area | Proposed portable meaning and necessary controls |
| --- | --- |
| Field identity | Resolve public names against the frozen Resource descriptor. Typed helpers and strict wire AST compile to the same accepted field identity; client strings never become raw SQL identifiers or expression fragments. Bound names/values/AST depth/node count/order terms/list operands before expensive work. |
| Operations | Start with equality/inequality, scalar ranges, bounded membership, presence/null tests, bounded AND/OR, and multi-field ordering. Existing equality/conjunction input must retain its meaning or receive an explicit version rejection. Unsupported operations fail before scanning, including empty datasets. |
| Missing and null | Use three distinct states: Missing, Null, Value. Equality to null means explicit null only; missing is tested separately. Ranges require a non-null present value. Proposed `ne null` means present non-null, excluding Missing. Do not implement generic NOT by assuming SQL three-valued logic equals a Rust Boolean complement. |
| Default order | Compare `(state_rank, scalar_value)`, with Missing < Null < Value for ascending and its reverse for descending. Append Resource ID ascending as a final deterministic tie-breaker. Expose these rules; backend defaults must not choose them. Alternative sentinel-placement options can follow only with the same reference evaluator. |
| Unsigned/signed integers | Compare `u64` and `i64` exactly within their declared field type. Do not cast through `f64` or reinterpret large unsigned values as signed. Test 2^53±1, i64 extrema, i64::MAX+1 as u64, and u64::MAX. No cross-field-type numeric coercion is needed. |
| Finite floats | Preserve the existing finite-f64 codec contract; it is not arbitrary-precision decimal. Order finite numeric values, treat -0 and +0 as equal for sorting and use ID to break ties. Test adjacent representable floats, subnormals, extrema, and JSON integer operands normalized by the actual float codec. |
| Strings and references | Use canonical encoded UTF-8 byte ordering and exact equality; no locale, case folding, Unicode normalization or target-resource dereference is implicit. Test non-ASCII, combining characters, embedded NUL, trailing spaces, quotes and unusual field names. |
| Enums and containers | Equality/membership use declared canonical values. If enum sorting is offered, choose documented encoded-string ordering rather than silently depending on declaration position. No numeric range on Boolean/enum, no list/map ordering, and no arbitrary nested path traversal in the first profile. |
| Text matching | If included, expose literal case-sensitive operations rather than user SQL LIKE patterns. Escape or avoid `%`/`_` wildcard semantics. Do not run a substring through a codec that expects a complete validated email/code; define fragment operands separately, or withhold text operations from that custom field until explicitly supported. |
| Custom fields | A wire `Shape::String` does not prove that the author's desired domain ordering is lexical. Define initial operations on canonical encoded values, and require explicit capability participation where different semantics are intended. Generated methods should rely on Rust traits to resolve aliases/custom types, not guess from syntax spelling. |

SQLite explains that `json_extract` returns SQL values, including SQL NULL for
both absent paths and JSON null; `json_type` distinguishes absent paths from the
text result `'null'`. Direct extraction therefore needs a separate presence/type
expression. [SQLite JSON functions](https://www.sqlite.org/json1.html#jex)
SQLite integer storage is signed 64-bit, and its affinity/conversion and storage
class ordering rules are not ROM's type system. BINARY, NOCASE and RTRIM are
different collations; NOCASE is not full Unicode folding.
[SQLite datatypes and collations](https://www.sqlite.org/datatype3.html)
Binary64 has limited precision: moving exact large integer comparisons through
REAL is not an acceptable general translation. [SQLite floating-point limits](https://www.sqlite.org/floatingpoint.html)
SQLite's default LIKE behavior also differs across ASCII and non-ASCII case
pairs, so adding `COLLATE BINARY` is not a substitute for specifying a literal
text predicate. [SQLite LIKE semantics](https://www.sqlite.org/lang_expr.html#like)

A safe first adapter may deliberately decline pushdown for an exact type/operator
pair. A later materialized scalar index can store canonical presence/type/order
keys, updated atomically with the Resource bundle. That requires an explicit
index/version/rebuild contract; it is not an unannounced storage-format change.
A Rust SQLite UDF can provide an exact comparator, but emitting such a function
in WHERE does not establish index use or bounded physical scanning.

## Moving keysets and authorization

For an order `(priority DESC, title ASC, id ASC)`, the continuation predicate is a
lexicographic disjunction: priority below the anchor, or equal priority and title
above it, or both equal and ID above it. Sentinel-state comparisons are part of
each term. Reusing `after_id` alone is incorrect once another field determines
order. A client anchor contains the values observed when that page was emitted;
reloading its row later would move the anchor if that row changed or disappeared.

A continuation must identify the kind, normalized filters, effective order,
query-contract/schema version and exact anchor values/ID. Reject mismatched or
malformed continuations and reauthorize each call. An integrity-protected token
is not an authorization grant; a merely encoded or signed token is not encrypted.
No token may contain a forbidden row ID or undisclosed field value. If plain sort
values cannot be disclosed, prefer rejecting that sort in the first profile;
encrypted/server-held continuations are a separate implementation choice.

A tie-breaker makes each page deterministic at its observation point, not across
mutations. Insertions before an anchor can be missed. Changing a sort value can
move a previously returned row after the anchor and produce a repeat, or move an
unseen row before it and omit it. Deletion does not invalidate a value-based
anchor. Live queries recompute the current ordered page under current authority;
they are not durable event cursors or repeatable snapshots. These consequences
are compatible with the owner's accepted moving-view premise and must appear in
API/CLI documentation and tests.

Filtering authorization already precedes data access. Sorting needs an explicit
pre-scan grant too: relative rank reveals information even if projection hides
the sorted field. Do not infer this grant from discoverability or ordinary row
read permission. A proposed usage-aware field policy can distinguish predicate
use, ordering and cursor-value disclosure; a smaller initial implementation can
require both an explicit sorting grant and readable sort values on returned rows.
Existing field-projection checks remain authoritative and must cover absent
fields as well as present values.

## Pushdown that remains correct under row policy and budgets

The native row policy is arbitrary Rust over Actor and Resource. Its source is
not a SQL expression. Translate only the canonical filter/order subset that an
adapter implements exactly, and retain final row authorization in core.

1. Validate and authorize the whole query before adapter work. Normalize complete
   value operands through the accepted codec; bind all SQL values. SQL/JSON path
   construction must handle descriptor names containing dots, quotes and brackets
   without reinterpreting them as client-defined paths.
2. Separate semantic support, execution strategy and index availability. An
   adapter reports which canonical predicates/order it executes exactly; core
   evaluates the residual. Pushing a conjunct is safe only if it cannot exclude
   a true match. Pushing one branch of OR as the whole condition is unsound.
3. Never take the client result limit before residual predicates and row policy.
   With ten denied rows followed by a readable row, SQL LIMIT 10 must not become
   an empty final result. Refill ordered candidate batches until enough authorized
   matches, proven exhaustion, or a declared work limit is reached.
4. Charge full inspected candidate rows/bytes across all batches, including rows
   rejected by residual predicates or authorization. Separately bound response
   bytes and retained sort memory. When the required answer cannot be established
   within the work bound, return an explicit error rather than a truncated page
   or false end-of-results. Never publish the last denied candidate as a cursor.
5. Ordered refill is valid only when candidate ordering is exact. A backend's
   partial order followed by core sorting requires collecting the entire bounded
   candidate set before taking the page. A top-k heap bounds retained sort memory,
   not necessarily inspected rows or elapsed database work.
6. Keep each observation's candidate reads coherent under the existing commit
   gate/current-authority and handoff checks. Later pages deliberately observe
   later states; do not require a long-lived cross-request transaction.

The current whole-kind snapshot bound and a new candidate-execution budget have
different behavior. A selective query succeeding on a large kind is an intended
new profile only if specified and tested; silently changing the old bound is not
an optimization with identical failure semantics. Hidden rows can also affect
work exhaustion and timing. Current bounded snapshots already expose coarse
capacity effects. Do not claim noninterference: removing that signal would need
a stronger policy-aware storage or budget contract, not just redacted cursors.

Database work is not bounded merely because few rows cross the adapter interface.
An unindexed predicate or sort can inspect many rows internally. Measure SQLite
VM work/progress or a finite execution budget as well as decoded rows. Use actual
EXPLAIN QUERY PLAN evidence to distinguish SEARCH, SCAN and temporary sorting;
SQLite cautions that its diagnostic output format can change, so assertions
should target relevant properties on the pinned engine rather than snapshot an
entire display. [SQLite EXPLAIN QUERY PLAN](https://www.sqlite.org/eqp.html)
Order/filter alignment with indexes is a separate physical-planning concern.
[SQLite optimizer overview](https://www.sqlite.org/optoverview.html)

## Proposed experiments: none executed in this note

Run isolated local experiments on native Rust 1.99 in `rom-dev`, using existing
SQLite/redb adapters and task-owned persistent targets. Record source revision,
lockfile, compiler, features, engine version, dataset seed, query cases and raw
measurements. Pin any trial dependency before execution; this research does not
select one.

### Experiment A: authoring strategy, identical executor

Implement the same two unrelated Resources, renamed fields, an alias, a custom
canonical scalar, optional-nullable field and typed reference using three inputs:

- **Generated:** derive emits resource-specific operation conveniences; an
  explicit full-generation variant may emit per-field evaluation dispatch.
- **Hybrid:** derive emits only typed field bindings; generic capability methods
  build a shared canonical AST, normalized/compiled once per request.
- **Runtime:** accepted descriptors validate public names/operators/value nodes
  from a strict runtime AST, with no generated query convenience layer.

Feed identical normalized queries to one evaluator and one adapter execution
path. Compare generated token/source volume, clean and incremental compile times,
release binary size, query construction/normalization time and allocations,
per-candidate evaluation cost, and diagnostics for wrong value type, unsupported
operation, field rename, invalid custom value and malformed wire input. Use
separate persistent build directories for honest cold/warm comparisons, but cap
their disk cost and retain raw timings. Compile-time measurements must distinguish
Rust dependencies from the changed application fixture.

Use deterministic loops with consumed results, repeated samples and randomized
case order; report distribution and workload size, not a single best run.
Generated dispatch must be genuinely different in the experiment if claiming to
measure generated evaluation: three wrappers calling the same generic evaluator
only compare authoring/construction costs. Conversely, keep the executor identical
when isolating ergonomic overhead. A manual Resource without the derive must
remain a positive control. Compiler fixtures are not a human usability study.

### Experiment B: canonical semantics versus physical translation

Use one independent Rust reference evaluator for expected membership/order and
literal fixtures with known IDs. Compare whole-snapshot baseline, exact candidate
pushdown, and indexed seek only if an actual index is implemented in the trial.
Run against both SQLite and redb; unsupported adapter capabilities must be visible
rather than approximated. Optionally compare SeaQuery with a small handwritten
bound SQL renderer while holding the SQL semantics/schema/driver constant; this
isolates construction cost from plan quality.

Datasets: small baseline plus 10k/100k rows; selective and broad filters; uniform
and skewed distributions; duplicate sort values; no matches; all denied; 1%/50%/
100% readable; one oversized row; deleted rows. Record latency distributions,
normalized-plan construction time, candidate/decoded/authorized counts, serialized
bytes, SQLite VM/plan evidence, redb iterator steps, allocations, and concurrent
write latency while observation holds the gate. Efficient translation must reduce
measured work on at least a supported selective or seek case; merely adding WHERE
while continuing to decode the whole kind is not that result.

Adversarial vectors include exact integer/float boundaries; Missing versus Null;
empty string/false/zero; Unicode/NUL; canonical custom values; literal `%`/`_`;
field names with JSON-path punctuation; mixed direction ties; null/missing order;
forbidden predicates and sort fields on empty data; expired/revoked actors;
filters whose pushed and residual components interact through OR; budget exactly
at and one below the next candidate; and denied candidates at page boundaries.
Compare typed, wire, ordinary and live results through the shared semantic path.

### Experiment C: moving continuation under mutation

Read a page, then insert before/after the anchor, delete the anchor, update sort
values across it, revoke field access and change filters/order while reusing the
continuation. Verify documented moving-view repeats/omissions, exact tie behavior,
rejection of mismatched cursors, no hidden sort values/IDs, and current authority
on each page. Include an empty visible result with remaining denied candidates;
absence of a visible row must not accidentally certify scan exhaustion.

## Adoption gate

Advance the smallest implementation that passes exact semantic/auth/cursor
conformance and produces a measured benefit. A pleasant hybrid interface is the
starting recommendation; generated dispatch or a SQL construction crate earns
adoption only if its measured advantage justifies code size, compile time,
dependencies and maintenance. Keep drivers/transports outside core. Publish each
adapter's supported operations, ordering, index and work-budget profile without
claiming universal database behavior. No aggregate/reporting API is implied.
