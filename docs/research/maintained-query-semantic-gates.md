# Maintained query semantic gates

Date: 2026-10-03. Baseline: `383b03edd0ce9876b9984c6fd0bfff776f5bdb24`.
This is a source audit and a proposed interface contract for release stage 3.
It does not implement indexes, select a cost model, or claim an index speedup.
The existing bounded-kind query profile remains the compatibility requirement.

The audit covers [selection](../../crates/rom/src/query_eval.rs),
[definitions](../../crates/rom/src/resource.rs),
[storage](../../crates/rom/src/persistence.rs),
[typed queries](../../crates/rom/src/query.rs),
[projection](../../crates/rom/src/projection.rs), and the runtime observation path.
The concurrent module refactor moves implementation files but does not change the
baseline functions described below. Prototype results in
[the selector review](query-selector-independent-review.md) do not establish these
maintained policy and codec guarantees.

## Recommendation

Keep one core evaluator and add a coherent native read session beneath it. The
core decides which rows an adapter may omit. The adapter chooses a physical path
only within that permission. Exact admission and optional cost estimates are
separate inputs. Neither a cheap estimate nor a small result limit grants
permission to omit a policy call.

Keep existing opaque policies on the exact reference path. Add an explicit
actor-only read-policy contract for applications that want index filtering to
skip nonmatching rows. Combine that contract with an explicit uniform field grant
when an index supplies ordered candidates. Do not inspect function addresses,
sample callbacks, infer purity from a closure body, or treat a descriptor as proof
that a custom decoder cannot fail.

## Observed evaluation order

The following order describes current successful observation attempts. A failed
step stops later steps. `observe` can repeat a successful attempt when the managed
mutation generation changes before delivery.

| Phase | Current behavior | Requirement for a native path |
| --- | --- | --- |
| Observation entry | Check actor, lifecycle and I/O capacity. Under the runtime gate, check current authority. | Keep admission/lifecycle errors and actor-gate checks in the core. Do not cache actor authority in an index. |
| Request bounds | Check the result-limit range, then serialized request size, then Resource registration. | Preserve ordering; an index must not make an invalid limit valid. |
| Predicate/sort grants | Call query policy for equality filters, then comparisons, in supplied order. Call sort policy for each order term. These checks run on empty kinds. | Keep grants before descriptor/operand normalization and before query storage access. No database lookup can establish these grants. |
| Normalization | Enforce AST limits, descriptor fields, actual field codecs, shapes, operators, sort uniqueness and anchor binding. Normalize filters before comparisons, then validate order and anchor. | Use the same core compiler and canonical values. Do not substitute a SQL cast or a second normalizer. |
| Whole-kind acquisition | `snapshot` rejects excess rows/encoded bytes without truncation. The core then checks kind identity, serialized full-Row bytes, ID ordering and duplicate IDs. Tombstones and protected metadata count. | Admission covers the entire kind before row policy or result limiting. Exact transactional metadata can replace enumeration only with an integrity contract. |
| ID boundary | With no explicit sort, skip IDs at/before `after_id` or the anchor ID before inspecting row values or calling row policy. | A native ID lower bound is safe after whole-kind admission. It must use the same byte order and strict boundary. |
| Row gate | Skip tombstones. `Registered::allows` first calls `R::decode`; a returned decode error hides the row. A decoder/policy panic remains observable. Denied rows are skipped. | An opaque read policy must run on every live row reached by the reference walk, including rows that later fail predicates. |
| Sort-field gate | For each readable row, check field-read access for each explicit sort field before predicates and anchors. Each check also decodes the Resource. Denial returns `Denied`, even on a nonmatching row. | Sorted candidate pruning cannot bypass this whole-kind preflight unless an explicit policy contract makes it unnecessary. |
| Predicates | Evaluate equality filters before comparisons, with conjunction short circuit. Missing/null/container/numeric rules are core rules. | Predicate reordering can change which malformed value produces `Storage`. Do not reorder fallible reference evaluation without proof. |
| Sort keys/anchor | Validate keys for matching rows, then compare against the moving anchor. Keys are not extracted from predicate-rejected rows. | A key validation pass over every row can introduce errors the reference path does not produce. |
| Page selection | ID order stops after enough admitted matches. Explicit sort evaluates the whole relevant walk, then sorts by requested keys and final ID ascending, then truncates. | Do not evaluate opaque suffix policies after an ID page fills. Do not apply a backend LIMIT before row authorization. |
| Disclosure | Project selected rows under current authority, row policy and field policy. Typed results additionally require every field and decode to the requested Resource. | Keep projection and complete typed disclosure in core. Candidate keys alone are never an authorized response. |
| Observation exit | Recheck current authority after success, then actor/lifecycle/generation at handoff. Live queries use the same path. | Preserve rechecks and bounded retry behavior. A session is not a client-retained snapshot. |

