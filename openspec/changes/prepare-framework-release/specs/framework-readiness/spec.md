## ADDED Requirements

### Requirement: Application authors use one public Resource contract
ROM SHALL support the reference application's actions, policies, reactive reads,
relations and reactions through public library APIs without per-kind repositories,
transport controllers or direct application database writes.

#### Scenario: Pending compensation survives application restart
- **GIVEN** two committed reservations and a confirmed rejection with durable pending work
- **WHEN** the application process terminates before work execution and is restarted
- **THEN** authorized recovery releases only the rejected reservation
- **AND** replaying the same rejection identity adds no mutation or event

### Requirement: Data lifecycle preserves obligations
ROM SHALL enforce declared restrict relationships within its supported atomic
scope and provide versioned migration and retention operations preserving required
receipt identities, pending work and recovery sources.

#### Scenario: Upgrade is interrupted
- **GIVEN** persisted Resources, receipts and pending work in a supported older format
- **WHEN** an offline upgrade is interrupted
- **THEN** the original source remains recoverable. Incomplete output is not activated.

#### Scenario: Concurrent reference creation and target deletion
- **GIVEN** a live target and two operations that create a source reference and delete the target
- **WHEN** native transactions arbitrate these operations
- **THEN** at most one operation succeeds
- **AND** no committed source refers to a missing or deleted target

#### Scenario: A source kind is absent from a later Runtime
- **GIVEN** a persisted live source that refers to a target
- **WHEN** a later Runtime registers only the target kind and requests target deletion
- **THEN** the persisted reference prevents deletion without disclosing the source identity

#### Scenario: Receipt replay retains historical meaning
- **GIVEN** a source creation receipt and a later unlink followed by target deletion
- **WHEN** the original command is replayed with the same identity and fingerprint
- **THEN** persistence returns the original receipt without creating a new reference
- **AND** Runtime applies current disclosure authorization

#### Scenario: A backup has an incomplete reference index
- **GIVEN** a backup whose stored reference graph differs from its current Resource values
- **WHEN** export or restore validates the backup
- **THEN** validation fails before a destination is published

#### Scenario: A Resource field changes representation
- **GIVEN** a typed consecutive schema step with an exact source descriptor
- **WHEN** an offline migration converts the store into a fresh destination
- **THEN** current values, historical rows and work-source snapshots use the target representation
- **AND** Resource identities, revisions, receipt identities and fingerprints remain unchanged
- **AND** live references are rebuilt and validated before publication

#### Scenario: A request is retried after a field rename
- **GIVEN** a migrated receipt and its explicitly registered original request codec
- **WHEN** the caller repeats its original request
- **THEN** current authorization runs before normalization or disclosure
- **AND** only the receipt's recorded codec version interprets the input
- **AND** the existing result is returned without another mutation, event or work item
- **AND** a fresh request cannot use a legacy codec

#### Scenario: Migration contains unfinished obligations
- **GIVEN** a snapshot with unfinished actions or deliveries
- **WHEN** a migration plan runs
- **THEN** publication requires explicit validation of destination consumer compatibility
- **AND** frozen invocations, external payloads, delivery identities and uncertain outcomes remain unchanged
- **AND** expanded values that exceed persisted lifecycle budgets fail without dropping work

#### Scenario: An older maintenance tool reads a migrated store
- **GIVEN** a store whose receipt codec versions are required for correct replay
- **WHEN** a maintenance tool without support for that metadata opens it
- **THEN** the format marker prevents that tool from silently dropping the metadata

#### Scenario: Retention expires an idempotency epoch
- **GIVEN** explicit monotonic admission and replay floors selected by the host
- **WHEN** maintenance removes eligible receipts from an expired epoch
- **THEN** a retry in that epoch fails without another mutation, even if its input changes
- **AND** omission of an epoch never assigns the current epoch to an old request
- **AND** native commit enforces expiry even for a retained current-row proof

#### Scenario: Sealed work drains before expiry
- **GIVEN** an epoch closed to new external commands with unfinished causal work
- **WHEN** an exact persisted claim completes work in that epoch
- **THEN** its mutation retains the original epoch and can commit
- **AND** expiry rejects unfinished or uncertain roots without deleting their budgets

#### Scenario: Old backup predates a retention policy
- **GIVEN** an older backup and a newer trusted retry fence stored outside that backup
- **WHEN** the host builds a Runtime from the restored store with that fence
- **THEN** activation fails before older retry boundaries can admit requests

### Requirement: Optimization preserves observable semantics
ROM SHALL integrate strategy selection and native planner adapters only with
transactionally maintained indexes, bounded rebuild/recovery and conformance to
the shared authorization, filter, ordering and admission contracts.

#### Scenario: Index recovery changes execution path
- **GIVEN** the same authorized query before and after an interrupted index rebuild
- **WHEN** ROM selects an available valid execution strategy
- **THEN** values, ordering and admission behavior retain the same defined semantics

### Requirement: Release readiness includes operational recovery
ROM SHALL provide tested CLI recovery, single-writer ownership, documented identity
and secrets setup, versioned extension conformance and locally verified packages.

#### Scenario: Operator recovers a reference deployment
- **GIVEN** a packaged reference application with interrupted work and configured identity
- **WHEN** its operator follows documented diagnosis, upgrade and recovery commands
- **THEN** data and outstanding obligations remain valid without manual database editing
