# execution-engine

## Purpose

Use Tokio and Rayon to execute ROM operations while preserving independent domain semantics, bounded capacity and durable commit guarantees.

## ADDED Requirements

### Requirement: Tokio and Rayon execution foundation
The runtime SHALL use Tokio for asynchronous execution and I/O and Rayon for CPU-bound work that requires offloading. Small synchronous validation SHALL NOT require dispatch to a worker pool.

#### Scenario: CPU work with concurrent I/O
- **GIVEN** an admitted action needs expensive CPU computation and another needs database I/O
- **WHEN** the CPU job is submitted
- **THEN** it runs on the configured Rayon pool and its caller awaits completion without synchronously blocking a Tokio worker

### Requirement: Independent domain contract
ROM SHALL own resource identity, validation, authorization, action semantics, durable persistence and committed events. Executor task identities SHALL NOT serve as durable resource identities.

#### Scenario: Reloaded resource
- **GIVEN** a resource has committed durable state
- **WHEN** the process restarts and reconstructs execution state
- **THEN** the resource retains its ROM identity regardless of newly allocated tasks or workers

### Requirement: Explicit executor ownership
The host SHALL provide shared executor instances or explicitly delegate their construction to ROM. ROM SHALL NOT create a runtime or thread pool per action or resource. Shutdown ownership and resource budgets SHALL be explicit.

#### Scenario: Multiple resource kinds
- **GIVEN** the host supplies one Tokio runtime and one Rayon pool
- **WHEN** multiple resource kinds execute work
- **THEN** they share those configured executors without creating additional per-kind pools

### Requirement: Bounded admission
The engine SHALL bound queued and executing work before spawning tasks or submitting CPU jobs. Capacity accounting SHALL include pending payloads and SHALL NOT rely solely on worker-thread count.

#### Scenario: Saturated admission
- **GIVEN** the configured admission limit is reached
- **WHEN** another action arrives
- **THEN** admission applies documented bounded waiting or rejection rather than retaining unlimited pending work

### Requirement: Cancellation preserves actual work accounting
Canceling a caller SHALL NOT release CPU capacity for work that is still running. CPU jobs SHALL return proposed results through the action pipeline rather than independently mutating durable resources.

#### Scenario: Caller stops waiting
- **GIVEN** a CPU job has started and holds an admission permit
- **WHEN** its caller is canceled before the job finishes
- **THEN** the permit remains held until actual completion and cancellation alone does not commit a resource change

### Requirement: Commit outcomes remain authoritative
Task completion, cancellation or channel delivery SHALL NOT by itself establish a durable resource transition. State and domain events SHALL become committed together through the persistence contract. An uncertain commit outcome SHALL be resolved through durable action identity before retrying its effects.

#### Scenario: Persistence failure
- **GIVEN** CPU computation successfully proposes a resource change
- **WHEN** its persistence transaction fails before commit
- **THEN** no resource state or success event from that proposal is committed

#### Scenario: Response lost after commit
- **GIVEN** the database commits an action but its caller loses the response
- **WHEN** the caller retries using the same scoped action identity
- **THEN** the existing outcome is resolved without repeating the committed transition

### Requirement: Shared extension contract
Native Rust extensions SHALL register through ROM's public contracts and request resource mutations through the action interface. They SHALL be documented as trusted in-process code rather than a sandbox.

#### Scenario: Optional HTTP extension
- **GIVEN** an HTTP integration is omitted
- **WHEN** an embedded caller submits a valid authorized action
- **THEN** the same resource behavior is available without an HTTP server
