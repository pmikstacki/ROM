# ROM 0.0.2 OIDC proof bridge results

Date: 2026-10-03. Status: implemented subset of Studio Task 4; generated-signature and native identity tests passed.

This report follows the repository writing rules and ASD-STE100 fallback guidance. It does not certify compliance.
The [Studio design](../superpowers/specs/2026-10-03-rom-0.0.2-studio-design.md) separates credential verification from browser sessions and current Resource authorization.
The [earlier research](rom-0.0.2-browser-auth-research.md) records the original verifier gap. This report records the implemented proof subset.

## Implemented contract

The optional `rom-auth/oidc` feature provides `OidcIdTokenAdapter` and depends on the existing `jwt` feature.
Signature verification uses the existing `jsonwebtoken` 11.1.0 `aws_lc_rs` backend.
SHA-256, base64url encoding and constant-time comparisons use the already resolved `sha2`, `base64` and `subtle` packages.
No new resolved package version or core dependency was added. The coordinator refreshed the workspace lockfile.

`VerifiedIdentity` now seals an `IdentityProfile` alongside authority, subject and exclusive expiry.
The profile fixes the principal kind. It has no public proof constructor, deserializer, raw credential field or Actor conversion.
The existing access-token and introspection adapters seal their respective profiles.

| Public API | Contract |
| --- | --- |
| `rom_auth::OidcIdTokenAdapter<K>` | `K: rom_auth::jwt::TrustedKeys`; source approval and issuer binding belong to the host. |
| `configured(authority: &str, issuer: &str, client_id: &str, source: K) -> Result<Self, AuthError>` | Configures exact names and host-selected keys. It makes no network request. |
| `with_trusted_audiences(self, additional: &[&str]) -> Result<Self, AuthError>` | Replaces the additional audience set. The configured client remains mandatory. |
| `authenticate(&mut self, id_token: &str, expected_nonce: &str, bindings: OidcTokenBindings<'_>, now: u64) -> Result<VerifiedIdentity, AuthError>` | Uses the independently retained nonce and trusted host Unix time. |
| `rom_auth::oidc::OidcTokenBindings<'a>` | Borrowed `access_token: Option<&'a str>` and `authorization_code: Option<&'a str>`; no credential retention or `Debug`. |
| `VerifiedIdentity::profile() -> IdentityProfile` | Distinguishes `JwtRs256Human`, `OAuthIntrospectionService` and `OidcRs256Human`. |
| `IdentityProfile::{as_str, principal_kind}` | Canonical profile identifier and its fixed principal kind. |
| `rom_identity::ProviderProfile::OidcRs256Human` | Ordinary Resource field value `oidc-rs256-human`. |

The implementation accepts signed RS256 authorization-code ID tokens.
It requires `iss`, `aud`, `sub`, `exp`, `iat` and `nonce` with their declared types.
It checks exact issuer, client audience, retained nonce, expiry, issued time and optional `nbf`.
Multiple audiences require matching `azp`. A present single-audience `azp` must also match the client.
Present optional claims cannot use null to disable a check.

Present `at_hash` and `c_hash` require their unchanged original inputs.
The verifier compares the base64url-encoded left half of SHA-256 with each hash claim.
Missing inputs, mismatches and malformed claims fail closed.
Hash absence is permitted for this code-flow profile.