`Registered::allows` and `allows_field` decode the complete Resource even if their
stored policy function always returns true. `allow_all_fields()` currently stores
ordinary functions; it carries no separate proof marker. Resource decoding can
call custom field code. Therefore its name alone does not make existing queries
safe for arbitrary candidate pruning.

A panic in the current observation closure becomes `Error::Panicked` and marks
the runtime terminal through supervised I/O. It is not equivalent to a denied row
or an optional planner failure. Preserving only successful result sets is not
sufficient compatibility.

## Eligibility matrix

Here, “opaque” means the current row/field callback contract, including its
Resource decode. “Explicit read grant” means the proposed actor-only contract
below, not an optimization inferred from an existing callback.

| Selection contract | ID-ordered query | Explicitly sorted query | Honest cost statement |
| --- | --- | --- | --- |
| Existing opaque row and field callbacks | Preserve the ascending-ID walk, ID-boundary skip and page short circuit. Do not push predicates ahead of row policy. | Preserve row policy and sort-field checks over the same ID walk before filtering. A separate reordered preflight must not change which error occurs first. | A full-kind admission proof can avoid payload work outside the ID page, but predicate indexes do not remove the required opaque authorization walk. |
| Explicit read grant; opaque field callbacks | Predicate candidates in ID order can be safe under the new contract. Only selected rows need final field disclosure. | Retain the field-policy preflight over readable rows; opaque field callbacks on excluded rows can deny or panic. | Candidate pruning can benefit ID pages. Sorted pruning still has a preflight cost. |
| Explicit read grant; tracked unconditional read-field grant | Native predicate candidates and native ID boundaries can be safe. | Native predicate/order/anchor candidates can be safe when comparator and index integrity are exact. | Admission can use exact kind metadata; candidate reads need not enumerate the full kind. This is the intended useful index profile. |
| Unsupported shape, ordering, codec contract or adapter path | Use the same bounded reference evaluator before starting candidate execution. | Same. | Report fallback rather than inventing an approximate interpretation. |

An implementation may initially use reference selection for both opaque rows in
the first matrix row. It need not build an elaborate permission DSL or optimize a
required authorization scan merely to claim a native plan was selected.

## Smallest explicit policy contract

A proposed authoring form is `.read_policy(fn(&Actor) -> bool)`. The name is
provisional. It grants or denies Resource reads without a Resource value. Existing
`.policy(fn(&Actor, Access, &R) -> bool)` continues to control its existing path;
write policy remains mandatory and independent.

The new contract must define all of the following:

- Its query read decision is row independent. Evaluate it in a documented core
  phase after whole-kind admission, including the documented empty-kind case.
  Callback failure/panic is not a planning hint. Recheck current disclosure as the
  observation contract requires.
- It expressly permits excluding nonmatching rows without decoding their
  Resources. Returned rows still receive Resource decoding and current disclosure.
  This is an explicit opt-in interpretation, not a claim that arbitrary existing
  decoders have become total or pure.
- Keep a separate marker for the unconditional field grant installed by
  `allow_all_fields()`. A later `field_policy(...)` clears that marker. Query and
  sort grant callbacks still run before normalization; their existence does not
  prove row-field access. Builder order must have explicit semantics. A later
  opaque `.policy(...)` should clear the actor-only read mode rather than leave
  a stale optimization permission.
