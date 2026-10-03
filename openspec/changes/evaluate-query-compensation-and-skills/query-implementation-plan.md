# Maintained query slice

Owner-authorized, following prototype b7cfdb1 and parent API review. No new dependencies, adapter format, pushdown, planner API, reporting or joins.

1. Add shared SQLite/redb conformance first: scalar ranges, exact u64, finite/custom codecs, missing/null order, Unicode/ties, typed/wire/live equality, empty/hidden sort denial, moving mutation pages and malformed/bound anchors.
2. Extend QuerySpec with bounded comparisons, order and client anchor. Retain existing equality/absence and ID-only after_id. Add typed field comparisons/order and fallible filter-only conjunction. Normalize once into a private plan; snapshot bounds remain unchanged.
3. Add default-denied sort_policy; whole-field explicit grant includes sort. Check before any storage rows, then require sort-field reads for every row-readable candidate before sorting. Hidden rows never contribute order keys.
4. Add after_snapshot typed helper and actor-checked projected-anchor helper. Anchors are client selections, not issued credentials. Bind format version, kind, schema version, normalized predicates/order; omit limit. Require bounded canonical values, correct count/types, no duplicate sort fields, no conflicting after_id. Recheck authority on each evaluation.
5. Verify focused conformance, existing consumer/query and HTTP behavior, format/lint. Parent performs independent integration review before merge.

Ranges only compare present non-null scalar values; missing does not satisfy ne, but explicit null can differ from a non-null equality operand. Ascending order is Missing < Null < value; descending reverses; text/reference/enum use UTF-8 bytes, booleans false<true, exact signed/unsigned compare by declared shape, finite floating values use numerical order (signed zero equal). Lists/maps retain structural equality and cannot range/order. Final tie-break always ascending resource ID. Moving anchors retain observed keys; mutations can move rows across the boundary, cause repeats or omissions; no snapshot retention promised.

Implementation files: resource.rs (typed front door and grants), query_spec.rs (wire types/builders), query_eval.rs (private normalization/evaluation/anchors), lib.rs module declaration, tests/persistence/tests/query_planning.rs, consumer-facing docs. No bespoke HTTP parser: serde QuerySpec already flows through the shared query path.
