## ADDED Requirements

### Requirement: Typed complete queries
The typed API SHALL allow unfiltered queries without inventing a field predicate.
It MUST retain the existing row, field, pagination and resource budget semantics.

#### Scenario: Read and observe all authorized resources
- **GIVEN** two Resources and policies hiding a row or field
- **WHEN** the author uses Query::all or its Default and observes results
- **THEN** ordinary and live reads use the existing authorization and bounds
- **AND** absence of a predicate does not grant access to hidden data

### Requirement: Fallible capacity configuration
Runtime and HTTP constructors MUST reject semaphore counts above the engine limit
using a configuration error rather than panic.

#### Scenario: Oversized input
- **GIVEN** an otherwise valid configuration
- **WHEN** a concurrency count is MAX_PERMITS plus one or usize::MAX
- **THEN** construction returns an error without unwinding or starting work

### Requirement: Authorized metadata discovery
Discovery SHALL require explicit metadata grants and current identity authority.
It MUST be bounded, default-denied and independent of row-operation permission.

#### Scenario: Hidden metadata and nested references
- **GIVEN** one discoverable Resource referencing an undiscoverable kind
- **WHEN** a caller requests discovery
- **THEN** the hidden kind, fields, actions and reference target are not disclosed
- **AND** discovery does not scan rows or infer mutation grants

#### Scenario: Current authority and capacity
- **GIVEN** an actor revoked during discovery or output exceeding its byte budget
- **WHEN** the result would be returned
- **THEN** no stale or truncated catalog is returned as success
