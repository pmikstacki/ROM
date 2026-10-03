## ADDED Requirements

### Requirement: One canonical bounded query contract
Typed, projected wire and live queries SHALL use the same canonical query plan and evaluator. QuerySpec SHALL preserve legacy conjunctive equality, explicit absence, ID ordering and after_id, and SHALL accept scalar eq, ne, lt, le, gt, ge comparisons plus up to four ordered fields. At most 32 combined predicates and the configured encoded command budget SHALL be accepted. Result limits SHALL be positive and no greater than the snapshot row limit. Duplicate order fields and unknown wire properties/operators SHALL be rejected. Typed filter-only conjunction SHALL reject nested order, limit or continuation instead of discarding them.

#### Scenario: Typed and wire predicates use the same codec
- **GIVEN** an accepted Resource field with a canonical codec
- **WHEN** typed, wire and live callers supply equivalent operands
- **THEN** the field shape and actual codec normalize operands before evaluation
- **AND** all callers observe the same authorized results

#### Scenario: Small page on an oversized kind
- **GIVEN** a maintained adapter's kind snapshot exceeds configured row or byte bounds
- **WHEN** a caller requests a one-row page with a selective predicate
- **THEN** the runtime returns TooLarge rather than a truncated candidate set
- **AND** the maintained adapter does not silently introduce approximate pushdown

### Requirement: Exact scalar and missing-value semantics
Integer comparisons SHALL preserve exact declared signed or unsigned values. Finite floating-point fields SHALL use numerical comparisons with positive and negative zero equal. Text, references and enum values SHALL compare by UTF-8 bytes without locale or Unicode normalization; booleans SHALL compare false before true. Ascending order SHALL rank missing before null before value, and descending SHALL reverse that rank. Resource ID SHALL remain the final ascending tie-breaker. Lists and maps SHALL support structural equality but SHALL reject ranges and ordering. Field-typed helpers MAY defer operator/shape compatibility checks until normalization.

#### Scenario: Adjacent large unsigned values
- **GIVEN** rows containing adjacent u64 values above signed-integer and exact-floating-integer ranges
- **WHEN** a query applies a range and sorts the field
- **THEN** comparison distinguishes the original exact values without signed or floating coercion

#### Scenario: Missing versus null
- **GIVEN** an optional nullable field with missing, null and concrete values
- **WHEN** equality, inequality, ranges or ordering are requested
- **THEN** equality to null matches only present null and inequality to a present operand excludes missing
- **AND** ranges match only present non-null values and reject null operands
- **AND** explicit absent equality matches missing while absent inequality matches present, with absent ranges rejected

### Requirement: Sort authority precedes ordering
Every predicate SHALL require the current query grant before consulting rows. Every ordered field SHALL require a separate default-denied sort grant before consulting rows, including empty datasets. The explicit allow_all_fields grant SHALL include sorting. Every row-readable candidate SHALL also grant read access to each ordered field before predicate evaluation or pagination; otherwise selection SHALL return Denied. Row-hidden resources SHALL not contribute sort keys or result positions.

#### Scenario: Protected order field
- **GIVEN** a principal may order a field but a row-readable candidate denies that field's read grant
- **WHEN** a projected query requests ordering by that field
- **THEN** the query returns Denied before filtering or limiting
- **AND** ranks do not reveal the protected value

#### Scenario: Sort policy denied on an empty kind
- **GIVEN** the principal has no sort grant and the kind contains no rows
- **WHEN** an ordered query is evaluated
- **THEN** the query returns Denied independently of the dataset

### Requirement: Client-chosen bound moving anchors
A QueryAnchor SHALL be an explicit client-chosen boundary, not an authorization credential or assertion of server issuance. It SHALL bind format version, kind, schema version, canonical predicate lists and ordering, and contain an ID plus one canonical missing/value key per ordered field. Validation SHALL check binding, bounds, key count and shape before scanning even empty datasets. Page limits SHALL not be bound. Current row, query, sort and field authority SHALL be checked independently at evaluation. after_id SHALL remain valid only for ID ordering and SHALL conflict with an anchor.

#### Scenario: Query-bound anchor rejection
- **GIVEN** an anchor for a specific normalized filter and order
- **WHEN** the caller changes kind, schema version, bound predicates, ordering, key count or canonical key shape
- **THEN** selection rejects the anchor even if there are no rows
- **AND** changing only a valid page limit remains accepted

#### Scenario: Data moves across a continuation
- **GIVEN** an anchor preserves values observed on an earlier page
- **WHEN** a row is updated, inserted or deleted before the next call
- **THEN** the next page compares current authorized rows with the observed boundary
- **AND** repeats or omissions caused by movement are allowed without claiming snapshot retention
- **AND** deleting the original anchor row does not invalidate its observed keys

#### Scenario: Arbitrary valid boundary grants no access
- **GIVEN** a client chooses valid anchor values not produced by a helper
- **WHEN** the query is evaluated
- **THEN** the boundary behaves as client input and cannot bypass current authority
- **AND** helpers using supplied projected values do not certify their provenance or current field visibility
