# ROM 0.0.2 Studio host contract

This document records the implemented authentication and asset slice of Task 4. Combined blob, browser, and release-package acceptance remains separate.

## Public Rust interface

`StudioHost::new(runtime, config)` creates an optional host. `router()` returns its Axum router. `serve(listener, stop)` owns shutdown.
`shutdown()` closes intake, cancels session observers, drains accepted authentication work, and then closes the generic HTTP binding.

`HostConfig::new(public_origin, base_path, asset_directory, host_actor)` requires an explicit trusted configuration reader.
The host never constructs a trusted Actor from browser data. The application must install its ordinary `IdentityGate` and Resource policies.

Configuration methods add approved `OidcProviderConfig` values, the primary provider, finite limits, and an explicit numeric-loopback HTTP exception.
`HostConfig.settings(id)` reads an ordinary `StudioSettings` Resource. Its `primary_provider` can select only an approved, enabled provider.
Applications register this Resource and its policies with the other definitions. `HostConfig.clock(...)` must match the trusted Runtime clock.
`HostConfig` validates exact origins, canonical base paths, duplicate authorities, endpoints, and nonzero limits before serving.
`OidcProviderConfig` pins authority, label, issuer, client ID, authorization endpoint, token endpoint, JWKS endpoint, and an optional server-only secret.
Mutable IdentityProvider Resources cannot supply new network endpoints. Provider activation must match the approved issuer, client ID, and OIDC profile.

## Browser protocol

The default base path is `/rom-studio/`. Every route is relative to the configured base path.

| Method | Route | Result |
| --- | --- | --- |
| GET | `auth/providers` | `{providers:[{id,label}],primary:null\|id}` |
| GET | `auth/login/{provider}` | Authorization-code redirect with PKCE, retained nonce, and one-use browser-bound state |
| GET | `auth/callback/{provider}` | Consume state, redeem code, verify original ID token, bind current Resources, and redirect to Studio |
| GET | `auth/session` | `{authenticated,generation,csrf_token?,user_id?,expires_at?}` |
| POST | `auth/logout` | Cancel all streams owned by this session and remove its cookie |
| POST | `api/*` | Existing generic ROM HTTP operations, resolved through the session |
| GET | asset path | Inventory-approved static file, or an extensionless Studio navigation fallback |

`generation` is an opaque string. `expires_at` uses Unix seconds. An unauthenticated response omits privileged session fields.
Unsafe requests require the exact configured Origin and `x-rom-csrf`. Login state also requires its separate HttpOnly attempt cookie.
Cookies use HttpOnly, SameSite=Lax, the configured base path, and Secure. Only explicitly approved numeric-loopback tests can omit Secure.
No callback parameter can select a redirect destination. Callback redirects use the configured Studio base path.

## Finite evidence renewal

The OIDC verifier limits each proof to 30 seconds. The host must preserve that limit.
A browser session can retain the original bounded ID token, nonce, and token bindings until the original token expires.
When evidence expires, the host can verify the same original token again against approved current keys and the captured provider configuration.
The host then obtains a new sealed proof and binds it against current IdentityProvider, IdentityLink, and User Resources.
The host never extends an existing Actor stamp. It never issues an OAuth refresh grant or extends the original token expiry.

Every request checks current Resources. Session-owned streams have a finite Actor lease. They cancel on proof expiry, current denial, or logout.
The wrapper cannot renew the Actor captured by the generic HTTP observer. The client revalidates its session and opens a fresh live query.
A journal subscription can separately recover through its journal cursor. A live query reopens with a fresh authorized snapshot.
A failed renewal closes the session. A provider configuration change rejects the captured activation instead of adopting new trust sources.
Raw credentials remain server-side and bounded by both session capacity and original token lifetime. They are never sent to Studio.

## Work ownership and limits

The host owns accepted code exchanges and verification work. Disconnecting a callback caller does not detach that work from shutdown supervision.
Token and JWKS requests use pinned endpoints, disabled redirects, finite deadlines, and streamed byte limits before JSON parsing.
Login attempts have finite count and expiry. State is atomically consumed before code redemption. Session capacity is finite.
Session lookup removes expired entries before admitting a new session. Logout cancels streams independently of browser cookie deletion.

