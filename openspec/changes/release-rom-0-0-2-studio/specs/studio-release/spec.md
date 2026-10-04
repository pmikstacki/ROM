## ADDED Requirements

### Requirement: Generic Studio uses the public Resource contract
Studio SHALL use reusable descriptor-driven components for registered Resources, fields, queries, and action inputs.
Studio MUST NOT require kind-specific controllers, repositories, or subscription implementations.

#### Scenario: A held-out Resource appears without infrastructure code
- **GIVEN** two registered Resources and reusable Studio components
- **WHEN** an application registers a third Resource and custom Field through public interfaces
- **THEN** discovery, editing, querying, and observation SHALL operate without new kind-specific infrastructure

### Requirement: Discovery describes authorized action inputs
The core SHALL expose bounded, versioned input descriptions that agree with registered codecs.
Metadata disclosure MUST NOT imply invocation permission or reveal disallowed references.

#### Scenario: A hidden action input remains hidden
- **GIVEN** an actor without discovery permission for an action or its referenced kind
- **WHEN** the actor requests discovery
- **THEN** the response MUST NOT reveal disallowed input metadata

### Requirement: Browser values retain mutation intent
Studio SHALL preserve omitted, null, false, zero, empty, removal, and exact integer values according to the operation contract.

The selected Resource editor SHALL show controls for current authorized values without requiring an operation selection for each ordinary edit. Changing a control SHALL create an explicit field intent. Nullable and optional field actions SHALL appear only when the descriptor permits them. Unknown custom codec versions SHALL remain read-only until an editor is registered.

#### Scenario: Edit a field directly on mobile
- **GIVEN** a selected Resource with a current value and an open mobile inspector
- **WHEN** the user changes one field control
- **THEN** Studio SHALL submit only that field's update
- **AND** it SHALL keep other fields unchanged

#### Scenario: Distinguish explicit field states
- **GIVEN** string, boolean, integer, nullable, and optional fields
- **WHEN** the user sets empty, false, zero, null, or remove
- **THEN** Studio SHALL encode each as its distinct mutation operation
- **AND** an unsupported custom codec SHALL not create an editable input

#### Scenario: Boundary integers round trip
- **GIVEN** fields with i64 and u64 boundary values
- **WHEN** Studio reads and submits these values
- **THEN** the committed values SHALL remain exact without JavaScript Number rounding

### Requirement: Human authentication retains current authority
The optional Studio host SHALL verify its declared OIDC human profile and maintain protected sessions.
The host MUST protect callbacks and unsafe cookie-authenticated requests against forgery and replay.
Current core authorization SHALL govern discovery, data, live updates, and receipt replay.

#### Scenario: Revocation removes current browser data
- **GIVEN** an authenticated browser with an active live query
- **WHEN** current identity authority is revoked or expires
- **THEN** subsequent server disclosure SHALL stop and Studio SHALL clear unauthorized projections when it detects invalid authority

#### Scenario: Another tab logs out with an active stream
- **GIVEN** two tabs share an authenticated session and one tab has an active stream
- **WHEN** the other tab logs out
- **THEN** the host SHALL cancel session-owned delivery and connected clients SHALL clear their projections on detected session invalidation

### Requirement: Browser recovery preserves accepted work
Studio SHALL recover uncertain mutations using their original operation identity and request.
Disconnects MUST NOT release core ownership of accepted work.

#### Scenario: Response loss does not duplicate a mutation
- **GIVEN** a mutation committed before its HTTP response is lost
- **WHEN** Studio recovers the outcome with the original request identity
- **THEN** ROM SHALL return the retained authorized result without another mutation or success event

### Requirement: Studio release has integrated acceptance evidence
Release acceptance SHALL exercise real Studio, HTTP, identity, and persistence on SQLite and redb.
Evidence SHALL distinguish combined journeys from component checks and source inspection.

#### Scenario: Active work survives actual process interruption
- **GIVEN** the serving process has accepted authentication or upload work
- **WHEN** the process receives the tested interruption and the application reopens
- **THEN** the recorded lifecycle guarantees SHALL hold without premature ownership release or duplicate mutation

### Requirement: Local artifacts and durable preview identify accepted source
The release SHALL bind clean source, Rust and frontend lockfiles, static assets, gate profile, inventory, and checksums.
The NixOS preview SHALL serve the accepted Studio assets and real protected API under the configured VPN base path.

#### Scenario: Incomplete frontend assets fail admission
- **GIVEN** a release archive with a missing asset or inconsistent inventory
- **WHEN** the artifact verifier checks extracted contents
- **THEN** verification SHALL fail before publication

### Requirement: Shared filter workspace
Studio SHALL provide quick filters in a popover and a full Filters / Details sidebar. Both presentations SHALL use one controlled filter draft. Editing SHALL NOT execute a query before Apply. Applied query summaries SHALL remain separate from pending edits, including invalid input.

#### Scenario: Switch between filter presentations
- **GIVEN** a browser with a pending filter edit
- **WHEN** the user switches between the popover and sidebar
- **THEN** the field, operator, exact value and invalid input SHALL remain available
- **AND** no query SHALL execute until the user selects Apply

#### Scenario: Keep an open Resource draft
- **GIVEN** a browser with an open Resource draft
- **WHEN** the user applies filters, changes tabs or changes between desktop and mobile layouts
- **THEN** the same draft and captured revision SHALL remain mounted
- **AND** current authorization and stale revision checks SHALL still apply

#### Scenario: Reject an unsupported filter
- **GIVEN** a filter with an unsupported operator, group, field or invalid value
- **WHEN** the user attempts to apply the filter
- **THEN** Studio SHALL reject it before query execution
- **AND** Studio SHALL NOT silently flatten a group or change an exact value

#### Scenario: Close the mobile inspector
- **GIVEN** an open mobile Filters / Details inspector
- **WHEN** the user presses Escape
- **THEN** the inspector SHALL close and return focus to its toolbar control
- **AND** the pending filter and Resource drafts SHALL remain available
