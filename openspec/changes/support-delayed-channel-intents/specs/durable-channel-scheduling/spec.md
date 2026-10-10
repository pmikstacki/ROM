## ADDED Requirements

### Requirement: Frozen channel eligibility

The framework SHALL provide `Channel::intent_at(payload, not_before_unix_seconds)` for an immutable first eligible delivery time.
Existing `intent(payload)` SHALL remain immediately eligible. The framework SHALL commit timing with its Resource mutation and intent.

#### Scenario: Delayed committed notification

- **WHEN** an action commits a delayed notification
- **THEN** its PendingWork SHALL retain the requested floor
- **AND** no claim, delivery-start record, or external delivery SHALL occur before that floor
- **AND** rejected actions SHALL create no notification work

#### Scenario: Duplicate and changed timing

- **WHEN** the same frozen pending identity is admitted again
- **THEN** equal timing SHALL preserve the original work unchanged
- **AND** changed timing SHALL return `IdentityMismatch` without overwriting the work

### Requirement: Scheduling preserves lifecycle budgets

The framework SHALL preserve the frozen floor through retry, operator scheduling, restart, and restore.
A delay SHALL NOT increase age, attempt, root-work, fanout, record, or byte limits.
Current authority, definition versions, lease fencing, and reconciliation SHALL remain decisive.

#### Scenario: Restored active claim

- **WHEN** an archive restores an active claim
- **THEN** the old claim SHALL be fenced
- **AND** the original floor, consumed attempts, and root usage SHALL remain
- **AND** unknown effects requiring reconciliation SHALL NOT become ordinary fresh delivery

#### Scenario: Age expires before eligibility

- **WHEN** original maximum age expires before the scheduling floor
- **THEN** the work SHALL become terminal with `Age` without an early claim or delivery
- **AND** terminal handling SHALL consume no delivery attempt

#### Scenario: Backward clock and arithmetic overflow

- **WHEN** a transition uses a time below the frozen floor or requires overflowing lifecycle arithmetic
- **THEN** the transition SHALL fail without publishing a partial candidate

### Requirement: Explicit scheduling format upgrade

New native databases SHALL use format 9. New logical archives SHALL use archive 7 and storage 9.
Ordinary readers SHALL reject predecessor markers. Explicit upgrades SHALL publish only into fresh destinations.
Format-8 and archive-6 conversion SHALL preserve existing operator and delivery metadata.
Older markers SHALL NOT admit scheduling metadata that they cannot represent.

#### Scenario: Populated predecessor upgrade

- **WHEN** a valid populated native-format-8 source or archive-6/storage-8 source is explicitly upgraded
- **THEN** rows, receipts, events, effects, tombstones, operator receipts, and original Work budgets SHALL remain
- **AND** absent scheduling metadata SHALL mean no delayed floor
- **AND** source bytes SHALL remain unchanged

#### Scenario: Unsupported old reader or contradictory legacy metadata

- **WHEN** an old format-8 reader opens a new format-9 database
- **THEN** it SHALL reject the database before mutation
- **WHEN** a predecessor source contains scheduling fields under an old marker
- **THEN** explicit conversion SHALL fail without replacing source or destination data
