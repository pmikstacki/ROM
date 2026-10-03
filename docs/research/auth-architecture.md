# Provider-neutral authentication and authorization

Research date: 2026-10-02. This report supplies design input and recommendations. It does not describe an implemented integration or an approved dependency list. The review used primary sources. No compatibility build, cryptographic audit, or provider interoperability test was performed. Proposed observable requirements are in [design-provider-neutral-auth](../../openspec/changes/design-provider-neutral-auth/proposal.md).

## Recommendation

Keep credential verification in adapters. Keep access enforcement in ROM's core. An adapter translates verified evidence into a small, immutable actor context. Every core operation consults a host-supplied authorizer, with denial as the default.

Start with a local Rust policy implementation. Make a policy engine optional. This preserves the existing single-resource domain model. Principals and authorization decisions are supporting values, not another mandatory domain entity or user database.

The practical interoperability promise is **documented adapter contracts and tested provider profiles**, not compatibility with every identity system. OIDC/OAuth cover many providers; API keys, mTLS, trusted gateways, or proprietary systems need their own evidence verification and mapping. The host chooses which issuers, audiences, mappings, and adapters it trusts. HTTP and RabbitMQ remain separate integrations; neither belongs in the core dependency graph.

## Authentication choices

| Boundary mechanism | Verified upstream behavior | ROM recommendation and tradeoff |
| --- | --- | --- |
| Local JWT access-token verification | RFC 9068 defines an OAuth access-token profile, including signed tokens and validation by the resource server. | Start here when a provider documents a compatible access-token profile. Low per-request network cost; revocation freshness is limited by token lifetime and any separately configured revocation checks. A valid signature alone is insufficient. |
| OAuth token introspection | RFC 7662 defines a protected endpoint returning `active` and optional identity/authorization metadata. | Support opaque tokens through an adapter. Require `active=true` and enough verified metadata to bind the token to ROM and its principal. Network availability, credentials, latency, and cache freshness become operational concerns. |
| OIDC login | OIDC identifies an end user to a relying party using an ID token. | Relevant when the embedding application also owns login/session handling. Do not accept an ID token as a ROM API access token. Keep redirects, cookies, sessions, logout, and refresh-token storage outside the core. |
| Custom or trusted upstream evidence | No single protocol guarantees arbitrary provider compatibility. | Allow trusted host implementations to create the same actor context after their own verification. A caller-supplied principal header or message field is not verified evidence. Document the actual upstream trust boundary. |

