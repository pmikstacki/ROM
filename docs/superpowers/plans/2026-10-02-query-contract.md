# Shared query contract implementation plan

> Use executing-plans and test-driven-development for this package.

**Goal:** Typed, projected and live reads share canonical field predicates and bounded keyset pagination.
**Architecture:** Derive generates single-field normalization from the same codecs as full Resource decoding; native implementations provide the same method. A core QuerySpec holds conjunctive equality filters, optional after_id and result limit. One row-selection implementation validates every predicate and permission before scanning, then filters authorized rows in stable ID order.
**Spec:** openspec/changes/integrate-resource-mvp/design.md.

- [x] Reproduce wire numeric/custom codec mismatch and write conformance tests for conjunction, pagination, unknown/forbidden fields on empty sets, bounded query size and typed/projected/live parity.
- [x] Add Resource::normalize_field with derive-generated implementation, including native manual fixtures. Preserve codec/schema agreement and fail unsupported fields.
- [x] Add QuerySpec/Equality and typed fluent and(field,value), after_id, limit; share selection with projected APIs while preserving existing equality wrappers for transports.
- [x] Require at most32 predicates and command-size encoded input, result limit1..snapshot_rows. Candidate snapshot remains bounded and errors on overflow; pagination does not silently hide that overflow. No OR/joins/arbitrary comparison or snapshot-consistent paging claim.
- [x] Verify tests, compiler diagnostics and Clippy; report identity keyset paging under concurrent changes and current authorization on every page.
