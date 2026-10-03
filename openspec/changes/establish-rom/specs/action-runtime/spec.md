# action-runtime

## Purpose

This capability defines one mutation path with validation and concurrency guarantees. It specifies the observable contract for ROM implementations.

## ADDED Requirements

### Requirement: One mutation path
All resource mutations SHALL pass through the core action contract, including observer and reaction updates.

#### Scenario: One mutation path contract
- **GIVEN** an observer receives new external state
- **WHEN** it requests a resource change
- **THEN** the core applies the same validation and authorization rules as for other callers

### Requirement: Presence-aware updates
Actions SHALL distinguish omitted fields, explicit null, and explicit values including false, zero and empty collections.

#### Scenario: Presence-aware updates contract
- **GIVEN** an existing boolean field is true
- **WHEN** an action explicitly sets it to false
- **THEN** false is persisted while omitted fields remain unchanged

### Requirement: Conflict detection
The core SHALL reject a mutation whose expected resource revision is stale without overwriting newer state.

#### Scenario: Conflict detection contract
- **GIVEN** two callers read the same revision
- **WHEN** one commits and the other submits its old expected revision
- **THEN** the second receives a conflict and the first change remains intact

### Requirement: Idempotent retries
Mutating actions SHALL support a scoped idempotency identity and reject reuse with different input.

#### Scenario: Idempotent retries contract
- **GIVEN** an action already committed
- **WHEN** the same caller retries the same identity and input
- **THEN** the original outcome is returned without another transition or event
