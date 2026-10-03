## ADDED Requirements

### Requirement: Invariants apply to every Resource mutation
ROM SHALL offer optional typed transition validation with the actor, actual previous value and normalized candidate value, including absent creation/deletion values. Every create, replace, patch, custom action and deletion SHALL pass this validation before commit. A rejected transition MUST produce no durable mutation, receipt, event or outgoing work. Existing definitions without a validator SHALL retain their behavior.

#### Scenario: Generic mutation cannot bypass an invariant
- **GIVEN** a Resource with a terminal-state invariant
- **WHEN** a typed or transported replacement, patch or custom action violates it
- **THEN** the operation fails without changing the revision, journal or pending work

#### Scenario: Creation and deletion are validated
- **GIVEN** a validator that rejects an invalid initial state or deletion of a protected state
- **WHEN** creation or deletion is invoked
- **THEN** the callback receives an absent previous or candidate value respectively and its rejection prevents commit

### Requirement: Validation preserves runtime authority and replay
Transition validation SHALL NOT grant permissions or bypass revision checks. Callback panics SHALL fail the operation without poisoning subsequent commits. Replaying a matching durable receipt SHALL preserve current authorization checks without revalidating an obsolete transition. Validators MUST be pure, deterministic, bounded and free of runtime reentry or external effects.

#### Scenario: Receipt replay after a later state change
- **GIVEN** a committed action receipt and a later state that would reject that old transition
- **WHEN** the authorized caller repeats the original command identity and input
- **THEN** ROM returns the saved result without rerunning transition validation or adding events

#### Scenario: Validator panic isolation
- **GIVEN** a validator that panics for one candidate
- **WHEN** that mutation is submitted and a valid mutation follows
- **THEN** the first returns a panic error with no commit and the second can commit normally
