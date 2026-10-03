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

#### Scenario: The reference application upgrades with pending compensation
- **GIVEN** the version-1 application with a string stock ID and committed pending compensation
- **WHEN** its process exits and offline migration converts Checkout to a typed stock reference
- **AND** the migrated database is backed up and restored into a fresh destination
- **THEN** the current application recovers the compensation through public ROM APIs
- **AND** the unrelated reservation and external folder attachment remain available
- **AND** replay of a version-1 receipt does not add a mutation, event or work item
- **AND** a reference to otherwise deletable empty stock enforces restrict after restore
- **AND** the offline source remains unchanged

### Requirement: Optimization preserves observable semantics
ROM SHALL integrate strategy selection and native planner adapters only with
transactionally maintained indexes, bounded rebuild/recovery and conformance to
the shared authorization, filter, ordering and admission contracts.

#### Scenario: Index recovery changes execution path
- **GIVEN** the same authorized query before and after an interrupted index rebuild
- **WHEN** ROM selects an available valid execution strategy
- **THEN** values, ordering and admission behavior retain the same defined semantics

#### Scenario: An excluded row has a protected sort field
- **GIVEN** an opaque row policy and a row-visible Resource whose sort field is not readable
- **WHEN** a sorted query excludes that row through a predicate, anchor or result limit
- **THEN** the query still returns Denied as the reference evaluator does
- **AND** a cheaper native estimate cannot bypass that disclosure check

#### Scenario: An opaque callback fails before filtering
- **GIVEN** an opaque read callback that panics on an inspected row
- **WHEN** a later predicate would exclude that row
- **THEN** the existing policy contract still reports Panicked and fails the runtime
- **AND** an ID-ordered page that stops before the row does not invoke that callback

#### Scenario: Planning uses coherent admission metadata
- **GIVEN** a native strategy with exact whole-kind row and byte admission metadata
- **WHEN** the adapter estimates and executes a normalized query
- **THEN** metadata, capabilities and execution refer to one coherent native read transaction
- **AND** an optional estimate failure can select the reference path
- **AND** an execution failure propagates without silently retrying another strategy

#### Scenario: Application policy can read storage
- **GIVEN** application-defined authorization or codec code evaluated during observation
- **WHEN** ROM selects a query execution path
- **THEN** the adapter does not invoke that code while holding its native connection mutex
- **AND** observation retains the existing current-authority and generation rechecks

#### Scenario: Explicit uniform read authorization
- **GIVEN** a Resource with an actor-only read policy
- **WHEN** ROM selects rows after query grants and normalization
- **THEN** it evaluates that policy once for this selection attempt before adapter access
- **AND** a denial returns Denied even for an empty or oversized kind
- **AND** accepted selection skips the Resource decode used only for an opaque row policy
- **AND** returned values still pass current disclosure, field and codec checks

#### Scenario: A later definition clears optimization permission
- **GIVEN** a definition with an actor-only read policy and explicit unconditional field grants
- **WHEN** its author supplies a later opaque row policy or field policy
- **THEN** that later declaration clears the corresponding uniform permission
- **AND** ROM requests reference rows when uniform read and field permissions are not both present
- **AND** a later actor-only read policy overrides reads without replacing the write policy

#### Scenario: Adapter returns candidates for a different request
- **GIVEN** a normalized request permitted to use native candidates
- **WHEN** its adapter returns a different request binding, unsupported profile or inconsistent admission metadata
- **THEN** ROM rejects the response instead of disclosing rows or performing a fallback read
- **AND** excessive whole-kind row or byte counts return TooLarge even if few candidates match

#### Scenario: Cost estimates cannot authorize a native path
- **GIVEN** optional native estimates for a query
- **WHEN** the policy mode, scalar profile, request binding or snapshot identity does not match
- **THEN** the shared selector chooses reference execution
- **AND** absent estimates, checked-cost overflow and equal costs also choose reference execution
- **AND** only matching complete candidate support with a strictly lower cost can select native execution

#### Scenario: An indexed mutation rolls back or replays
- **GIVEN** scalar memberships and exact kind counters derived from committed Resources
- **WHEN** a mutation fails before commit or repeats a stored receipt
- **THEN** keys, counters and generation remain unchanged
- **AND** a successful mutation changes them in the same transaction as state, receipt, journal and pending work

