## ADDED Requirements

### Requirement: One reusable Resource path
ROM SHALL provide one maintained library path for derived and manually registered
Resources, including managed User and settings Resources. Applications MUST NOT
need a repository, controller or subscription implementation for each kind.

#### Scenario: Downstream author adds an unrelated kind
- **GIVEN** a packaged consumer declaring a task Resource through public APIs
- **WHEN** it registers a settings Resource and a custom field codec
- **THEN** the same action, persistence, query and observation machinery handles both
- **AND** no infrastructure implementation is added for the new kind

### Requirement: Integrated durable guarantees
ROM SHALL commit state, revision, receipt, events and pending effects atomically
through its persistence contract. Each advertised database adapter MUST pass the
same conformance suite. An unresolved commit MUST be distinguishable from rollback.

#### Scenario: Restart after an acknowledged response is lost
- **GIVEN** an action committed its complete bundle but the caller lost its reply
- **WHEN** the process restarts and an authorized caller repeats the same identity
- **THEN** ROM resolves the recorded result without a second mutation or event
- **AND** conflicting input with that identity is rejected

### Requirement: Bounded supervised execution and observation
Accepted work SHALL retain ownership until actual completion. Reads and streams
MUST enforce configured admission and byte/row limits. Authorization MUST be
checked before disclosure using the supported profile's current authority.

#### Scenario: Client disconnects during execution
- **GIVEN** an accepted action still executing after its caller disconnects
- **WHEN** another caller requests capacity and shutdown begins
- **THEN** the first action still occupies its execution permit
- **AND** shutdown rejects new intake and reports or drains accepted work

#### Scenario: Permission changes before live delivery
- **GIVEN** a live result became invalidated before its caller polled it
- **WHEN** access is revoked or the actor expires before that poll
- **THEN** ROM does not disclose the old buffered values
- **AND** the observation terminates or returns an explicitly authorized replacement

### Requirement: Recoverable bounded reaction chains
ROM SHALL schedule reactions durably after commit and invoke downstream actions
through the shared pipeline. It MUST preserve upstream commits on downstream
failure and expose terminal work when retry or causal budgets are exhausted.

#### Scenario: Retry and cycle protection coexist
- **GIVEN** A committed and its reaction requests a mutation of B
- **WHEN** B fails transiently, restarts, and later triggers a reaction back to A
- **THEN** retry uses the same durable step identity and preserves A's prior commit
- **AND** unchanged values do not create another change event
- **AND** a continuing oscillation stops at a declared budget with inspectable cause

### Requirement: Protocol independent public invocation
ROM SHALL define invocation, live observation and journal delivery separately.
An HTTP adapter MUST map those contracts without defining Resource behavior.

#### Scenario: Embedded and HTTP clients perform the same action
- **GIVEN** the same registered kind, trusted identity and semantic command
- **WHEN** callers use Rust directly and the actual HTTP adapter
- **THEN** both paths enforce the same validation, policy, revision and idempotency rules
- **AND** the core builds without HTTP or database driver dependencies

### Requirement: Auditable MVP completion
MVP completion SHALL include a traceable inventory of every planned experiment,
open question and recommendation, integrated tests, dependency review and a
packaged consumer. Unsupported profiles MUST be documented and rejected rather
than presented as implemented. A human usability study MUST NOT be claimed from
agent-written examples alone.

#### Scenario: Completion is assessed
- **GIVEN** all standalone experiment reports are available
- **WHEN** the integrated consumer or required conformance tests still fail
- **THEN** the MVP remains incomplete
- **AND** the remaining work is listed with reproducible evidence