- Opaque custom codecs remain a separate concern. Descriptor shape only certifies
  representation. The implementation must either use the explicit new selection
  semantics above, or require an additional trusted codec-totality contract.
  Do not silently elide `R::decode` in the old policy path.

This keeps ordinary authoring short while making the optimization permission
visible. It avoids asking every application to express policy as a query AST.
Tests must compare an opaque definition to its old oracle, and an opted-in
definition to its newly documented oracle. They must not hide a changed failure
contract inside a performance flag.

## Coherent adapter interface requirements

Prefer an owned request/result interface. Storage owns the coherent native read
transaction internally; no application callback runs under an adapter lock. The
following names are proposals, not implemented public types:

```rust
fn query_read(&self, request: &StorageQuery, bounds: QueryBounds)
    -> Result<QueryRead>;

// StorageQuery carries a core-normalized query, accepted descriptor binding,
// and selection permission derived from the definition, never from a cost hint.
enum SelectionMode {
    ReferenceOnly,
    UniformReadAndFields,
}

enum QueryRead {
    Reference { rows: Vec<Row> },
    NativeCandidates {
        rows: Vec<Row>,
        admission: KindAdmission,
        binding: ReadBinding,
    },
}
```

A default implementation calls the existing bounded snapshot once and returns
reference rows. A native implementation opens one transaction, reads exact
admission and index availability, invokes a pure shared core path selector, reads
complete Rows through the chosen path, then closes the transaction before return.
The core validates the returned contract and runs its evaluator and disclosure
outside native guards. Exact admission includes whole-kind row count and complete
Row bytes; the binding identifies the accepted schema, normalized request and
coherent session used by the native result. Binding must be native/core metadata,
not a token supplied by an untrusted query client.

This is materially smaller than exposing a borrowed transaction or callback
session. It preserves object safety for `dyn Storage`, avoids self-referential
SQLite transaction storage, and avoids application panic or authority reentry
under a connection mutex. Selection code, comparator semantics and the pure cost
selector remain core-owned; native SQL/table traversal remains adapter-owned.

Determine selection eligibility from declaration metadata before storage, but
prefer evaluating the actor-only read decision after whole-kind admission. An
earlier false result or panic could otherwise hide `TooLarge`. Eligibility does
not require the callback's value: it establishes row independence, not permission
to disclose a result. If the new authoring contract deliberately chooses an
earlier global grant check, document that distinct empty-kind/error behavior
instead of claiming compatibility with an old opaque row callback.

Initially return the complete predicate/anchor candidate set, without a native
page LIMIT. Core still sorts and truncates. This gives selective indexes a useful
path while preserving bounded memory and avoiding premature truncation before
residual checks. A native limited page requires a stronger exact membership/order
contract and explicit selected-row-only codec behavior. It is a later optimization,
not a prerequisite for the first maintained index path.

Session requirements:

1. Bind exact admission, schema/index availability, optional estimates, chosen
   plan and candidate reads to the same native transaction or materialized
   snapshot. A process-local mutation counter alone is not this proof.
2. Core decides permitted selection mode from the accepted definition and current
   query contract. The adapter cannot upgrade that permission or call application
   authorization itself. Use a canonical query representation, not client SQL.
3. Whole-kind count and bytes are authoritative admission facts, not estimates.
   Preserve overflow errors even when all predicates miss, the page is one row,
   or an ID boundary is past the last row. Include tombstones and full protected
   metadata. Charge new physical work against separate execution budgets without
   changing which logical queries satisfy the current public bounds.
4. Exact maintained metadata and index completeness must be updated atomically
   with native row changes. Open, restore, migration, retention and old-format
   upgrade must validate or rebuild them. Restore must not publish indexes based
   on pre-conversion values. Query hints must never repair integrity silently.
5. Statistics are optional ranking hints bound to this session and canonical
   query. Missing/stale/overflowing estimates exclude that estimate, not a query
   error. An optional planner failure can choose the reference path before
   execution. Storage/integrity errors and execution errors must remain errors;
   do not retry a new snapshot after a partially executed candidate path.