Assets use normalized relative paths and an explicit inventory. The host refuses traversal, encoded path separators, and symlink escapes.
The generic HTTP binding owns Resource request semantics. The host supplies authentication and CSRF boundaries without per-kind controllers.
Blob supervision and upload integration will follow the tested authentication/router slice. Separate component tests cannot prove their combined shutdown behavior.

## Planned checks

Configuration tests reject plaintext network origins, endpoint substitution, ambiguous paths, duplicate providers, and unbounded settings.
Login tests cover state binding, replay, PKCE, nonce, signature, issuer, audience, current activation, and approved endpoint enforcement.
Session tests cover current User/link/provider denial, expiry renewal, cookie flags, CSRF, concurrent tabs, and stream cancellation.
Host process tests must interrupt accepted authentication work and verify bounded draining before Runtime shutdown.
Both native stores must run the same browser-facing Resource operations. A real upstream provider must verify the complete code-exchange journey.

## Primary-source rationale

Use authorization code with PKCE S256 and exact redirect matching. These controls follow [RFC 9700](https://www.rfc-editor.org/rfc/rfc9700.html).
Bind callback state to the browser attempt, retain the nonce, and verify the original ID token through the sealed verifier.
These validation responsibilities follow [OpenID Connect Core](https://openid.net/specs/openid-connect-core-1_0.html).
ROM additionally checks current local Resource activation and bounds server-owned work. Those requirements come from this repository's authorization and resilience contracts.

## Internal modules

| Module | Responsibility |
| --- | --- |
| `configuration` | Host approval, canonical paths, endpoint bindings, and finite limits |
| `assets` | Bounded startup inventory and traversal-safe asset lookup |
| `oidc` | Pinned bounded HTTP acquisition and sealed verifier integration |
| `login` | Browser-bound attempts, PKCE, nonce, state consumption, and callback ownership |
| `session` | Bounded opaque sessions, CSRF secrets, finite retained token evidence, and cancellation |
| `csrf` | Exact Origin and unambiguous cookie/header parsing |
| `session_observation` | Response-stream cancellation and expiry/category handling |
| `router` | Protocol composition and generic HTTP mounting |
| `lifecycle` | Intake closure and accepted-work draining |

The token verifier exposes finite proof expiry, not original token expiry. After successful verification, the host reads original `exp` from that same token.
This parse does not establish trust. It only bounds retained evidence whose signature and complete claims have already passed verification.
The host retains token bindings when a verified ID token contains `at_hash` or `c_hash`. Reverification uses the same original bindings.

## Executed verification

The focused host suite passed 23 tests before final review. The final verification log is `evidence/rom-0.0.2/task4/verified-final-2.log`.
The real upstream provider journey ran on SQLite and redb. It verified code exchange, PKCE, signatures, current linking, and callback replay rejection.
The same journey verified 30-second evidence renewal, finite stream expiry, fresh query reopening, logout stream closure, and current User disablement.
It changed primary-provider selection through ordinary Resource commands. These server tests are not the later Playwright browser acceptance.

The logout stream test first timed out. The cancellation wrapper then made the same test pass.
A cancelled drain waiter also exposed abandoned ownership. The supervisor now retains unfinished jobs until completion.
Transient admission overload preserves the session. Confirmed denial removes it and cancels its observers.

The initial real-provider test froze wall time before provider startup. A second boundary could reject a legitimate new `iat`.
The corrected test clock uses real time plus a controlled offset. It tests expiry without using a stale initial timestamp.
A combined build temporarily failed on concurrent `rom-http` source. That source was stabilized by its owner before the host suite resumed.
Raw intermediate logs remain in the evidence directory. No intermediate failure is omitted or presented as a final product defect.

The optional host requires the application to install the ordinary `IdentityGate`. The host does not rewrite an existing Runtime registration.
Session storage is memory-only. Restart requires new login. The loopback provider fixture is not an externally reachable VPN identity provider.
Blob upload and actual-process shutdown with combined accepted authentication and blob work still require their dedicated integration checks.
