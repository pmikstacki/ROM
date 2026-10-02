# Shared query semantics

Typed selectors, projected queries and live queries use the same core selection
path. The MVP supports conjunctions of equality predicates, stable UTF-8 Resource
ID ordering and optional keyset pagination. Storage adapters never reinterpret
predicates as backend-specific SQL syntax.

```rust,ignore
let query = Task::owner_field().equals("alice".to_owned())
    .and(Task::done_field(), false)
    .limit(20);
let first = runtime.query(&actor, &query).await?;
if let Some(last) = first.last() {
    let next = query.after_id(last.id.clone());
    let second = runtime.query(&actor, &next).await?;
}
```

For dynamic bindings, `QuerySpec::equal("owner", json!("alice"))`
`.and("done", json!(false)).limit(20)` expresses the same operation.
`query_spec_projected` and `live_spec_projected` return authorized views;
complete typed queries require access to every returned field. Existing
single-equality projected entrypoints delegate to this same implementation.
`QuerySpec::all()` has no predicate; row and returned-field policies still apply.

Predicate values first pass the accepted field shape and its actual codec.
`Resource::normalize_field` is generated from the same field list as decode,
encode and metadata; handwritten native Resources provide that method explicitly.
This matters for finite floats and canonical custom fields: JSON `0` must find
stored `0.0`, and a trimming codec must behave identically in embedded and wire
queries. The coordinator's negative control bypassed this normalization and
observed zero matches instead of three; restoring it passes the regression.

Every predicate's query permission is checked before scanning, including empty
result sets. Up to 32 predicates and the configured command-size encoded budget
are accepted. Requested result limits must be between one and the runtime's
snapshot row limit. Unknown operators are rejected; this is not an arbitrary
JSON expression language, OR/joins/aggregates or per-provider approximation.

Keyset pages are **moving views**: `after_id` means IDs strictly greater than that
value under current data and current authorization. Concurrent insertions before
the continuation can be missed; this is not a retained transactional snapshot.
An empty final page ends traversal. Results do not disclose a next ID belonging
to an unauthorized row. Each call still takes a bounded candidate snapshot;
if the candidate set exceeds configured row/byte limits the operation fails
instead of silently truncating it. Paging does not make large datasets fit this
initial scan-based execution profile. Indexed/seek-capable adapters are a later
capability, not a hidden performance claim.

Three focused consumer tests cover codec equivalence, typed/projected/live
conjunction and pagination, and invalid/bounded query input. Existing policy and
live lifecycle tests run through the shared selector. The complete native verifier
passes; no SQL benchmark or production query planner is claimed.