6. A native candidate stream must have exact or conservatively complete membership
   for the admitted route. No false negatives, duplicate IDs or wrong-kind rows.
   A final core filter can remove false positives, but cannot recover omitted
   authorized rows. Requested LIMIT is not a candidate cap when core residual
   filters or authority can reject candidates; stream until enough valid results
   or exhaustion, within the admitted kind.
7. Never hold the SQLite connection mutex while calling existing
   `project_outcome`/`check_authority`: actor gates call Storage.load and can
   reenter the same adapter. The smallest integration keeps the session scoped to
   admission and selection, ends it before existing projection, and retains the
   outer runtime gate and authority rechecks. A broader session needs explicit
   session-routed authorization reads rather than hidden reentry.
8. Calling opaque policy code under a native mutex can newly poison that mutex on
   panic. Preserve the existing terminal `Panicked` outcome without introducing
   adapter damage. For example, defer unwinding until native guards are released,
   or evaluate materialized reference rows outside the native lock. Do not convert
   a policy panic to an ordinary nonterminal planner error.

The existing supported ownership contract is one ROM owner per adapter. The new
session must not claim cross-process actor revocation isolation or writer
coordination that the current runtime does not provide. A moving anchor remains
a value boundary; no transaction is retained across requests or live refreshes.

## Exact comparison and integrity requirements

Use the existing missing < null < value order for ascending keys and reverse that
order for descending keys. Append ID ascending even after descending terms.
Keep full-range u64/i64 comparisons exact, finite-f64 semantics including equal
negative and positive zero, and encoded UTF-8 byte order for strings/enums/refs.
References compare IDs; they do not authorize or load target Resources. Container
equality remains structural; container ordering is unsupported. The normalized
field codec remains authoritative for query operands and anchors.

Maintained counters do not by themselves prove that every stored Row has a valid
kind/ID, that stored IDs are unique, or that an index has no missing entries.
Document the native integrity invariant and test broken inventories, duplicate
candidate output, stale keys and missing index entries. Startup validation and
atomic writes can establish the supported persisted-state invariant. They must
not be described as detecting every physical corruption in an unread payload on
every query. Fault adapters lacking the required evidence use the reference path
or fail closed; they do not get an optimization capability by returning a count.

## Characterization evidence and required tests

Three regressions were added to the existing
[query planning suite](../../tests/persistence/tests/query_planning.rs). They use
public APIs and run against both SQLite and redb without implementing a feature:

- `sorted_field_denial_precedes_predicates_anchor_and_page_limit`: a readable row
  outside the predicate/page still denies sort access; rows excluded by the anchor
  still require sort-field permission.
- `id_page_short_circuit_does_not_evaluate_later_opaque_policy`: a later policy
  panic is not evaluated after an ID page fills, but occurs once its row is reached.
- `opaque_row_policy_runs_before_a_nonmatching_predicate_can_skip_the_row`: a
  predicate index cannot hide an earlier opaque-policy panic on a nonmatching row.

Existing maintained cases cover default/empty sort denial, moving anchors,
missing/null/ranges, exact numeric extrema, custom operand normalization, wire /
typed / live parity, and whole-kind overflow despite a one-row page.

Before enabling native candidates, add differential cases for a hidden earlier
row, a nonmatching decoder panic, multiple fallible predicates in reversed input
order, permission overrides after an explicit grant, empty-kind uniform read
policy, authority changes at handoff, and planner-versus-execution failure.
Instrument adapters to prove one admission/candidate snapshot, exact plan binding,
no fallback after execution starts, and actual row/index visit counts. Run the
same vectors with and without indexes. Report authorization-walk costs separately
from native candidate work. A fast plan label alone is not performance evidence.

Verification: `cargo test -p rom-storage-conformance --test query_planning
--locked --offline` passed all **9 tests**, each backend-sensitive case running
against both SQLite and redb. The three additions characterize existing behavior;
they do not require the proposed interface. The run used Rust 1.99.0 in the
`rom-dev` environment, two build jobs, and separate target
`/var/tmp/rom-query-gates-target`. Log: `/var/tmp/rom-query-semantic-gates.log`
inside that environment. No full workspace check or performance experiment was
run for this audit.
