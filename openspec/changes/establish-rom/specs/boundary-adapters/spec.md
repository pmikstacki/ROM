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

### Requirement: Descriptor-driven endpoints and streaming
An enabled transport adapter SHALL expose registered resource kinds through shared handlers and stream integration derived from their definitions and policies. Adding a kind SHALL NOT require application-authored endpoints or a separate live-update implementation. Core operations SHALL remain available without a transport adapter.

#### Scenario: HTTP-enabled host registers a resource
- **GIVEN** the host enables the generic HTTP adapter and its streaming transport
- **WHEN** a valid resource definition is registered
- **THEN** its permitted standard endpoints and committed-change subscription become available without writing per-kind HTTP or streaming handlers
