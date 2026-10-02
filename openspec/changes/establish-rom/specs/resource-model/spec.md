# resource-model

## Purpose

Resource identity, typed fields, revisions and lifecycle. This capability defines the observable contract for ROM implementations.

## ADDED Requirements

### Requirement: Uniform identity
The library SHALL identify every resource by a stable identity and declared resource kind, independent of any transport.

#### Scenario: Uniform identity contract
- **GIVEN** a registered resource kind
- **WHEN** a resource is created
- **THEN** its identity and kind remain stable across storage round trips

### Requirement: Revisioned transitions
Every committed resource transition SHALL advance its revision; a rejected or no-op action SHALL NOT advance it.

#### Scenario: Revisioned transitions contract
- **GIVEN** a resource at a known revision
- **WHEN** an action changes its state
- **THEN** the returned resource has the next revision

### Requirement: Typed state
Resource state SHALL conform to its registered, versioned field definitions.

#### Scenario: Typed state contract
- **GIVEN** a field declared as boolean
- **WHEN** an action provides an incompatible value
- **THEN** the core rejects the action without changing stored state
