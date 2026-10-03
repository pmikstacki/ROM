## ADDED Requirements

### Requirement: One generic resource command client
The CLI SHALL invoke the shared transport for any registered kind without
application-specific command handlers. Human and JSON outputs MUST be distinct.

#### Scenario: Unrelated resource kinds
- **GIVEN** Task and Inventory registered on the same runtime
- **WHEN** users mutate, query and observe them through the CLI
- **THEN** the same commands preserve codec, authorization and revision semantics
- **AND** JSON output contains only complete machine-readable results

### Requirement: Honest mutation recovery
The client MUST NOT automatically retry mutations or infer rollback from a lost
reply, cancellation or post-commit denial. Once a mutation is submitted, every
non-success response MUST be classified as unresolved until a phase-bearing
protocol can prove otherwise; raw HTTP categories do not provide that proof.

#### Scenario: Acknowledgement is lost
- **GIVEN** a submitted action committed before its response disappeared
- **WHEN** the CLI reports the failure
- **THEN** it identifies unresolved outcome and preserves the original recovery identity
- **AND** authorized explicit replay causes no second mutation

#### Scenario: Host policy fails after commit
- **GIVEN** a trusted ActorGate returns Conflict or TooLarge during post-commit observation
- **WHEN** the CLI receives that HTTP failure
- **THEN** it reports unresolved outcome rather than a definitive rejection
- **AND** the committed Resource and its receipt remain available for authorized recovery

### Requirement: Bounded distinct streams
Live snapshots and journal batches SHALL retain their own lifecycle and cursor
semantics. Malformed frames, size exhaustion and history gaps MUST be explicit.

#### Scenario: Slow or interrupted stream
- **GIVEN** arbitrary byte chunks and a slow or closed stdout consumer
- **WHEN** the CLI observes resources or subscribes to journal batches
- **THEN** memory remains bounded and only complete frames are emitted
- **AND** a history gap never silently advances to current head

### Requirement: Traceable deferred capability research
Every deferred topic SHALL have primary-source evidence, alternatives,
recommendation, dependencies, acceptance experiment and explicit evidence limits.
Research MUST NOT be represented as implemented capability.

#### Scenario: Goal completion
- **GIVEN** Studio is excluded and research of deferred features is in progress
- **WHEN** the completed goal is assessed
- **THEN** the research inventory, tested core/CLI changes and independent review are available
- **AND** future implementation remains clearly distinguished
