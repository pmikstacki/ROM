# boundary-adapters

## Purpose

Transport-independent access and separate integrations. This capability defines the observable contract for ROM implementations.

## ADDED Requirements

### Requirement: Transport-independent core
The core library SHALL be usable without an HTTP server or RabbitMQ broker dependency.

#### Scenario: Transport-independent core contract
- **GIVEN** an application embeds the core and a persistence adapter
- **WHEN** it executes an action in-process
- **THEN** the resource transition requires neither HTTP nor RabbitMQ

### Requirement: Equivalent mutation semantics
Transport adapters SHALL invoke the same core mutation contract rather than write resource storage directly.

#### Scenario: Equivalent mutation semantics contract
- **GIVEN** equivalent authenticated actions arrive through two different adapters
- **WHEN** the actions are executed against equivalent state
- **THEN** validation and conflict outcomes follow the same core rules

### Requirement: Deferred WASM
The initial extension contract SHALL support native Rust implementations without requiring a WASM runtime.

#### Scenario: Deferred WASM contract
- **GIVEN** an application uses a custom Rust field type
- **WHEN** the application starts and validates that field
- **THEN** no WASM runtime is required