Sources: [RFC 9068, access-token validation](https://www.rfc-editor.org/rfc/rfc9068.html#section-4), [RFC 7662, introspection](https://www.rfc-editor.org/rfc/rfc7662.html#section-2), [OIDC Core, ID tokens](https://openid.net/specs/openid-connect-core-1_0.html#IDToken).

For a browser login adapter, use authorization code plus PKCE and the applicable state/nonce checks. Do not turn legacy grants appearing in a library API into recommended product flows. Current OAuth security guidance rejects the password grant and strengthens authorization-code flow requirements. [RFC 9700](https://www.rfc-editor.org/rfc/rfc9700.html#section-2).

Declare whether each profile accepts bearer or sender-constrained access tokens. DPoP or mTLS-bound tokens require the corresponding request/key proof; reject unsupported proof requirements instead of treating such a token as an ordinary bearer credential. Sender constraints are recommended by current OAuth security guidance, but implementing them is a separate adapter capability. [RFC 9700, access-token replay prevention](https://www.rfc-editor.org/rfc/rfc9700.html#section-2.2.1).

## Verification profile, keys, and freshness

The proposed JWT adapter profile requires an exact configured issuer, an intended ROM audience, signature verification with issuer-bound keys, an explicit permitted algorithm set, required expiry, and not-before validation when present. It also validates the configured access-token type/profile, preventing substitution of an ID token. An RFC 9068 profile checks `at+jwt` or `application/at+jwt`; providers using another format need a separately documented profile. Required claims and claim values are separate checks. [RFC 9068 §4](https://www.rfc-editor.org/rfc/rfc9068.html#section-4), [JWT security guidance](https://www.rfc-editor.org/rfc/rfc8725.html#section-3).

An unverified issuer or key identifier can select among already trusted configurations. It cannot create a new trusted issuer. Never fetch arbitrary token-provided key URLs. Bound token size, request time, key refresh frequency, and clock skew. Discovery must resolve an explicitly trusted provider configuration and validate its issuer. These are ROM adapter recommendations applying the JWT guidance against algorithm confusion, substitution, and attacker-controlled lookup URLs. [RFC 8725](https://www.rfc-editor.org/rfc/rfc8725.html#section-3.10).

OIDC describes JWKS rotation through published key sets and refresh on an unfamiliar `kid`. For ROM, keep a bounded per-issuer cache, coalesce refreshes, rate-limit unknown-key retries, and reject unresolved keys. Specify a maximum cache age and any permitted use of previously fetched keys during outages; do not retain removed keys forever. Normal rollover and emergency compromise require distinct runbooks. [OIDC key rotation](https://openid.net/specs/openid-connect-core-1_0.html#RotateSigKeys).

Introspection uses TLS and authenticated access to the configured endpoint. `active` does not by itself prove that ROM is the intended recipient: require audience or another explicit provider contract establishing resource binding, plus unambiguous identity and principal-kind mapping. Missing information is a configuration/integration failure, not permission to infer elevated access. Cache for a bounded duration no later than reported expiry; fail closed on an unavailable endpoint once usable cached evidence expires. RFC 7662 explicitly notes the revocation window caused by caching. [RFC 7662 security considerations](https://www.rfc-editor.org/rfc/rfc7662.html#section-4).

## Smallest useful interfaces

These are conceptual signatures, not compilable Rust or a settled async/ownership API:

```text
PrincipalId = { authority, subject, kind: Human | Service }
ActorContext = { principal, client?, trusted_attributes, valid_until }

Adapter.authenticate(credential, request_evidence) -> ActorContext | AuthnError
Authorizer.decide(actor, operation, target_snapshot, request_context)
    -> Permit | Deny | PolicyError

Operation = Execute(action, arguments, changed_fields)
          | Read(requested_fields)
          | Discover(query_shape)
          | Subscribe(subscription_scope)
          | DeliverEvent(event_metadata, requested_fields)

Core.execute(actor, action)
Core.read(actor, resource, fields)
Core.subscribe(actor, selector)
```

`authenticate` is an adapter-local seam; requiring every adapter to share one credential enum would pull protocol details into ROM. The public core needs only actor values and one policy decision seam. `authority` is a stable host-configured trust namespace bound to the verified issuer; `subject` is an opaque identifier within it. Do not identify principals by email, display name, or unqualified subject. `client` records the verified calling application where distinct from the end user. Roles, scopes, tenant membership, or assurance are allowlisted, namespaced attributes whose provenance and freshness the host defines. Never copy arbitrary token claims into trusted attributes.

The actor context is immutable. An ordinary request cannot deserialize it as trusted state. Explicit host integration APIs construct it. It is not an in-process security sandbox: native Rust extensions and the embedding host are trusted code and can access their own memory and storage. Raw tokens, passwords, private keys, client secrets, cookies, and refresh tokens do not belong in the context.

Use service identities for workers, observers, schedulers, and machine clients. Human-delegated calls preserve both the human and client identities. Distinguish human and service namespaces using verified provider semantics; a matching `sub` or a `client_id` alone is not a reliable human/service discriminator. OAuth guidance documents the resulting impersonation risk. [RFC 9700 §4.15](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.15).

Reactions execute with their configured service identity and current permissions. Store original actor attribution and causation only as audit context. Replaying an event must never reconstruct the originating caller's credentials or revive their authority. Delegation on behalf of a user is a separate future contract, with explicit scope and expiry.

## Authorization and transaction boundaries

The following are proposed ROM choices, not guarantees supplied by OIDC or an authorization crate:

1. Deny unless an explicit policy grants the operation. Missing policy, missing required attributes, expired actor context, policy failure, and unavailable required policy data cannot permit access. Anonymous access, if offered, requires an explicit principal category and policy rather than a missing actor bypass.
2. Authorize action, resource, and field access. For mutations, check every supplied writable field and the resulting transition, including server-derived changes relevant to policy. Reject a forbidden field instead of silently dropping it. Action authorization does not automatically grant permission to read the resulting resource.
3. Authorize reads, query predicates, ordering, projections, counts, and subscriptions. Otherwise filtering on a hidden field or exposing a total count leaks information despite redacted output. Start with exact resource reads and reject unsupported safe query shapes; define authorized pagination before adding broad listing.
4. Authenticate and authorize before releasing an idempotency result, validation details, resource data, or an event. Scope idempotency by authority, principal kind/subject, tenant where applicable, operation/target, and key; compare canonical input. Keep read permissions separate for cached outcomes. Revoked access cannot retrieve a previously authorized sensitive response; it also cannot cause the committed transition to run again. Deletion retries need a safe minimal outcome and authorization metadata retained with the deduplication record, not a dependency on a still-existing resource.
5. Evaluate state-dependent mutation policy against the resource revision actually committed. Retry or deny after conflicting revisions. Policy data outside the transaction needs an explicit freshness/version contract; do not promise instantly atomic revocation across an external policy store and ROM's database.
6. Check subscriptions at creation and event delivery, with expiry and policy refresh boundaries. A durable journal is internal persistence, not an automatically authorized feed. Event field projection needs its own decision; current resource read access alone need not grant historical access. Revocation cannot retract data already delivered, but must stop subsequent delivery after the documented freshness boundary.
7. Persist only necessary actor identifiers, policy decision/version references, and causation in audit metadata. Do not put credentials or the whole actor context in resource events, idempotency records, or logs. Protected event fields remain subject to the separate protected-data/retention design.

A small synchronous authorizer over an immutable policy snapshot is the recommended first implementation. Host code refreshes external policy data outside transition computation. A remote policy decision point can later implement a controlled preflight/snapshot workflow with explicit failure and consistency behavior. Authorization must not introduce uncontrolled I/O or external side effects inside the resource transition.

## Rust candidates

| Candidate | Documented capability | Proposed placement |
| --- | --- | --- |
| [`openidconnect`](https://docs.rs/openidconnect/latest/openidconnect/) | Typed OIDC clients, discovery, ID-token verification, authorization-code examples. | Optional login adapter when login is needed. Not the complete API access-token verifier. Its documentation warns to disable HTTP redirects to reduce SSRF exposure. |
| [`oauth2`](https://docs.rs/oauth2/latest/oauth2/) | OAuth clients, client-credentials exchange, introspection, revocation. | Optional introspection adapter and machine-client token acquisition. Core must not become an OAuth client merely to execute an action. |
| [`jsonwebtoken`](https://docs.rs/jsonwebtoken/latest/jsonwebtoken/struct.Validation.html) | Signature/claim validation configuration. | Candidate for a narrowly scoped JWT access-token adapter. Explicitly require issuer/audience/subject/expiry and set permitted algorithms and skew. Current validation docs show `nbf` checking defaults off, required claims default to `exp`, and issuer/audience values require configuration; defaults are not the ROM profile. Check additional profile claims/header type separately. |
| [`josekit`](https://docs.rs/josekit/latest/josekit/) | JWS, JWE, JWK, and JWT support. | Alternative when a concrete provider requires JOSE capabilities beyond signed JWTs. Greater feature surface is not a reason to add JWE to the first adapter. |
| [`cedar-policy`](https://docs.rs/cedar-policy/latest/cedar_policy/) | Embedded policy evaluation with principal/action/resource/context and validation support. | Optional typed policy adapter if application policy complexity warrants it. Cedar defaults to deny and satisfied forbids override permits, but individual evaluation errors are skipped. A ROM adapter must inspect diagnostics and turn policy errors into denial under the proposed contract. [Cedar evaluation semantics](https://docs.cedarpolicy.com/auth/authorization.html). |
| [`casbin`](https://docs.rs/casbin/latest/casbin/) | Rust authorization enforcer; Casbin models support subject/object/action and RBAC relationships. | Alternative for applications already using Casbin policy models. Select explicit deny/error and role-mapping semantics; do not adopt a root-name bypass. It does not authenticate users. [Casbin scope](https://casbin.apache.org/docs/overview/). |

Adopt one verifier and a plain Rust authorizer first. Engine adapters must map ROM concepts without adding mandatory user/role/resource repositories. Before dependency selection, verify release versions, features, crypto backend, licenses, MSRV, maintenance, and build compatibility; these sources establish capabilities, not a tested stack.

## Decisions to resolve before implementation

- Select the first provider and its access-token profile, then a second independently configured provider to test that mapping is genuinely replaceable.
- Decide whether login/session management belongs in the first integration; backend API token verification alone does not require it.
- Choose principal identity linking and tenant isolation rules; never merge accounts solely because verified email strings match.
- Define claim/policy/key cache ages, bounded clock skew, stream expiry, and compromise invalidation behavior.
- Specify field paths, derived-field checks, safe read projections, query/cursor/count behavior, and historical-event authorization.
- Specify deduplication authorization metadata and deletion/revocation retry outcomes without exposing stored results.
- Decide policy snapshot/version ownership and how current authorization is reconciled with transactions, retries, and external relationship data.
- Choose concrete Rust ownership and sync/async signatures through a minimal vertical slice; do not freeze the conceptual interfaces above prematurely.