#### Scenario: A derived index is corrupt
- **GIVEN** valid authoritative records and an incomplete or incorrect derived index
- **WHEN** a host opens the store normally
- **THEN** bounded integrity validation rejects it instead of serving incomplete results
- **AND** explicit offline rebuild can publish a validated fresh destination without changing the source
- **AND** authoritative corruption or an exceeded complete validation budget prevents publication

#### Scenario: Maintenance reconstructs an index
- **GIVEN** an offline source with registered kinds, including kinds omitted by the next application
- **WHEN** restore, migration, retention, upgrade or explicit index rebuild publishes a replacement
- **THEN** derived memberships and exact counters cover the complete retained catalog and authoritative rows
- **AND** the replacement has a fresh index identity
- **AND** interruption before publication leaves the destination absent and the source unchanged

#### Scenario: The previous native format has retry epochs
- **GIVEN** a format-6 native source or archive-4 source with valid nonzero retry boundaries
- **WHEN** the host performs an explicit format upgrade
- **THEN** it retains retry boundaries and receipt origins without treating them as epoch-zero legacy data
- **AND** ordinary open rejects that previous format without an implicit upgrade

### Requirement: Query strategy probes preserve the Resource contract
The maintained SQLite adapter SHALL bound optional selectivity probes independently
from logical query limits. It SHALL apply whole-kind admission before probes and
retain the shared semantic eligibility gate before native execution.

#### Scenario: A broad predicate precedes a selective predicate
- **GIVEN** supported predicates within the adapter's bounded inspection set
- **WHEN** a small exact candidate count is found after a saturated probe
- **THEN** the adapter can use the smaller complete candidate set
- **AND** core applies every residual predicate and the same authorization and disclosure rules
- **AND** probe LIMIT clauses do not limit the returned candidate set

#### Scenario: Probe work reaches its configured bound
- **WHEN** the adapter cannot establish an exact count within its bounded probes
- **THEN** it treats saturation as a lower bound, not an exact cardinality
- **AND** it uses a conservative estimate or reference execution without truncating results
- **AND** completed probe costs apply to both remaining execution alternatives

#### Scenario: Native execution is already ineligible
- **GIVEN** a reference-only request or an unsupported query semantics version
- **WHEN** the adapter admits the read
- **THEN** it skips selectivity probes and uses reference execution
- **AND** test-only forced execution cannot override that decision

### Requirement: Runtime ownership excludes competing supervisors
Storage adapters SHALL explicitly support one Runtime owner per shared backing store.
ROM SHALL acquire that claim before storage registration and retain it through accepted work and all Runtime handles.
Adapters without an ownership implementation SHALL reject Runtime construction with Unsupported.

#### Scenario: A wrapper exposes an already owned store
- **GIVEN** a Runtime holds the ownership claim for a backing store
- **WHEN** another builder uses that store directly or through a transparent wrapper
- **THEN** construction returns Conflict before storage registration
- **AND** a wrapper delegates ownership to the same backing store

#### Scenario: A caller stops waiting for accepted work
- **WHEN** a caller or shutdown waiter is cancelled while accepted work still runs
- **THEN** the ownership claim remains held until the work and Runtime handles release it
- **AND** a stopped Runtime clone still prevents replacement construction

#### Scenario: Construction fails after acquisition
- **WHEN** a builder acquires ownership and subsequent storage registration fails
- **THEN** the failed builder releases its claim
- **AND** a corrected builder can acquire the store

### Requirement: Native ownership covers open and offline maintenance
The supported local native profile SHALL exclude competing ROM owners with a persistent operating-system lock.
Offline operations SHALL reserve the source and fresh destination before conversion and retain those guards through publication and reopening.
The core SHALL remain independent of filesystem paths and native lock mechanisms.

#### Scenario: A process exits with committed data
- **GIVEN** a native owner has committed data and still holds its database handle
- **WHEN** that process exits without normal destruction
- **THEN** the operating system releases its ownership lock
- **AND** a replacement owner recovers committed data without deleting the lock sidecar

