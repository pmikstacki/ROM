# provider-neutral-auth

## Purpose

Provider-independent identity and access enforcement for the resource library. This proposed capability complements the initial action and event contracts without defining a user database or mandatory transport.

## ADDED Requirements

### Requirement: User uses the Resource model

Users represented by ROM SHALL be User resources using the shared definition, action, persistence, authorization and observation contracts. Dedicated account-management views SHALL invoke permitted resource actions rather than use a separate entity engine. Trusted actor context SHALL remain separate from stored profile data, and caller-supplied user identifiers SHALL NOT establish authentication.

#### Scenario: External identity linked to a User resource

- **GIVEN** a trusted adapter verifies an external authority-qualified identity
- **WHEN** an explicit host linking policy resolves that identity to a User resource
- **THEN** user operations use the shared resource pipeline and authentication does not grant unrestricted profile, role or credential administration

#### Scenario: Generic profile editing cannot elevate privileges

- **GIVEN** an actor may edit their own User resource's permitted profile fields
- **WHEN** a request attempts a protected privilege change without permission
- **THEN** the shared authorization contract rejects the mutation without a partial commit

### Requirement: Trusted provider-independent actors

The core SHALL accept actor context through an explicit trusted host integration contract independent of provider protocols. Principal identity SHALL distinguish authority, subject, and human or service kind. The core SHALL NOT treat caller-supplied serialized actor metadata as authenticated evidence.

#### Scenario: Same subject from different authorities

- **GIVEN** two configured authorities verify the same subject string
- **WHEN** their principals reach the core
- **THEN** they remain distinct identities unless an explicit host-owned linking rule maps them together

#### Scenario: Forged actor metadata

- **GIVEN** a transport request supplies an administrator identity in its payload
- **WHEN** the adapter cannot verify evidence for that identity
- **THEN** the request cannot execute as that administrator

### Requirement: Explicit verification profiles

Authentication adapters SHALL establish trusted identity and intended-resource binding before constructing an actor context. JWT adapters SHALL validate configured issuer, audience, signature, permitted algorithms, expiry, applicable not-before, and access-token profile. They SHALL reject ID-token substitution. Introspection adapters SHALL require an active response and sufficient verified identity and resource-binding evidence. Unsupported or ambiguous profiles SHALL fail closed.

#### Scenario: Valid signature for the wrong API

- **GIVEN** an access token has a valid trusted signature but an audience excluding ROM
- **WHEN** it is presented to the ROM adapter
- **THEN** authentication fails before any protected result or resource is returned

#### Scenario: Inactive or insufficient introspection

- **GIVEN** introspection reports an inactive token or omits evidence required by the configured profile
- **WHEN** the adapter processes the response
- **THEN** it creates no authenticated actor context

#### Scenario: Wrong token type

- **GIVEN** a caller presents an OIDC ID token to an API access-token adapter
- **WHEN** the adapter verifies the configured token profile
- **THEN** authentication fails even if its signature is valid

### Requirement: Bounded verification freshness

Adapters SHALL fetch trust metadata only through configured trusted sources and SHALL bound evidence validity, key-cache age, refresh attempts, and clock skew. Unknown keys or unavailable verification dependencies SHALL NOT permit authentication without still-valid evidence under the configured policy. Introspection caches SHALL NOT outlive reported token expiry.

#### Scenario: Key rollover

- **GIVEN** a trusted issuer rotates to an unfamiliar signing key
- **WHEN** verification refreshes its configured key source
- **THEN** only if bounded refresh yields a matching trusted key and all other checks pass, it accepts the token

#### Scenario: Attacker-provided key URL

- **GIVEN** a token contains a key URL outside configured trust sources
- **WHEN** the adapter processes its header
- **THEN** that URL cannot establish a new trust source

### Requirement: Default-deny core enforcement

The core SHALL authorize every protected action, read, discovery operation, subscription, and event delivery through the same host-supplied policy contract regardless of entrypoint. Missing permission, expired actor validity, missing required policy inputs, and policy evaluation failures SHALL NOT grant access. Mutation authorization SHALL use resource state consistent with the committed revision.

