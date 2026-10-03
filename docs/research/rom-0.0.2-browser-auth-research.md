# ROM 0.0.2 browser authentication research

Date: 2026-10-03. Status: source review and proposed acceptance profile.

This report applies the repository writing rules and ASD-STE100 fallback guidance. It does not certify compliance.
No new authentication code, dependency resolver, provider journey, or browser test ran for this report.

## Existing implementation boundary

The selected [Studio design](../superpowers/specs/2026-10-03-rom-0.0.2-studio-design.md) requires a human OIDC authorization-code flow with PKCE.
The host provides opaque cookie sessions. The Rust Resource pipeline remains the authorization authority.

Source review found three reusable foundations:

- [rom-auth JWT verifier](../../crates/rom-auth/src/jwt.rs) provides pinned RS256 access-token verification through `jsonwebtoken` 11.1.0 and `aws_lc_rs`.
- [identity binding](../../crates/rom-identity/src/binding.rs) binds verified proof to current Provider, IdentityLink, and User revisions.
- [identity gate](../../crates/rom-identity/src/gate.rs) rejects changed revisions, disabled resources, and unstamped human actors.

The existing JWT profile is not an OIDC ID-token verifier. It requires `typ=at+jwt`, `jti`, `client_id`, and `principal_kind=human`.
It explicitly rejects ID-token types. An ordinary provider ID token cannot pass this profile without a false token rewrite.

`VerifiedIdentity` has no public constructor or deserializer. `ProviderProfile` currently accepts only JWT-human and introspection-service profiles.
OIDC integration therefore needs an explicit verifier/profile extension. Do not manufacture a request-controlled `Actor::trusted` as a shortcut.
Keep verified proof construction inside the authentication adapter. Keep current identity binding and gates in `rom-identity`.

The [existing provider fixture](../../demo/provider-fixture/provider.mjs) pins `oidc-provider` 9.12.2.
It supplies opaque client-credentials service tokens, no redirect URIs, and no authorization-code clients.
Its existing results are service-authentication evidence. They are not human-login evidence.

## Candidate dependencies

| Candidate | Verified source fact | Release implication |
| --- | --- | --- |
| `openidconnect` 4.0.1 | Published docs expose typed discovery, code exchange, PKCE, nonce, and ID-token verification. Tagged manifest declares Rust 1.65 and MIT. | Lowest protocol implementation burden, but its dependency graph has a concrete audit problem. |
| `oauth2` 5.0.0 | Tagged manifest declares Rust 1.65, MIT OR Apache-2.0, configurable HTTP, and optional reqwest 0.12. | Reuse typed code exchange and PKCE without its default HTTP dependency. It does not implement OIDC identity verification. |
| Existing `jsonwebtoken` 11.1.0 | ROM already uses its `aws_lc_rs` verifier with an explicit algorithm and pinned keys. | Reuse cryptographic verification for a separate narrow OIDC profile. Implement and test the OIDC claim contract explicitly. |
| Custom OAuth/OIDC protocol and crypto | No dependency saves all host lifecycle work. | Reject handwritten crypto and a wholly bespoke code-exchange implementation. |