These checks follow [OpenID Connect Core ID-token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation).
The [code-flow ID-token section](https://openid.net/specs/openid-connect-core-1_0.html#CodeIDToken) defines optional `at_hash`.
The [code validation section](https://openid.net/specs/openid-connect-core-1_0.html#CodeValidation) defines `c_hash` correspondence.
This implementation checks every present corresponding hash; it does not implement the hybrid flow.

## Fixed limits and trust boundary

| Item | Implemented limit |
| --- | --- |
| ID token | 16,384 bytes |
| Authority, issuer, client and audience names | Nonempty, at most 2,048 bytes, no surrounding whitespace |
| Trusted audience set | Client plus at most seven additional distinct names |
| Token audiences | One to eight distinct approved names; configured client must be present |
| Subject | One to 255 printable ASCII bytes, excluding spaces |
| Retained and token nonce | One to 2,048 printable ASCII bytes, excluding spaces |
| Original hash input | One to 16,384 printable ASCII bytes, excluding spaces |
| JOSE header | Object containing only `alg=RS256`, nonempty `kid` up to 64 bytes and optional `typ=JWT` |
| Issued token lifetime | At most 3,600 seconds; no clock leeway |
| Complete key set | One to eight entries; nonempty key IDs up to 64 bytes |
| Key refresh | At most once per five seconds; successful refresh replaces the complete set |
| Proof deadline | Minimum of token expiry, cached-key deadline and verification time plus 30 seconds |

The shared private key cache preserves the existing access-token refresh behavior.
The access-token adapter continues to reject ID-token types. The ID-token adapter rejects access-token types and symmetric algorithms.
Token headers cannot supply URLs, embedded keys, certificates, critical extensions or alternate payload rules.
Header admission occurs before key acquisition. A positional header array is rejected.

The host must approve the issuer and key source independently of editable Provider data.
It must bound key acquisition time and bytes and reject duplicate key IDs before constructing the trusted map.
It must use the exact captured activation configuration when constructing the verifier.
The adapter does not establish endpoint approval, PKCE, one-use state, code redemption or a browser session.
The host must consume each retained login attempt once. Calling `authenticate` alone does not consume it.

## Current identity bridge

`ProviderActivation::verify` requires the proof authority and exact sealed profile to match the captured Provider configuration.
`ActivatedIdentity::bind` rechecks the current Provider revision, exact profile, explicit IdentityLink and enabled User.
The Actor stamp records the profile and current Provider, link and User revisions.
`IdentityGate` checks that stamp against current records at ordinary Runtime authorization checkpoints.
An access-token proof cannot establish an OIDC-profile actor, even though both profiles describe humans.

Provider changes invalidate captured activation evidence and existing actors.
User and link changes invalidate existing actors. Disabled records deny new binding.
Fresh binding can use current enabled link and User records, provided the captured Provider revision is still current.
There is no automatic email linking, first-admin enrollment or request-controlled Actor constructor.

Existing Provider field encodings remain unchanged. The OIDC profile is an additional closed field value.
The private Actor stamp now requires the exact profile. Earlier stamps without that field fail closed; they are not silently upgraded.
This change does not modify persisted work metadata, native storage format or archive format.

## Executed evidence

The task started after the coordinator's successful native baseline at `91954b6`.
The scoped tests ran in `rom-dev`, Rust 1.99.0, with two build jobs and development/test debug information disabled.
They used `/var/tmp/rom-release-measured-verification-target`.
The final author checks observed HEAD `dad972f4aa39e93ac30c0ec87cd55cb0c90dd53d` plus this scoped patch and parallel owned changes.
The tested lockfile SHA-256 was `17ba0e390f683f9081945703b15d4e9f9d2504e988b1bdb776063ce098fdd201`.

| Command or scenario | Result and evidence |
| --- | --- |
| Initial missing feature, then missing public API | Expected RED, exit 101: [feature log](evidence/rom-0.0.2/oidc-proof/api-red.log), [API log](evidence/rom-0.0.2/oidc-proof/compiled-api-red.log). |
| ID-token API skeleton | Expected behavioral RED: five of six cases failed. [Log](evidence/rom-0.0.2/oidc-proof/behavior-red.log). |
| Managed OIDC profile and cross-profile binding | Missing variant RED, then cross-profile behavioral RED. [API log](evidence/rom-0.0.2/oidc-proof/identity-api-red.log), [behavior log](evidence/rom-0.0.2/oidc-proof/identity-behavior-red.log). |
| Header object admission | Expected RED: array caused one key fetch instead of zero. [Log](evidence/rom-0.0.2/oidc-proof/header-object-red.log). |
| `./crates/rom-auth/verify` | Exit 0: 11 OIDC cases, all 19 existing profile cases, three public examples and two compile-fail doctests. Strict Clippy, formatting, documentation, independent feature builds and core dependency isolation passed. [Final log](evidence/rom-0.0.2/oidc-proof/auth-verify-final.log). |
| `./crates/rom-identity/verify` | Exit 0: eight integration cases and two doctests. Strict Clippy, formatting, documentation and normal dependency isolation passed. [Log](evidence/rom-0.0.2/oidc-proof/identity-verify.log). |

The OIDC cases cover real RSA signatures, wrong signatures, issuer/audience/nonce/time errors and missing claims.
They also cover header trust, claim/configuration bounds, hash correspondence, key rotation, expired cache and failed acquisition.
The identity cases use SQLite and ordinary typed Resource commands with explicit host bootstrap authority.
They cover profile mismatch, finite expiry, absent links, stamp mismatch, current Provider/User/link changes and stale activation.
The existing access-token, introspection, receipt/live and action-race tests passed unchanged in behavior.

The [source hashes](evidence/rom-0.0.2/oidc-proof/source.sha256) identify the two complete crate source selections.
Compiler and command provenance is in [environment.txt](evidence/rom-0.0.2/oidc-proof/environment.txt).
Private signing keys and credentials stayed in memory; logs contain no tokens, codes or nonce values.
Two authoring issues were corrected before final checks: a `json!` array expression and fixture-extraction imports.
Neither was treated as a feature acceptance RED.

The coordinator owns the full local verifier and integration gate after source freeze.
This subset does not establish actual provider interoperability, browser login, PKCE redemption, session/CSRF behavior or host lifecycle acceptance.
Those journeys require the separate maintained host and actual provider fixture.
