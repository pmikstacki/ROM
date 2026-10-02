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

### Requirement: Unambiguous input normalization
Resource operations SHALL normalize supported field and query identifiers deterministically and reject conflicting aliases. Unsupported input constructs SHALL produce explicit diagnostics rather than silently dropping declared fields.

#### Scenario: Conflicting aliases
- **GIVEN** two supported input spellings normalize to the same field operation but supply different values
- **WHEN** the input is normalized
- **THEN** it is rejected as ambiguous rather than choosing a value according to map iteration order

### Requirement: One declaration supplies standard resource behavior
Registering a resource definition SHALL supply its standard create, read, update, delete, query and subscription behavior through shared ROM machinery. Applications SHALL NOT need resource-specific repositories, CRUD handlers or event-publication plumbing for those standard operations. Explicit policy may disable or restrict operations.

#### Scenario: Additional resource kind
- **GIVEN** an application has configured a persistence adapter and registered one resource kind
- **WHEN** it registers another valid resource definition with fields and policy
- **THEN** the standard operations, persistence and committed-change subscriptions become available without adding a repository or standard-operation implementation for that kind