Sources: [openidconnect API](https://docs.rs/openidconnect/4.0.1/openidconnect/), [4.0.1 manifest](https://raw.githubusercontent.com/ramosbugs/openidconnect-rs/4.0.1/Cargo.toml), [oauth2 5 manifest](https://raw.githubusercontent.com/ramosbugs/oauth2-rs/5.0.0/Cargo.toml), [current ROM manifest](../../crates/rom-auth/Cargo.toml).

`openidconnect` unconditionally depends on `rsa ^0.9.2`, plus elliptic-curve and Ed25519 implementations.
Disabling default HTTP features does not remove those cryptographic dependencies.
RustSec advisory RUSTSEC-2023-0071 reports no patched version. Its September 2026 update still identifies stable `rsa` 0.9.10 as affected.
The advisory concerns private-key timing leakage. A relying party limited to public-key verification has a different exposure.
That distinction does not remove the advisory from the resolved lockfile. [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2023-0071.html).

Recommend `oauth2` 5 with `default-features = false`, a bounded host HTTP adapter, and the existing `jsonwebtoken` backend.
Use a deliberately narrow RS256 OIDC ID-token profile. Keep its checks separate from the access-token verifier.
This costs more protocol validation code than `openidconnect`, but preserves the release's dependency gate without a silent exception.
Run a resolved dependency trial and audit before adoption. Declared Rust versions do not establish the full graph's minimum compiler version.

If a maintained OIDC verifier without the affected dependency passes the same gate, prefer its verified protocol implementation.
Do not claim that source inspection alone validates the proposed host verifier.

## Protocol checks and ownership

The following contract derives from OIDC Core, PKCE, and OAuth Security BCP.
[ID-token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation), [PKCE](https://www.rfc-editor.org/rfc/rfc7636.html), [OAuth BCP](https://www.rfc-editor.org/rfc/rfc9700.html).

| Boundary | Required behavior for the selected profile |
| --- | --- |
| Authorization start | Generate independent unpredictable state, nonce, and S256 verifier. Bind them to one browser transaction, approved provider revision, client, and exact callback. |
| Callback | Require exactly one code and state. Match the browser transaction, consume it once, and reject expiry or replay. Reject duplicate parameters. |
| Code exchange | Use the retained verifier and exact redirect URI. Authenticate the configured client. Do not reuse a code after an uncertain exchange. |
| ID token | Require the expected issuer, supported signature, client audience, expiry, issue time, and nonce. Check authorized-party rules for multiple audiences. |
| Key selection | Allow only configured provider keys and algorithm. Reject unsigned tokens, attacker key URLs, and incompatible key types. Bound key counts and bytes. |
| Token binding | Validate applicable token hashes if the profile consumes those returned tokens. Do not treat an ID token as an API bearer. |
| Identity | Map exact issuer/provider authority plus verified subject to an explicit IdentityLink. Email and display name do not link accounts. |
| Session | Create an opaque random ID after successful verification and current binding. Rotate any pre-login session identity. |

Use the library's secure random PKCE/state helpers. Use a constant-time secret comparison primitive for nonce or state secrets where needed.
Do not reconstruct these values from resource IDs, time, or user-controlled callback data.

For the first human profile, request only `openid` and the presentation claims actually used.
Do not add refresh tokens, UserInfo, dynamic registration, or arbitrary signature families without corresponding acceptance tests.
Short session lifetime is explicit. Refresh and provider logout semantics are separate future behavior, not implied OIDC guarantees.

## Approved endpoints and bounded acquisition

Provider Resources carry managed configuration, but editing one does not grant network acquisition authority.
The host independently approves issuer, authorization endpoint, token endpoint, JWKS endpoint, and callback origin.
Discovery metadata must match the exact issuer and approved endpoint rules before acquisition.

Disable HTTP redirects for token, discovery, and JWKS requests. The `openidconnect` documentation explicitly warns about SSRF through redirect following.
[HTTP client warning](https://docs.rs/openidconnect/4.0.1/openidconnect/#security-warning).
Approval must also address DNS changes, IP destinations, proxies, and private-network access.
A numeric loopback-only HTTP exception belongs to the fixture; deployed provider traffic uses approved HTTPS origins.

Use finite request timeouts, response-byte limits, pending-login limits, and authentication concurrency.
Bound decompressed response bytes, not only Content-Length. Reject oversized JSON before parsing.
Admission belongs to the host supervisor. A disconnected callback client must not release its permit while acquisition continues.
Avoid synchronous network calls on the async executor. If a blocking verifier runs, use a bounded blocking lane.

Do not retry a code exchange indefinitely after response loss. The provider may already have consumed the authorization code.
Fail that login transaction and offer a fresh login rather than pretending an unknown exchange succeeded.
This is distinct from a Resource mutation's durable receipt recovery.

## Cookies, origin, and CSRF

Use HttpOnly opaque sessions with Secure cookies on deployed HTTPS. Use an explicit loopback-only fixture exception.
Use SameSite=Lax for the selected top-level GET authorization callback. A cross-site POST callback needs separate cookie and CSRF acceptance.
Avoid Domain cookies. Restrict paths deliberately. A `__Host-` cookie requires Secure, no Domain, and Path=/.
[Browser cookie rules](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie).

For unsafe same-origin operations, require an exact approved Origin and a session-bound CSRF token.
SameSite is supplementary protection. CORS is not an authentication or CSRF mechanism.
Login start and logout need their own CSRF behavior. Reject arbitrary return URLs and keep callback URLs fixed.
[CSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).

Trust forwarding headers only from an explicitly configured proxy. Do not derive callback authority from untrusted Host headers.
Do not expose access tokens, client secrets, PKCE verifiers, or bootstrap authority to frontend source or localStorage.
The frontend receives current session presentation and a CSRF mechanism, not arbitrary actor claims.

A session establishes a verified identity; it does not freeze authorization.
Each request and live disclosure still checks current Provider, IdentityLink, User, and Resource policy.
On logout, expiry, or denial, close browser streams and clear projections.
Local logout invalidates the ROM session. It does not itself promise global provider logout.

## Real provider fixture

Extend the pinned `oidc-provider` 9.12.2 fixture with a separate authorization-code client.
Configure exact redirect URIs, `response_types: ['code']`, the selected client authentication method, and required S256 PKCE.
Retain the service client fixture and its evidence without token rewriting.

Use provider-generated authorization codes, ID tokens, discovery, and JWKS.
Implement deterministic fixture account authentication and consent through the provider interaction API.
The tagged upstream example uses `interactionDetails`, `interactionFinished`, and Grant consent records.
Do not copy its demo credentials or development interaction behavior into deployed ROM authentication.
[Tagged interaction example](https://raw.githubusercontent.com/panva/node-oidc-provider/v9.12.2/example/routes/koa.js), [tagged configuration example](https://raw.githubusercontent.com/panva/node-oidc-provider/v9.12.2/example/support/configuration.js).

A browser must follow the actual authorization redirect and complete the fixture login interaction.
Pre-minting a JWT and injecting it into a cookie is not equivalent evidence.
After callback, verify a real authorized mutation on SQLite and redb through the ordinary Resource pipeline.

## Acceptance matrix

| Case | Expected result |
| --- | --- |
| Correct code flow | Actual browser login produces an opaque session and authorized Resource operation. |
| Wrong state, nonce, client audience, issuer, or signature | No session and no Resource mutation. |
| Missing/wrong PKCE verifier or callback URI | Provider rejects exchange; host does not establish identity. |
| Callback replay and concurrent duplicate callback | At most one successful transaction; no duplicate session establishment. |
| Unknown/disabled User or IdentityLink | Denial without automatic email linking. |
| Provider changed during acquisition | Captured activation cannot bind against the new revision. |
| User/provider/link disabled after login | Next operation and live disclosure deny current authority. |
| Cross-origin unsafe request, missing CSRF, wrong Origin | No mutation; legitimate same-origin request still succeeds. |
| Endpoint redirect, unapproved destination, oversized response | Acquisition rejects before authority is established. |
| Slow provider and caller disconnect | Limits remain occupied until work ends; shutdown drains or terminates according to the host contract. |
| Exchange response lost | No fabricated successful login; old transaction cannot be replayed to bypass consumption. |
| Reopen/maintenance journey | Browser re-establishes identity explicitly; native state and pending obligations remain valid. |
| Provider outage | Existing local authorization remains bounded by the declared session profile; no silent fallback administrator. |

Record clean source, both lockfiles, provider version, browser engine, commands, and results.
Separate existing service-token tests, new human browser tests, negative verifier fixtures, and current-state authorization checks.
