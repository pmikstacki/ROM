# Probe the local OAuth service profile

Date: 2026-10-03.
Status: isolated compatibility experiment passed. Product integration and release task 4.3 remain unfinished.

**oidc-provider 9.12.2 issued a real access token that ROM's unchanged introspection verifier accepted.**
The test used the provider's client credentials grant and authenticated introspection endpoint.
No middleware rewrote the response. No verifier condition changed.

This result supports one disposable service profile. It does not establish production deployment support.
The [selection note](real-provider-profile-selection.md) describes source-based feasibility and the Keycloak alternative.
The [release task](../../openspec/changes/prepare-framework-release/tasks.md) and [release design](../../openspec/changes/prepare-framework-release/design.md) retain the wider integration boundary.

## Candidate history and dependency review

The initial source study selected version 9.5.1 before a package install or dependency audit.
Its isolated install resolved `@koa/router` 14.0.0.
The npm audit reported the moderate advisory `GHSA-47p6-69vm-vw6v` and the affected direct provider dependency.
The advisory describes an access-control bypass in the router. That pin was rejected. [Official advisory](https://github.com/advisories/GHSA-47p6-69vm-vw6v).

No successful interoperability result is claimed for version 9.5.1.
Its startup attempt reached token issuance and failed on fixture client metadata.
The later diagnosis confirmed a missing provider scope declaration in the same configuration on version 9.12.2.
The rejected candidate's install log, lockfile, audit response and failed probe logs remain separate evidence.

Version 9.12.2 is an exact replacement pin. Its official package declares the MIT license.
Its production dependencies exclude the old router.
The pinned source still provides actual client credentials issuance, stored extra claims and introspection. [Release](https://github.com/panva/node-oidc-provider/releases/tag/v9.12.2), [package metadata](https://github.com/panva/node-oidc-provider/blob/v9.12.2/package.json), [grant](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/actions/grants/client_credentials.js), [opaque formatter](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/models/formats/opaque.js), [introspection](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/actions/introspection.js).

The final npm audit returned exit 0 with zero known vulnerabilities for the exact lockfile.
The installed inventory has 40 packages: 38 declare MIT and two declare ISC.
The inventory records each package's name, version, license declaration, resolved URL and integrity value.
This is a package metadata review and a dated registry audit. It is not a complete security or legal certification.

| Locked input | Value |
| --- | --- |
| Provider | `oidc-provider` exactly `9.12.2` |
| Tarball | `https://registry.npmjs.org/oidc-provider/-/oidc-provider-9.12.2.tgz` |
| npm integrity | `sha512-UaqeVpeijTocxVbDowPqPKloGjb49bxRSzHfChyI86teR+6xxoiI1V5jv8CHL2AIUmA2rg0xTZcYaC9GguympA==` |
| npm lockfile SHA-256 | `1ace0c2b909ae7ad2b27f830eb57501a907f12bc1a4daf2d9bb01e41c38dc87c` |
| Standalone Cargo lockfile SHA-256 | `42c94c1d69e680df589900c783eec7282389a41744694d47e4141de3aee41e75` |
| Runtime | Node `v22.16.0`; npm `10.9.2` |
| Rust compiler | `rustc 1.99.0 (b940084d7 2026-09-28)` |
| Cargo | `cargo 1.99.0 (5f94df478 2026-08-27)` |

## Executed fixture

The fixture binds to `127.0.0.1` on a newly allocated port.
Its exact issuer is `http://127.0.0.1:<allocated-port>`.
It creates two separate random secrets in private temporary files and removes them after the probe.
It creates a fresh RSA private JWK in memory.
The probe records neither secrets, tokens, private keys nor raw credential responses.

The issuing client is `rom-service`. Its grant is `client_credentials`, its scope is `rom`, and its authentication is `client_secret_basic`.
The separate `rom-introspector` client has no issuance grants.
The introspection policy permits that client to inspect tokens issued only to `rom-service`.
Both clients have empty response types and redirect URI lists.
Browser development interactions are disabled.

The resource indicator is `https://rom.fixture.invalid/api`. This value identifies the local API; no network request uses that hostname.
Its audience is `rom-api`, its token format is opaque, and its token lifetime is 60 seconds.
The fixture explicitly sets provider `scopes: ['rom']` and `ttl: { ClientCredentials: 60 }`.
The issuance hook sets `sub` from the provider-authenticated token's `clientId` and sets `principal_kind` to `service`.
It rejects token kinds or issuing clients outside this profile.

The first version of the proposed configuration omitted provider `scopes: ['rom']`.
The actual provider rejected the client's scope as `invalid_client_metadata`.
The pinned metadata validator confirms that client scopes must exist in the provider's scope set.
Adding the explicit scope declaration resolved that failure. No token response changed after issuance. [Pinned client schema](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/helpers/client_schema.js).

The corrected provider returned its original introspection response with the following properties:

| Property | Observed result |
| --- | --- |
| `active` | `true` |
| `iss` | Exact allocated loopback issuer |
| `aud` | `rom-api` |
| `sub` and `client_id` | Both `rom-service` |
| `principal_kind` | `service` |
| `token_type` | `Bearer` |
| `exp` | After current host time and within the 60-second fixture lifetime |
| `iat` | At or before current host time |
| `nbf` | Absent, or no future value |
| `cnf` | Absent |

A standalone Rust consumer links to the existing `rom-auth` source with only its `introspection` feature.
It receives the secret and token through a pipe, not command arguments.
`IntrospectionAdapter` makes its own authenticated HTTP POST directly to the actual provider endpoint.
It parses the provider's original response. The Node probe supplies no replacement response body.
ROM's public `VerifiedIdentity` reports authority `local-oidc`, subject `rom-service`, service kind and a proof deadline within five seconds.

## Results and isolation

| Scenario | Result |
| --- | --- |
| Real grant, original introspection and unchanged ROM adapter | Accepted |
| Expected audience `other-api` with the same actual token | `AuthError::Binding` |
| Never-issued token through the actual provider | `AuthError::Inactive` |
| Trusted host time 61 seconds after acquisition | `AuthError::Expired` |
| Reinstall from the exact npm lockfile, then repeat the probe | Passed |
| Client-secret and bearer-token absence from consumer output | Passed |
| Provider and consumer process termination | Passed; owned processes joined |
| Temporary secret file removal | Passed |

Probe HTTP requests have three-second deadlines. ROM retains its 250-millisecond connect and 500-millisecond request limits.
The consumer has a five-second completion deadline.
The fixture stops and joins only its own processes. It does not stop existing ROM services.
The Cargo target is `/var/tmp/rom-provider-profile-probe-target`, separate from the active verification target.
Product manifests, product lockfiles and `rom-auth` source are unchanged.

The provider warns that its default memory adapter loses tokens on process restart.
That warning is retained in the successful log. The fixture deliberately uses disposable state.
ROM host reopen tests can retain this provider process. Provider restart must be a separate token-unavailability case.

## Retained local evidence and reproduction

Scratch assets and evidence remain under the private directory:

```text
.superpowers/sdd/2026-10-03-operator-recovery/provider-probe/
  throwaway-provider-9.5.1/       rejected candidate and lockfile
  throwaway-provider-9.12.2/     probe.mjs, package.json, package-lock.json
  throwaway-consumer/           standalone Cargo consumer and lockfile
  evidence/                    sanitized commands, hashes, logs and inventory
```

These assets are experimental. They are not a maintained package or a published fixture.
The following commands ran inside `rom-dev`, from the indicated scratch subdirectories.
They used a private npm cache inside the probe directory.

```sh
# throwaway-provider-9.12.2
npm install --ignore-scripts --no-audit --no-fund --cache ../throwaway-npm-cache
npm audit --json --cache ../throwaway-npm-cache
npm ci --ignore-scripts --no-audit --no-fund --cache ../throwaway-npm-cache

# throwaway-consumer
CARGO_TARGET_DIR=/var/tmp/rom-provider-profile-probe-target \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_BUILD_JOBS=2 cargo build --locked

# throwaway-provider-9.12.2
node probe.mjs
```

The final command passed after `npm ci` and a locked consumer build.
The standalone consumer also passed `cargo clippy --locked -- -D warnings` and `rustfmt --check`.
Both JavaScript files passed Node syntax checks. Both reports passed local-link checks without private publication links.
`provider-profile-9.12.2-frozen-green.log` records the final sanitized result.
`npm-audit-9.12.2.json` and `npm-inventory-9.12.2.json` record the dependency result.
`source.txt` records source and lockfile hashes.
Failed logs retain static phases and recognized error categories. They contain no raw error descriptions or credentials.

ROM source baseline is `62285bd40b7890000f89ee8fed2cba371053ada8` with concurrent operator changes.
The introspection source SHA-256 remains `71e9b3b095cb8a9e331c44d1e8598848e3ba16ff0b6f50055435cb6f57a3d8b2`.
The product Cargo lockfile remains `13659b8f2f4114437bb754d7b6f1a69ae25fb53e163ae52878bd6f2bcf664cf7`.

## Remaining work

This experiment proves interoperability for this exact local configuration and locked provider graph.
It does not implement host authentication middleware, IdentityGate composition, bootstrap, secret rotation or CLI acceptance on either native store.
It does not test TLS, provider persistence, real secret stores, production accounts, human login or operator policy.
Those items remain separate stage 4.3 work. No release task was marked complete.

Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary.
Full compliance requires verification against the official standard. This document does not certify compliance.