#### Scenario: A database has an alias
- **WHEN** a competing owner opens a symlink to an owned database
- **THEN** canonical path identity produces the same ownership exclusion
- **AND** unsupported hard-link aliases are rejected before native engine setup
- **AND** exact SQLite ephemeral path sentinels retain independent per-connection stores

#### Scenario: A conversion callback attempts a concurrent open
- **GIVEN** offline maintenance holds source and destination reservations
- **WHEN** conversion or a publication observer attempts either native open
- **THEN** the attempt returns Conflict
- **AND** maintenance uses the guarded canonical paths even if a callback changes the current directory

#### Scenario: Publication is interrupted before staging cleanup
- **GIVEN** the complete destination has been published through a non-overwriting hard link
- **WHEN** the process exits before removing its private staging link
- **THEN** ordinary open rejects the unsupported two-link destination
- **AND** documented recovery verifies exact artifact identity under the destination lock before removing only that staging link
- **AND** recovery preserves the destination and unrelated links

### Requirement: Release readiness includes operational recovery
ROM SHALL provide tested CLI recovery, single-writer ownership, documented identity
and secrets setup, versioned extension conformance and locally verified packages.

#### Scenario: Operator recovers a reference deployment
- **GIVEN** a packaged reference application with interrupted work and configured identity
- **WHEN** its operator follows documented diagnosis, upgrade and recovery commands
- **THEN** data and outstanding obligations remain valid without manual database editing

### Requirement: Operator work views use current authority and bounded projections
ROM SHALL deny operator inspection and control unless the host explicitly authorizes them.
Public work views SHALL omit frozen values, action inputs, provider payloads, credentials and raw internal identities.
ROM SHALL bound the complete ledger read and public response independently, and authorize records before visible pagination.

#### Scenario: Authority changes between pages
- **GIVEN** an operator previously received a work page or control receipt
- **WHEN** current authority no longer permits its disclosure
- **THEN** the next request does not disclose hidden handles, counts, payload fragments or the old receipt
- **AND** possession of a cursor or work handle grants no authority

#### Scenario: The view cursor refers to an older storage generation
- **WHEN** an operator continues a page after restore changes the storage generation
- **THEN** ROM returns HistoryGap instead of silently restarting pagination
- **AND** an oversized complete result returns TooLarge rather than truncated JSON

### Requirement: Operator controls have atomic version and identity arbitration
ROM SHALL compare storage generation and a monotonic work revision before accepting a new control.
Every accepted work change SHALL invalidate the previous work version, even when its claim generation does not change.
The adapter SHALL atomically persist the control transition and its bounded durable receipt.

#### Scenario: Delivery changes without a new claim
- **GIVEN** an operator holds a work version from before a delivery-state update
- **WHEN** that operator submits a new control with the old version
- **THEN** the adapter rejects the stale control
- **AND** revision overflow rejects the complete candidate without changing work or budgets

#### Scenario: A control commits but its response is lost
- **WHEN** the same principal repeats the exact request with the same idempotency key
- **THEN** current authorization precedes disclosure of the original control receipt
- **AND** receipt replay precedes comparison with the work version changed by that control
- **AND** changed input under that key returns IdentityMismatch

#### Scenario: An operator requests another attempt
- **WHEN** authorized retry makes unchanged work eligible for the ordinary worker
- **THEN** payload, execution identity, cause, start time and consumed budgets remain unchanged
- **AND** active leases and exhausted original budgets prevent unsafe retry
- **AND** a stale worker cannot overwrite a newly controlled generation

### Requirement: Delivery reconciliation preserves declared guarantees
ROM SHALL preserve the existing at-least-once default and support an explicit reconciliation-before-retry profile.
Only host-registered verification SHALL supply trusted external outcomes; client request data SHALL NOT assert those outcomes.
Verifier calls SHALL run outside native transactions and the core commit gate, with bounded supervision.

#### Scenario: An uncertain delivery uses the hold profile
- **WHEN** timeout, unknown outcome or restart interrupts delivery acknowledgement
- **THEN** the work waits for reconciliation without another send
- **AND** current authority and the exact work version are checked again after verification

