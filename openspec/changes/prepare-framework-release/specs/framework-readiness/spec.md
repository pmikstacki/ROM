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
