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

### Requirement: Human-readable authoring and diagnostics
The public resource interface SHALL provide a documented path for defining a resource, adding custom behavior and observing it without per-kind infrastructure code. Generated resource definitions SHALL remain inspectable. Rejected declarations and operations SHALL report structured error categories and safe resource, action or field context sufficient to locate the problem, without exposing protected values.

#### Scenario: Diagnose an invalid field declaration
- **GIVEN** a resource declaration contains an unsupported field capability
- **WHEN** compilation or registration rejects that declaration
- **THEN** the diagnostic identifies the responsible field and unsupported capability rather than requiring the author to trace internal generated code

#### Scenario: Use a resource without transport infrastructure
- **GIVEN** an application has configured ROM and its persistence adapter
- **WHEN** the author exercises a declared resource and custom action in a library test
- **THEN** the same core behavior is available without starting an HTTP server or writing resource-specific scheduler code

### Requirement: Validated registration contract
Derived and manually implemented resource definitions SHALL pass through the same registration checks before use. Registration SHALL reject duplicate resource identities, conflicting bindings and unsupported declared adapter capabilities. Accepted definitions SHALL remain immutable during execution; successful registration SHALL NOT bypass validation of actual values, permissions or revisions.

#### Scenario: Equivalent invalid definitions
- **GIVEN** a derived definition and a manually implemented definition require the same unsupported adapter capability
- **WHEN** either definition is registered
- **THEN** registration rejects it with the same structured error category and identifies the responsible declaration and capability

### Requirement: Metadata and codec agreement
ROM's field descriptor and executable codecs SHALL agree on public names, presence, nullability, defaults and value validation. Protocol adapters SHALL use this accepted contract. Independent serialization annotations SHALL NOT silently redefine ROM's protocol.

#### Scenario: Renamed nullable field round trip
- **GIVEN** a resource field has a declared public name and permits null
- **WHEN** a supported adapter decodes, validates, persists and encodes that field
- **THEN** every step uses the declared public name and preserves the distinction between omission and explicit null according to the operation contract

#### Scenario: Unsupported direct serialization configuration
- **GIVEN** a declared codec mode encounters a conflicting or unsupported serialization setting
- **WHEN** the definition is compiled or registered
- **THEN** ROM rejects the configuration with field context instead of publishing metadata that disagrees with actual encoding
