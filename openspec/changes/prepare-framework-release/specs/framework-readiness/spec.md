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
