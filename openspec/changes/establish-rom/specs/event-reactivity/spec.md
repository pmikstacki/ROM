# event-reactivity

## Purpose

Live resource reads, committed events and recoverable reactions. These are observable behaviors of the same resource, with distinct delivery contracts.

## ADDED Requirements

### Requirement: Committed facts only
Events SHALL describe committed transitions and SHALL NOT announce successful state changes before commit.

#### Scenario: Committed facts only contract
- **GIVEN** a transaction fails
- **WHEN** an event subscriber reads the journal
- **THEN** no event from the failed transaction is observable

### Requirement: Recoverable consumption
Reaction progress SHALL survive process restart and permit safe retry of unacknowledged work.

#### Scenario: Recoverable consumption contract
- **GIVEN** a reaction stops before acknowledging an event
- **WHEN** the process restarts
- **THEN** the pending event can be delivered again with the same identity

### Requirement: Reaction chains preserve prior commits
A committed mutation MAY trigger registered reactions that submit actions for other Resources. Failure of a downstream action SHALL NOT implicitly revert earlier committed mutations in the chain. Retryable failed steps SHALL remain recoverable under an explicit bounded retry policy. Exhausted or non-retryable work SHALL have an inspectable terminal outcome. Any compensation SHALL be a separate authorized action, not an implicit rollback of committed history.

#### Scenario: A downstream mutation fails
- **GIVEN** a mutation of Resource A committed and triggered a reaction targeting Resource B
- **WHEN** the action on B fails with a retryable error
- **THEN** A retains its committed revision and the downstream work remains eligible for bounded retry
- **AND** no successful mutation event for B is published before its commit

### Requirement: Ordered resource history
Committed events SHALL preserve revision order within each resource.

#### Scenario: Ordered resource history contract
- **GIVEN** successive revisions of a resource commit
- **WHEN** a consumer reads that resource history
- **THEN** events are returned in revision order

### Requirement: Shared action contract
Reactions SHALL request resource changes through the core action contract.

#### Scenario: Shared action contract contract
- **GIVEN** a reaction receives an event
- **WHEN** its resulting action fails validation
- **THEN** no partial resource change is committed

### Requirement: Generic live reads
Standard resource reads SHALL have a live form supplied by the framework from the resource definition and query. Applications SHALL NOT need a separate per-kind subscription resolver or broadcaster. Committed changes affecting query membership SHALL cause the authorized result to converge to the current query result, including additions, removals and deletions.

#### Scenario: A mutation changes filter membership
- **GIVEN** a client observes resources whose completed field is false
- **WHEN** an action commits completed as true on a matching resource
- **THEN** the live result removes that resource without application-specific subscription code

### Requirement: Snapshot and subscription consistency
Starting or resuming a live read SHALL establish a consistent snapshot/change boundary or detect the gap and refresh. The runtime SHALL NOT silently miss a committed change between the initial read and subscription setup. Conservative dependency invalidation MAY be used before more precise tracking is implemented.

#### Scenario: A write races subscription setup
- **GIVEN** a resource changes after the initial read and before live delivery is established
- **WHEN** the subscription becomes active
- **THEN** the change is reflected through delivery or a refreshed snapshot

### Requirement: Distinct delivery semantics
Live reads MAY coalesce intermediate results. Committed-event consumers SHALL have explicit cursor, ordering, retention and gap semantics and SHALL NOT depend on best-effort in-memory notifications for recovery. A slow consumer SHALL trigger bounded buffering, recovery or termination rather than unbounded memory growth.

#### Scenario: A transient notification is lost
- **GIVEN** a committed event exists but its in-memory notification is lost
- **WHEN** a durable consumer resumes from its last acknowledged cursor
- **THEN** the event remains recoverable within the declared retention contract

### Requirement: Authorized live projections
Live delivery SHALL apply the resource's current authorization and field projection rules under a documented authorization freshness contract. Dependency changes that affect visibility SHALL invalidate or terminate the subscription as required by that contract.

#### Scenario: Subscription access is revoked
- **GIVEN** an active subscription and a policy change that removes its access
- **WHEN** the authorization freshness boundary is reached
- **THEN** subsequent protected results are withheld and the subscription is terminated or recomputed safely
