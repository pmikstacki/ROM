# capability-adapters

## Purpose

Provider-independent semantic contracts for resource persistence, blob storage and notification delivery.

## ADDED Requirements

### Requirement: Provider-independent core
ROM core SHALL depend on core-owned semantic interfaces and values. Database query languages, transaction handles, storage SDK types, provider credentials and native error types SHALL remain inside host configuration and adapters. Resource definitions SHALL NOT require changes when substituting adapters that satisfy the same requested contract.

#### Scenario: Substitute a persistence model
- **GIVEN** a resource uses a supported persistence contract
- **WHEN** the host replaces a relational adapter with a conforming transactional key-value adapter
- **THEN** the same resource operations execute without database-specific code in the resource or core

### Requirement: Explicit capability admission
ROM SHALL validate required capabilities and limits against the configured adapter profile. Unsupported guarantees SHALL be rejected without silently weakening semantics or applying partial effects.

#### Scenario: Unsupported conditional write
- **GIVEN** a blob adapter does not support conditional replacement
- **WHEN** an operation requests replacement only for a matching version
- **THEN** the operation fails as unsupported without performing an unconditional write

### Requirement: Conditional atomic persistence
A conforming durable persistence adapter SHALL atomically arbitrate expected resource state and command identity, then commit resource state, revision, action outcome, events and requested durable effect intentions within its declared scope. Reuse of an identity with different canonical input SHALL be rejected.

#### Scenario: Concurrent duplicate command
- **GIVEN** two attempts submit the same command identity and canonical input
- **WHEN** both attempt to commit
- **THEN** at most one transition is applied and successful resolution returns the same recorded outcome

#### Scenario: Effect intention write failure
- **GIVEN** a transition includes a notification intention
- **WHEN** storage fails before the complete atomic bundle commits
- **THEN** no partial resource transition, success event, action receipt or delivery intention becomes committed

### Requirement: Honest outcome and durability reporting
Adapters SHALL distinguish known non-commit from unknown outcome and SHALL declare their acknowledgment durability and consistency profile. Receipt absence during an unresolved attempt SHALL NOT be reported as proof that the attempt cannot commit. Retention expiry SHALL have explicit recovery semantics.

#### Scenario: Delayed commit after timeout
- **GIVEN** an attempt times out and an authoritative read currently finds no receipt
- **WHEN** the original attempt remains able to commit
- **THEN** ROM preserves the uncertain outcome and any retry uses the same atomically arbitrated command identity

### Requirement: Safe journal continuation and queries
Journal continuation SHALL be opaque to core and SHALL NOT silently skip later-visible retained commits. Adapters SHALL preserve ROM's requested query semantics and consistency or reject them. Unsupported or expired continuation SHALL produce an explicit error or resync requirement.

#### Scenario: Journal allocation differs from commit order
- **GIVEN** two writers allocate storage identifiers in a different order from their commits
- **WHEN** a consumer resumes a retained journal
- **THEN** adapter continuation does not omit the later-visible committed event because its allocated identifier was lower

#### Scenario: Snapshot requirement unavailable
- **GIVEN** a query requires a coherent snapshot across pages
- **WHEN** the configured adapter cannot establish that guarantee
- **THEN** ROM rejects the requirement rather than presenting an ordinary paginated scan as a snapshot

### Requirement: Blob completion and limits
Blob adapters SHALL enforce their declared key, size and execution limits and distinguish successful complete-object visibility from interrupted or unknown writes. Missing-object deletion SHALL be idempotent. Filesystem isolation and crash durability SHALL be declared separately from key validation and atomic visibility.

#### Scenario: Interrupted upload input
- **GIVEN** an upload has not submitted a provider write
- **WHEN** its input fails or exceeds the accepted limit
- **THEN** the adapter rejects it without replacing the existing object with partial content

### Requirement: Named typed notification channels
ROM SHALL support named channels implemented by ordinary native Rust sending functions behind a shared delivery contract. Registration SHALL bind payload schema/version and validation. Delivery SHALL occur only after its durable intention commits, outside the resource database transaction.

#### Scenario: Custom function channel
- **GIVEN** the host registers a checked payload and async Rust function for a channel
- **WHEN** a committed intention targets that channel
- **THEN** the generic dispatcher invokes the function without provider-specific branching in core

### Requirement: Recoverable external effects
Notification delivery SHALL preserve stable identity across attempts, bound retries, classify accepted/retryable/permanent/unknown outcomes and retain inspectable unresolved or terminal failures. Stale workers SHALL NOT overwrite outcomes recorded under a newer claim. ROM SHALL NOT claim exactly-once external effects without a supported recipient contract.

#### Scenario: Acceptance precedes lost acknowledgment
- **GIVEN** the receiver accepts a delivery but the worker stops before recording acknowledgment
- **WHEN** ROM recovers the intention
- **THEN** its identity remains unchanged and recovery accounts for possible duplicate effects or supported recipient reconciliation

### Requirement: Cross-capability transaction boundaries
ROM SHALL NOT represent a database commit, blob upload and external send as one atomic transaction unless an explicit implementation proves that contract. Resource/blob coordination SHALL specify incomplete-operation cleanup and external effect recovery.

#### Scenario: Blob upload followed by rejected resource commit
- **GIVEN** an immutable blob upload succeeds
- **WHEN** its resource-reference commit fails
- **THEN** ROM treats the blob as an unreferenced cleanup candidate under the configured policy rather than reporting a successful resource operation

### Requirement: Shared conformance evidence
Supported adapter profiles SHALL pass shared semantic and resilience tests with their core dependencies independently inspected. Test reports SHALL distinguish actual interoperability and injected fault coverage from source review, simulation and untested guarantees.

#### Scenario: S3-compatible certification scope
- **GIVEN** tests pass against one local S3-compatible server
- **WHEN** the adapter's supported profiles are documented
- **THEN** the report names the exercised endpoint implementation and does not claim all S3-compatible deployments were tested
