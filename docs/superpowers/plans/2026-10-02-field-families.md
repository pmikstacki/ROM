# Maintained field families implementation plan

> Use the executing-plans and test-driven-development skills for this package.

**Goal:** Extend the single Field contract without per-resource codecs or a second schema.
**Architecture:** Add signed integers, finite floats, collections, enum metadata and typed identity references to existing Shape/Field; registration rejects ambiguous or invalid nested shapes.
**Tech stack:** Existing Rust/serde_json, no new dependencies.
**Spec:** openspec/changes/integrate-resource-mvp/design.md.

Constraints: preserve missing/null behavior. Reject nested nullable ambiguity at every collection depth. Bound the declared shape nesting. A typed reference does not imply database foreign-key enforcement. Native custom fields still implement Field and their declared wire shape. Derive and handwritten resources share normalization.

- [x] Add public-API failing tests for mixed scalar/collection/reference Resource roundtrip, invalid nested elements, nonfinite float rejection, canonical map order, enum validation and hidden nested nullable registration rejection.
- [x] Implement Shape::{I64,F64,List,Map,Enum,Reference}, Field for i64/FiniteF64/Vec<T>/BTreeMap<String,T>, ResourceRef<R> with nonempty validated ID, and recursive shape validation limited to 16 levels.
- [x] Verify unknown reference target rejected on build, while cyclic type declarations are permitted; reference existence/delete integrity is explicitly not advertised by this identity-only profile.
- [x] Run maintained consumer suite, compilation diagnostics, fmt/Clippy and core no-default-feature build; document supported field/presence behavior and remaining PATCH/query integration.
