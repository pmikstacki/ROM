# event-reactivity

## Purpose

Committed events and recoverable reactions. This capability defines the observable contract for ROM implementations.

## ADDED Requirements

### Requirement: Committed facts only
Events SHALL describe committed transitions and SHALL NOT announce successful state changes before commit.

#### Scenario: Committed facts only contract
- **GIVEN** a transaction fails
- **WHEN** an event subscriber reads the journal
- **THEN** no event from the failed transaction is observable

### Requirement: Recoverable consumption
Reaction progress SHALL survive process restart and permit safe retry of unacknowledged work.

#### Scenario: Recoverable consumption contract
- **GIVEN** a reaction stops before acknowledging an event
- **WHEN** the process restarts
- **THEN** the pending event can be delivered again with the same identity

### Requirement: Ordered resource history
Committed events SHALL preserve revision order within each resource.

#### Scenario: Ordered resource history contract
- **GIVEN** successive revisions of a resource commit
- **WHEN** a consumer reads that resource history
- **THEN** events are returned in revision order

### Requirement: Shared action contract
Reactions SHALL request resource changes through the core action contract.

#### Scenario: Shared action contract contract
- **GIVEN** a reaction receives an event
- **WHEN** its resulting action fails validation
- **THEN** no partial resource change is committed
