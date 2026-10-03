# field-extensions

## Purpose

Built-in field families and Rust extension contracts. This capability defines the observable contract for ROM implementations.

## ADDED Requirements

### Requirement: Rust registration
The core SHALL support application-defined field types through public Rust extension interfaces without editing core source.

#### Scenario: Rust registration contract
- **GIVEN** a custom field implementation supplies a stable type identity and version
- **WHEN** the application registers it
- **THEN** resource definitions may use it through the same validation contract as built-in fields

### Requirement: Versioned round trip
Field implementations SHALL define validation and versioned canonical storage encoding and decoding.

#### Scenario: Versioned round trip contract
- **GIVEN** a valid custom field value
- **WHEN** it is persisted and reloaded using its registered version
- **THEN** the decoded value preserves the defined semantics

### Requirement: Explicit capabilities
The runtime SHALL reject unsupported field capabilities before executing dependent operations.

#### Scenario: Explicit capabilities contract
- **GIVEN** a field or adapter does not support ordering
- **WHEN** a caller requests ordering by that field
- **THEN** the caller receives an unsupported-capability error

### Requirement: Unambiguous registration
The registry SHALL reject conflicting registrations of the same type identity and version.

#### Scenario: Unambiguous registration contract
- **GIVEN** a type version is already registered
- **WHEN** a different implementation attempts to register that identity and version
- **THEN** startup fails with a diagnostic identifying the collision

### Requirement: Recursive type fidelity
The registered descriptor model SHALL preserve complete nested type information across schema and storage adapters. Unsupported nesting SHALL be rejected explicitly rather than degraded to an unconstrained value.

#### Scenario: Nested integer lists
- **GIVEN** a field is declared as a list of lists of integers
- **WHEN** its schema is exported or its stored value is validated
- **THEN** both collection levels and the integer element constraint remain represented

### Requirement: Unique descriptor identity
The registry SHALL validate resource identities, field paths and any generated identifiers before publishing a descriptor generation. Distinct resource namespaces SHALL NOT silently collapse into one identity or generated symbol.

#### Scenario: Equal short names
- **GIVEN** two resource kinds have the same short name in different namespaces
- **WHEN** the registry prepares their descriptors
- **THEN** before publication, it preserves both full identities or explicitly rejects an unsupported naming collision

### Requirement: Immutable descriptor snapshots
Published descriptor snapshots SHALL be immutable to ordinary consumers. Reconfiguration SHALL publish a validated generation rather than expose partially mutated shared metadata.

#### Scenario: Concurrent metadata readers
- **GIVEN** two consumers hold the same published descriptor generation
- **WHEN** a new registration generation is prepared
- **THEN** both readers continue to see their complete original generation until they explicitly acquire a newer snapshot
