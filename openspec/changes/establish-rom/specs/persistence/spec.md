# persistence

## Purpose

Atomic persistence and a database adapter contract. This capability defines the observable contract for ROM implementations.

## ADDED Requirements

### Requirement: Atomic transition persistence
A persistence adapter SHALL commit resource state, revision, action receipt and resulting events atomically.

#### Scenario: Atomic transition persistence contract
- **GIVEN** a transition would create state and events
- **WHEN** a storage failure occurs before commit
- **THEN** none of those writes become visible

### Requirement: Durability after acknowledgment
A successful durable action receipt SHALL imply that its committed transition survives application restart under the adapter durability contract.

#### Scenario: Durability after acknowledgment contract
- **GIVEN** an action has returned durable success
- **WHEN** the application restarts
- **THEN** the stored state and its events can be recovered

### Requirement: Adapter conformance
Supported database adapters SHALL pass the shared persistence conformance scenarios.

#### Scenario: Adapter conformance contract
- **GIVEN** a new database adapter
- **WHEN** it cannot provide atomic state and event persistence
- **THEN** it is not advertised as a conforming durable adapter
