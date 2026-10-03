# Shared query semantics

Typed selectors, projected queries and live queries share one normalized plan and bounded evaluator. Supported operations are conjunctive equality/absence, scalar `eq`, `ne`, `lt`, `le`, `gt`, `ge`, up to four sort fields, and moving keyset pages. Both maintained adapters use this same evaluator over a bounded snapshot; the experimental SQL/index comparisons do not change the maintained storage format or imply indexed execution here.

```rust,ignore
let query = Inventory::quantity_field().at_least(100)
    .and_where(Inventory::code_field().not_equals("retired".into()))?
    .order_by(Inventory::quantity_field(), Direction::Desc)
    .order_by(Inventory::code_field(), Direction::Asc)
    .limit(20);
let first = runtime.query(&actor, &query).await?;
if let Some(last) = first.last() {
    let next = query.after_snapshot(last)?;
    let second = runtime.query(&actor, &next).await?;
}
```

`FieldRef` provides `equals`, `not_equals`, `less_than`, `at_most`, `greater_than`, `at_least` and explicit `compare(CompareOp, value)`. Values are field-typed, while range/sort compatibility with the registered shape is checked at runtime; containers therefore compile with these generic helpers and are rejected on evaluation. Existing `.and(field, value)` adds equality. `.and_where(other_query)?` accepts predicates only and rejects any nested order, limit or continuation, so no nested configuration silently disappears. `Query::<R>::all()`/`default()` and `QuerySpec::all()` select without predicates.

The equivalent wire shape is accepted by generic `/query` and `/live`, and by the CLI's query file:

```json
{"kind":"inventory","query":{"comparisons":[{"field":"quantity","op":"ge","value":100},{"field":"code","op":"ne","value":"retired"}],"order":[{"field":"quantity","direction":"desc"},{"field":"code","direction":"asc"}],"limit":20}}
```

A CLI query file contains the inner `query` object. Existing `filters:[{"field":"done","value":false}]` and single-equality envelopes remain valid. Dynamic code uses `QuerySpec::all().compare("quantity", CompareOp::Ge, json!(100)).order_by("quantity", Direction::Desc)`. Unknown properties/operators, duplicate sort fields, more than 32 combined predicates or four order fields, and oversized encoded queries are rejected. A result limit must be one through the configured snapshot row limit.

## Canonical values and ordering

Operands pass the accepted field shape and its actual `Resource::normalize_field` codec before evaluation. JSON `0` on a finite-float field matches canonical `0.0`; a trimming field normalizes operands just as it normalizes writes. Integers compare exactly according to their declared signed/unsigned shape, without converting unsigned values through float or signed storage. Finite floats use numeric comparison, including equal positive/negative zero. There is no cross-field numeric coercion.

Ascending order is **missing, null, value**; descending reverses it. Strings, references and enum values compare by UTF-8 bytes without locale collation, case folding or Unicode normalization. Booleans order false before true. Resource ID is always the final ascending tie-breaker. Lists/maps retain structural equality but cannot be used for range comparisons or sorting.

A range comparison only matches present non-null values and rejects a null operand. Equality to null matches only present null. `ne` to a present value excludes missing fields; present null differs from a non-null value. Optional-field absence uses `filters:[{"field":"memo","value":null,"absent":true}]` or `QuerySpec::absent("memo")`. Typed `Presence::Missing` comparisons encode explicit `absent:true`: `eq` matches missing and `ne` matches present; absence is rejected for ranges and non-optional fields.

## Authority and moving pages

Every predicate requires `query_policy` permission before any rows are consulted. Ordering separately requires default-denied `Definition::sort_policy`; `allow_all_fields()` explicitly grants field reads, predicate use and ordering. Even after a sort grant, every row-readable candidate must grant read access to each ordered field, or the query returns `Denied` before filtering/pagination. This prevents hidden values influencing observable ranks. Row-hidden resources never supply sort keys. Complete typed results still require every field grant; projected results may omit other protected fields. Live refreshes perform the same current checks.

Without requested ordering, `after_id` keeps its original meaning: IDs strictly greater than the supplied ID. It cannot be combined with field ordering or an anchor. For ordered queries, `.after_snapshot(&observed_snapshot)?` captures observed sort keys and the ID. Dynamic callers can build an anchor from an observed projected view with `runtime.query_anchor(&actor, &spec, &view).await?`, then use `spec.after(anchor)`.

A `QueryAnchor` is an explicit client-chosen boundary, **not a credential or a claim of server issuance**. It contains format version 1, kind, schema version, normalized equality/comparison lists and ordering, ID, and explicit missing/value sort keys. The runtime checks binding, canonical codec values, key count, types and encoded size even on empty data. Changing the page limit is allowed; changing the bound kind/predicates/order is rejected. Clients may choose different valid boundary values just as they may choose filters; this never grants access. Anchor helpers use only supplied observed values and do not fetch hidden fields. The projected helper checks current actor, query and sort grants; it does not certify current row/field visibility on the supplied view. Actual selection independently checks those grants.

These are moving views with current data/current authority. Updates may move a row across the boundary and cause repeats or omissions. Inserts before the boundary may be missed; deleting the anchor row does not invalidate its observed keys. No transactional snapshot is retained, and a full page does not prove another authorized row exists. Every call still loads the bounded kind snapshot: a small result limit does not bypass snapshot row/byte limits, and exceeding those bounds returns `TooLarge` rather than a partial page.

Shared SQLite/redb conformance covers exact numeric and codec behavior, missing/null, sort grants, binding and malformed anchors, typed/projected/live behavior, and mutation-driven continuation. HTTP tests exercise the same structured query through query/live routes. Indexed execution and cost-based strategy selection remain separately measured experiments until a reviewed storage planning contract is introduced.
