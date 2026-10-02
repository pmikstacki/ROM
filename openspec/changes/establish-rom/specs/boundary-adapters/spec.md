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

### Requirement: Core-owned transport capabilities
Core SHALL define transport-independent semantic contracts for resource discovery, action invocation, reads and queries, live observation, and committed-journal subscriptions. HTTP and broker bindings SHALL implement declared capability profiles over these contracts. Core SHALL NOT require their wire formats, server types or broker delivery identifiers. Supporting protocol values SHALL NOT introduce additional application domain entities alongside Resource.

#### Scenario: One resource through different bindings
- **GIVEN** a resource is registered once and two bindings support its action contract
- **WHEN** equivalent trusted callers invoke that action through either binding or directly in Rust
- **THEN** the same resource definition, authorization, validation, revision and durable idempotency rules apply without per-kind handlers

### Requirement: Validated transport profiles
Each binding SHALL declare supported operation families, compatible codecs, size limits and applicable lifecycle and recovery guarantees. Registration SHALL validate required combinations against core and persistence capabilities. Unsupported operation families SHALL fail explicitly before effects. Capability support SHALL NOT itself grant a caller permission or weaken core invariants.

#### Scenario: Live observation is unavailable
- **GIVEN** a binding supports actions but does not support live queries
- **WHEN** a host requires live observation through that binding
- **THEN** registration rejects that configuration rather than treating command delivery as a live-query capability

### Requirement: Distinct observation contracts
Live-query subscriptions and journal subscriptions SHALL retain separate semantic contracts and recovery rules. A live result MAY coalesce intermediate states under an explicit freshness and resync policy. A journal subscription SHALL expose committed history under scoped cursor and retention rules, reporting gaps explicitly. Both SHALL enforce current disclosure policy, bounded buffering and defined overflow behavior.

#### Scenario: Resume after history retention expires
- **GIVEN** a journal cursor is older than retained history
- **WHEN** a consumer resumes through a binding supporting journal subscriptions
- **THEN** ROM reports a history gap rather than substituting a current live snapshot as complete event history

### Requirement: Transport lifecycle preserves action outcomes
Bindings SHALL distinguish transport receipt, resource commit and response delivery. Caller cancellation SHALL NOT imply rollback or release the capacity of admitted work still executing. Broker acknowledgment SHALL follow a recoverable terminal action outcome or a durable handoff with an explicit recovery owner. Serialized caller claims SHALL NOT construct trusted authority; outcome disclosure SHALL use current authorization.

#### Scenario: Commit precedes a lost transport response
- **GIVEN** an action commits but the reply or broker acknowledgment is lost
- **WHEN** an authorized caller retries the same scoped action identity and semantic input
- **THEN** durable resolution returns the original outcome without a second transition, subject to the explicit identity retention policy
