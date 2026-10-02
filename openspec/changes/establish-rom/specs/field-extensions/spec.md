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
