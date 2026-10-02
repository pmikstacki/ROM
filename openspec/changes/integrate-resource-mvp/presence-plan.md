# Presence and typed patch implementation plan

> For agentic workers: execute task by task using superpowers:executing-plans; existing autonomous implementation authorization applies.

Goal: preserve omission, null and removal through derive, actions, queries and the shared commit pipeline.
Architecture: Presence<T> is a top-level optional field; Option<T> remains explicit nullable. Patch operations use accepted codecs and the existing revision/idempotency/authorization path.
Tech stack: existing Rust/serde/Tokio workspace only.
Spec: specs/integrated-mvp/spec.md, design.md.

Constraints: no per-kind plumbing, no database or transport implementation in core; reject ambiguous nested presence; explicit standard patch operation namespace.
Review focus: denied absent fields must not leak presence through complete reads; unchanged forbidden patch fields still require permission; null must not become omission; retry must not apply twice; typed and wire predicates agree.

- [x] Add failing consumer tests in examples/consumer/tests/presence.rs: Missing/Value(None)/Value(Some) encode distinctly; patch omission leaves unchanged, remove deletes, null persists, replay stays same revision, live missing predicate tracks removal; unauthorized absent field complete read denied; nested presence registration rejected; Presence action input roundtrip.
- [x] resource.rs + derive: Field presence/default missing/input hooks, Shape::Optional accepted only top-level, descriptor normalization permits declared absent fields; typed selectors select missing explicitly.
- [x] patch.rs + invocation/execution: Patch<R>, FieldUpdate Set/Remove, standard Patch variant; canonicalize before fingerprint, enforce selected field permission even unchanged, normalize complete proposal before commit.
- [x] query_spec.rs + projection.rs: explicit absence predicate and complete-read field authorization independent of map length.
- [x] Run focused tests then full scripts/check. Document wire semantics and evidence; commit only tracked source/test/docs.