#### Scenario: A provider lookup is inconclusive
- **WHEN** verification returns Unresolved or only an eventually consistent lookup miss
- **THEN** ROM makes no work transition and grants no resend permission
- **AND** Accepted completes without sending while terminal NotAccepted permits only a budget-valid unchanged retry

#### Scenario: The application keeps its existing delivery profile
- **WHEN** an unknown delivery outcome occurs under the at-least-once profile
- **THEN** existing budgeted retry remains available
- **AND** duplicate risk remains explicit unless the provider deduplicates the stable delivery identity

### Requirement: Operator CLI preserves generic Resource semantics
CLI operator commands SHALL call the same core control contract as embedded callers and optional transports.
Controls SHALL use exact request files and validate response protocol, target and request identity before reporting success.
Compensation SHALL remain an explicit ordinary Resource action without generic rollback or budget reset.

#### Scenario: A server returns valid JSON for another work item
- **WHEN** the response has the wrong handle, operation identity or protocol version
- **THEN** CLI rejects it before reporting control success
- **AND** interruption remains responsive when output pipes are blocked

#### Scenario: Maintenance preserves operator evidence
- **WHEN** backup, restore, migration or retention processes operator metadata
- **THEN** durable control receipts and declared delivery profiles remain validated and retained
- **AND** explicit format upgrade initializes prior-format work without silently changing its delivery guarantee

### Requirement: A release identity profile uses real provider evidence
ROM SHALL provide one reproducible configured provider profile that uses its unchanged public verification and identity APIs.
The profile SHALL preserve provider-independent core behavior and generic transport routes.
It SHALL distinguish actual-provider interoperability from synthetic failure fixtures and deployment claims outside the tested profile.

#### Scenario: An actual service token reaches an ordinary Resource
- **GIVEN** a pinned configured OAuth provider and an explicit provider, User and IdentityLink in ROM
- **WHEN** the actual CLI submits a valid service token over the configured HTTP adapter
- **THEN** the host verifies the original provider evidence and binds current identity state before the authorized Resource operation
- **AND** unlinked, expired or incorrectly bound evidence cannot disclose Resource data

### Requirement: Authentication acquisition has bounded owned execution
An asynchronous transport authentication callback SHALL use the same generic request path as the synchronous callback.
The deployment host SHALL bound authentication admission, credential acquisition and caller waiting.
Caller cancellation or timeout SHALL NOT release admission while accepted verification remains active.

#### Scenario: A caller leaves during blocking verification
- **GIVEN** an admitted authentication job that is still performing bounded blocking acquisition
- **WHEN** the caller disconnects or reaches its response deadline
- **THEN** the job retains its permit until acquisition and identity binding finish or fail
- **AND** host drain remains able to await that job after a previous drain waiter is cancelled

### Requirement: Provider configuration cannot grant arbitrary acquisition authority
The host SHALL approve provider settings and map opaque credential references to explicitly approved secret material.
Resource configuration SHALL NOT directly authorize arbitrary file paths or external endpoints.
The local Linux profile SHALL validate the opened regular-file handle and private permissions without blocking on FIFO input.
Credential bytes SHALL NOT enter Resource state, journal, receipts, archives or diagnostics.

#### Scenario: Configuration changes while evidence is obtained
- **GIVEN** verification bound to one provider Resource revision
- **WHEN** an authorized local maintainer changes that revision before identity binding
- **THEN** the old activation cannot produce a currently authorized Actor
- **AND** replacement preparation failure denies authentication instead of restoring stale configuration

### Requirement: Local provisioning is explicit and restartable
The reference host SHALL provision identity Resources through ordinary authorized mutations with stable exact identities.
Serving SHALL exclude provisioning authority from network authentication and SHALL NOT use a first-caller enrollment rule.
The local maintenance command SHALL validate configured target keys and expected revisions before applying identity changes.

#### Scenario: Provisioning stops between Resource commits
- **GIVEN** an explicit local provisioning sequence with at least one committed step
- **WHEN** the host restarts and repeats the original requests
- **THEN** accepted steps replay without duplicate state or events and remaining steps can finish
- **AND** changed input under an accepted identity is rejected
- **AND** the procedure does not claim a multi-Resource atomic transaction
