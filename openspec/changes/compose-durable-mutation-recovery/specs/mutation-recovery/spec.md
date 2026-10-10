## ADDED Requirements

### Requirement: Host-selected durable pending intent

The recovery helper SHALL use host-selected storage with atomic comparison and a host-supplied durable principal binding.
It SHALL NOT select localStorage or process-global storage by default.
It SHALL persist the exact accepted invocation before dispatch. Stored data SHALL NOT grant authority or contain credentials.

#### Scenario: Save response is lost

- **WHEN** a real HTTP mutation commits but its acknowledgement is dropped
- **THEN** the accepted command remains recoverable with its original expected revision, idempotency identity and retry epoch
- **AND** reload restores an unknown outcome without treating it as an uncommitted mutation

#### Scenario: Storage admission fails

- **WHEN** storage write or atomic comparison fails before dispatch
- **THEN** no mutation request is sent
- **AND** the helper reports a storage error without replacing the accepted identity

### Requirement: Existing receipt recovery under current authority

Retries SHALL use existing invoke and receipt semantics. They SHALL NOT add a parallel receipt repository.
Restore SHALL create a new prepared object from the saved exact invocation in the current client generation.

#### Scenario: Same principal renews session

- **WHEN** authority, principal kind and subject remain the same but client generation and CSRF change
- **THEN** the saved command can be retried in the current generation
- **AND** no generation, CSRF or credential value is persisted as durable identity

#### Scenario: Principal changes

- **WHEN** the host switches principal or logs out
- **THEN** the previous command is quarantined and disclosed draft/result content is cleared
- **AND** no old command is submitted under the new principal

#### Scenario: Receipt disclosure is denied

- **WHEN** current authority denies a retry after an unknown result
- **THEN** the latest response is shown as rejected
- **AND** earlier commit knowledge remains unknown
- **AND** the helper does not silently create a replacement identity

#### Scenario: Retry epoch is no longer replayable

- **WHEN** the server rejects the saved epoch because its retained history is unavailable
- **THEN** the helper retains the saved epoch and exposes the unresolved failure
- **AND** it does not advance the epoch or reconstruct a receipt

### Requirement: Draft isolation and navigation composition

The latest unaccepted draft SHALL remain separate from the accepted command.
A later draft SHALL NOT change an unknown command's bytes, key, expected revision or operation.
Navigation SHALL remain host-owned and SHALL have observable unresolved-intent and draft state.

#### Scenario: Repeated edits during uncertainty

- **WHEN** draft A is accepted and the user edits B then C while A is unknown
- **THEN** A remains the exact retry target and C remains the latest draft
- **AND** confirmation of A does not dispatch C automatically
- **AND** explicit acceptance of C uses a new host-selected identity and expected revision

#### Scenario: Navigation is blocked

- **WHEN** the application attempts to leave with a draft or unresolved mutation
- **THEN** the host can retain the view and draft and offer retry or explicit local discard
- **AND** local discard does not claim server cancellation or rollback

### Requirement: Bounded public consumer acceptance

The implementation SHALL preserve exact wire values and reject malformed saved records before dispatch.
It SHALL pass actual HTTP recovery tests on SQLite and redb and an independently installed Svelte consumer in Chromium and WebKit.

#### Scenario: Exact wire values survive reload

- **WHEN** a saved command contains large integers, decimal-token categories, null, missing, false, zero, empty text or removal
- **THEN** its restored invocation serializes to the exact originally accepted wire bytes
- **AND** invalid format versions, excess fields, limits and corrupted wire data fail without dispatch

#### Scenario: End-to-end independent consumer

- **WHEN** an extracted consumer saves, loses acknowledgement, reloads and retries against each real adapter
- **THEN** one logical mutation produces one committed event bundle and receipt outcome
- **AND** the browser uses only supported public package entries and isolated fixture credentials