#### Scenario: Policy evaluation error

- **GIVEN** a policy engine encounters an evaluation error while another policy permits the operation
- **WHEN** its adapter reports the decision to the core
- **THEN** the operation is denied under the ROM contract

#### Scenario: State changes after authorization

- **GIVEN** mutation permission depends on resource state at revision N
- **WHEN** another action changes the resource before this mutation commits
- **THEN** the stale decision cannot authorize a transition against a different revision without renewed validation and authorization

### Requirement: Field and result protection

The core SHALL enforce field permissions for requested mutations and disclosed results, including derived changes relevant to policy. It SHALL reject unauthorized writes rather than silently ignore them. Queries SHALL NOT expose unauthorized information through predicates, ordering, counts, projections, or cursors; unsupported safe query shapes SHALL be rejected.

#### Scenario: Partially forbidden mutation

- **GIVEN** an action contains both writable and forbidden fields
- **WHEN** the core evaluates field permissions
- **THEN** the action is rejected with no partial commit

#### Scenario: Write access without read access

- **GIVEN** a principal may change a protected field but cannot read it
- **WHEN** the action succeeds
- **THEN** its response does not disclose that field

### Requirement: Authorized idempotency outcomes

The core SHALL scope idempotency identities to principal identity and the applicable isolation and operation scope, compare canonical input, and require current authentication and authorization before disclosing a stored outcome. Denied replay SHALL NOT repeat a committed transition. Retry authorization SHALL remain defined after resource deletion using retained non-secret authorization metadata.

#### Scenario: Permission revoked after commit

- **GIVEN** an action committed and the caller subsequently lost permission
- **WHEN** that caller retries its original idempotency identity
- **THEN** the protected stored outcome is not disclosed and the transition is not repeated

#### Scenario: Another principal reuses a key

- **GIVEN** one principal committed an action with an idempotency key
- **WHEN** a different principal submits that same key
- **THEN** the first principal's outcome is not returned through key matching

#### Scenario: Retry after deletion

- **GIVEN** an authorized deletion committed with an idempotency identity
- **WHEN** the caller retries after the resource no longer exists
- **THEN** retained authorization metadata supports the defined safe outcome without restoring the resource, bypassing authorization, or duplicating the transition

### Requirement: Independently authorized event delivery and reactions

The core SHALL authorize subscriptions and each event delivery, including historical fields, under documented actor and policy freshness bounds. Expired or revoked authority SHALL stop subsequent protected delivery once those bounds are reached. Reactions SHALL submit actions as explicitly configured service principals with current permissions; event attribution SHALL NOT confer original-caller authority.

#### Scenario: Subscriber expires

- **GIVEN** a subscription was authorized but its actor validity has expired
- **WHEN** another protected event becomes available
- **THEN** it is not delivered until valid authority is established

#### Scenario: Replaying an administrator's event

- **GIVEN** an event records an administrator as its originating actor
- **WHEN** a reaction replays that event under a limited service identity
- **THEN** its action is subject to the service's permissions. The event grants no administrator authority.

### Requirement: Credentials excluded from attribution

Actor context passed into the core SHALL exclude credentials. Resource events, stored idempotency outcomes, and diagnostics SHALL NOT contain access tokens, refresh tokens, passwords, client secrets, private keys, or session credentials. Attribution SHALL retain only explicitly selected non-secret metadata and SHALL NOT persist the full actor context or provider claims.

#### Scenario: Authenticated action emits an event

- **GIVEN** an adapter authenticates an action using a bearer token
- **WHEN** the action commits and emits diagnostics and event metadata
- **THEN** selected actor identifiers can appear but the token and complete claims do not

### Requirement: Optional integration dependencies

The core SHALL operate with an in-process trusted actor and Rust authorizer without requiring an OIDC client, HTTP server, broker, or external policy engine.

#### Scenario: Embedded application

- **GIVEN** a host configures a local service identity and Rust authorization policy
- **WHEN** it executes an authorized resource action in-process
- **THEN** no identity-provider network call or transport dependency is required
