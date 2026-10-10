# Production identity admission research

Date: 2026-10-07. Status: source investigation and proposed experiments. No provider experiment was executed for this report.

Select an **isolated Authentik 2026.8.3 deployment** for R7. This matches the consumer named in [the research plan](rom-0.1.0-consumer-research-plan.md:62).
Dex is a smaller generic provider, but its results cannot substitute for this consumer requirement.
Use fresh accounts, signing keys, credentials, database volumes, and endpoints. Do not contact or modify Astral Plane's deployment.

The [implementation plan](../superpowers/plans/2026-10-07-production-identity-admission.md) requires coordinator review before implementation.
Neither document changes the support matrix or closes R7.

## Source investigation

The examined ROM checkout has HEAD `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470` with ongoing candidate changes.
HEAD alone does not identify those changes. The source hashes below identify the main investigated seams.
Recompute a complete source manifest before execution, including the extracted candidate, fixture, locks, and all host authentication modules.

| Source | SHA-256 |
| --- | --- |
| `Cargo.lock` | `843ad72ac3badece40e5c0f01e60f42374bba5dbc61f67b44aad16917827e3a2` |
| `crates/rom-auth/src/oidc/adapter.rs` | `ca899fb87fb65b32f29249d8306ccdf76663fafc6ca67d5495177e9c4648cfea` |
| `crates/rom-auth/src/keys.rs` | `17ee1b006f7995ac30cd99ef559e410daf6b75ee824bd6b9835b36711cb94742` |
| `crates/rom-identity/src/binding.rs` | `428381b654c10ebd688d9d456b457c81f0271b801eb818678b2afbca566d4cce` |
| `crates/rom-identity/src/gate.rs` | `3a68e60b1fb4137eaf68dc04681f211a92dd4dcef062fee58d61f8c1db4aacda` |
| `crates/rom-studio-host/src/configuration.rs` | `83d23b2392dd859c5d184e35a1547fe6b54ad30097d44b13fccb50a1074eddbd` |
| `crates/rom-studio-host/src/authentication.rs` | `b95050efc055ce54ba67a57c90ea2ca302fcf72d887af6a206e427f020603561` |
| `crates/rom-studio-host/src/oidc.rs` | `af4ad66f6395af725bc86ffd1b2e5673fdf6bb2b54e642138937f43540761d0b` |
| `crates/rom-studio-host/src/session.rs` | `11503afded810e92dcb6c1b8ccd4fd604fc31609f9d53692b55e09ef9225486f` |
| `crates/rom-studio-host/src/router.rs` | `445e7335be230a89ffb3f41b14277b57dfc26e2c6c73925741dc3c62117becf5` |
| `demo/provider-fixture/human-server.mjs` | `04ee91663652cc0f41af0652b166bef3f5253f5a41e61b942f9548354fcd7032` |

| Boundary | Existing behavior and exact seam |
| --- | --- |
| Token verification | [OidcIdTokenAdapter](../../crates/rom-auth/src/oidc/adapter.rs:20) accepts RS256 authorization-code ID tokens. It checks exact issuer, client audience, nonce, trusted time, and supplied token hash bindings. Token lifetime is at most one hour; proof/key validity is at most 30 seconds. |
| Identity binding | [ProviderActivation](../../crates/rom-identity/src/binding.rs:66) captures the current provider revision. Its public `verify` and `ActivatedIdentity::bind` methods retain the configured trust namespace. [IdentityGate](../../crates/rom-identity/src/gate.rs:26) checks current provider, link, and User state. Email is not an automatic linking credential. |
| HTTP authentication | [AsyncAuthResolver](../../crates/rom-http/src/authentication.rs) lets the host own asynchronous acquisition. It does not acquire a provider or establish deployment policy. |
| Approved transport | [HostConfig](../../crates/rom-studio-host/src/configuration.rs:68) approves exact provider endpoints, origin, callback base path, and finite limits. HTTPS is the default. Loopback HTTP and exact internal backchannels require explicit host configuration. |
| Browser login | [authentication.rs](../../crates/rom-studio-host/src/authentication.rs:83) retains browser-bound state, nonce, and an S256 PKCE verifier. Callback consumption precedes exchange. Exchange uses the exact callback and configured confidential client. |
| Session authority | [oidc.rs](../../crates/rom-studio-host/src/oidc.rs:174) re-verifies the original ID token after proof expiry, then binds current Resource identity. [SessionStore](../../crates/rom-studio-host/src/session.rs:72) caps session lifetime by the original token expiry. Sessions are in memory; host restart requires fresh login. |
| CSRF and local logout | [csrf.rs](../../crates/rom-studio-host/src/csrf.rs:4) requires one exact Origin and one matching CSRF token. [logout](../../crates/rom-studio-host/src/router.rs:236) removes the local session and clears its cookie. It does not invoke an OP end-session endpoint. |
| Observation | [session_observation.rs](../../crates/rom-studio-host/src/session_observation.rs:21) checks cancellation/current authority before disclosure. A captured Actor is never extended in place. |
| Existing real provider fixture | [human-server.mjs](../../demo/provider-fixture/human-server.mjs:30) uses actual `oidc-provider` 9.12.2 protocol handling. It generates new keys and cookie secrets at each start, uses memory storage, and supplies fixture accounts. This does not prove persistent provider restart or production key rotation. |

The [support document](../release-support.md:108) already distinguishes human OIDC implementation from production TLS evidence.
The [service deployment report](provider-deployment-results.md) concerns a different introspection profile. Its synthetic outage tests do not prove human session recovery.

## Findings that need failing experiments

**F1: Authentik JWKS metadata is incompatible with the current closed decoder.**
ROM's [Jwk](../../crates/rom-studio-host/src/oidc.rs:12) rejects unknown fields.
Authentik's pinned RSA JWKS generator emits `x5c`, `x5t`, and `x5t#S256`, in addition to the core RSA parameters.
Its view serves that key without removing those fields. This predicts rejection of the unchanged provider response.
This is a source inference, not an observed failure. Preserve the actual response and reproduce login before changing ROM.
Do not make a fixture response filter conceal the incompatibility. [Pinned JWKS source](https://github.com/goauthentik/authentik/blob/version/2026.8.3/authentik/providers/oauth2/views/jwks.py#L62).

**F2: Acquisition failure becomes permanent session loss.**
[bounded](../../crates/rom-studio-host/src/oidc.rs:264) maps network and response-stream failures to `Error::Denied`.
[resolve](../../crates/rom-studio-host/src/authentication.rs:19) passes that result to [SessionStore::failed](../../crates/rom-studio-host/src/session.rs:109), which removes denied sessions.
A provider outage after the 30-second proof interval therefore predicts session removal before original token expiry.
AP-UX-006 requires transient failure handling that preserves context. The proposed correction must deny protected operations during unavailable verification.
It must not extend expired proofs, reinterpret invalid signatures as transient, or hide current revocation.
The public `AuthError::Unavailable` already exists; any public error-contract change requires separate review.

**F3: Key rotation can be a cutover rather than an overlap.**
Authentik's pinned JWKS view publishes the currently selected signing key and optional encryption key.
It does not enumerate historical signing keys. Switching the selected key can remove the previous key while earlier tokens remain unexpired.
Measure this actual behavior. Do not import Dex's overlap guarantee into Authentik acceptance.
A documented cutover may require reauthentication after ROM's existing proof interval.
An uninterrupted overlap requirement would need a separately reviewed provider procedure. [Pinned JWKS view](https://github.com/goauthentik/authentik/blob/version/2026.8.3/authentik/providers/oauth2/views/jwks.py#L123).

**F4: Local logout is not provider-wide logout.**
The current host has no end-session configuration, `sid` binding, logout-token verifier, or back-channel logout route.
Require actual local logout and old-cookie rejection for the supported contract.
Run OP logout separately and record its effect on fresh login and existing ROM cookies.
Do not claim immediate OP revocation of a signed ID token without a tested notification or introspection contract.
Provider-wide logout is a separate feature if the release requires it. [RP-Initiated Logout standard](https://openid.net/specs/openid-connect-rpinitiated-1_0.html), [Authentik provider/logout endpoints](https://docs.goauthentik.io/add-secure-apps/providers/oauth2/).

**F5: Fresh Authentik key identifiers exceed the current bound.**
Authentik generates an unpadded base64url SHA-512 key identifier. Its 64-byte digest produces an 86-character identifier.
ROM limits identifiers to 64 bytes in the shared cache, JWT header, OIDC header, and host JWKS decoder.
The actual fresh-key fixture must reproduce this separately from certificate metadata rejection.
Do not configure a legacy short identifier to conceal the mismatch. [Pinned key generation](https://github.com/goauthentik/authentik/blob/version/2026.8.3/authentik/crypto/models.py#L80).

The bounded consumer vendor comparison found a `MAX_KEY_ID_BYTES = 256` constant and inert certificate metadata fields already present.
Those are consumer patches, not admitted upstream implementation or test evidence.
`vendor/rom/crates/rom-studio-host/src/oidc.rs` hashes to `58300fff1a368a605401f1d3f12d68301fd380a6dff93d5e3969b0a6e2ce025e`.
`vendor/rom/crates/rom-auth/src/jwt.rs` hashes to `56a40b5756c516cf96aa045f6fa8d9ff8ea420f8b3c6feee4151097cef4646bc`.
Evaluate one shared finite limit with real long identifiers and explicit overflow cases. Preserve both consumer and historical sources.

## Provider comparison and selection

| Candidate | Cost and guarantees | Decision |
| --- | --- | --- |
| Existing `oidc-provider` 9.12.2 | Node runtime and locked package graph already exist. MIT. Real protocol engine. Persistent adapter, account administration, persisted key ring, and cookie secrets would become fixture-owned implementations. | Retain existing tests. Extending this alone would not reproduce the named consumer deployment. |
| Dex 2.45.1 | Apache-2.0. One process, SQLite, local password connector, and native persisted signer rotation. Declares Go 1.25.0. RSA signer retains previous public keys through token validity. Go, Dex, Docker, and Podman were absent in the bounded `rom-dev` prerequisite inspection. | Lowest-cost standalone generic persistent IdP among these alternatives. Optional diagnostic lane; unnecessary for closing the Authentik-specific R7 requirement. |
| Authentik 2026.8.3 | Known consumer configuration and pinned images. Server, worker, PostgreSQL 16, and isolated TLS transport. Higher process cost, but no custom provider storage or rotation engine. | Select the smallest isolated deployment that satisfies the actual R7 consumer requirement. |

The existing Node pin is not a claim about the latest release. Its upstream v9 line is maintained and requires a supported Node LTS runtime.
[Pinned README](https://github.com/panva/node-oidc-provider/blob/v9.12.2/README.md), [runtime gate](https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/index.js), [license](https://github.com/panva/node-oidc-provider/blob/v9.12.2/LICENSE.md).

Dex's selected observed release is 2.45.1; acquisition must resolve its exact commit and source hash before execution.
Its local signer checks rotation every 30 seconds, so a short configured interval does not guarantee immediate rotation.
The relevant dependencies include `go-sqlite3` 1.14.34, `go-jose/v4` 4.1.3, and `go-oidc/v3` 3.17.0.
The upstream image build enables CGO. A source build would need a C compiler and an independently bounded Go cache.
[Release](https://github.com/dexidp/dex/releases/tag/v2.45.1), [module graph](https://github.com/dexidp/dex/blob/v2.45.1/go.mod), [license](https://github.com/dexidp/dex/blob/v2.45.1/LICENSE), [signer](https://github.com/dexidp/dex/blob/v2.45.1/server/signer/local.go), [rotation](https://github.com/dexidp/dex/blob/v2.45.1/server/signer/rotation.go), [image build](https://github.com/dexidp/dex/blob/v2.45.1/Dockerfile).

Authentik's pinned source declares Python 3.14 and version 2026.8.3. Use admitted images rather than install that Python graph into ROM.
Its community source is MIT with explicit documentation, enterprise, and third-party exceptions. Inventory selected image dependencies and notices separately.
Neither provider changes ROM's declared Rust 1.99 requirement or its MIT license.
[Pinned package metadata](https://github.com/goauthentik/authentik/blob/version/2026.8.3/pyproject.toml), [license terms](https://github.com/goauthentik/authentik/blob/version/2026.8.3/LICENSE).

Official Authentik Compose guidance specifies at least two CPU cores and 2 GB RAM.
Those are upstream minimums, not measured fixture usage. The experiment budgets appear in the implementation plan.
Compose v2 and an isolated container engine remain prerequisites; this investigation did not install either.
[Official installation guide](https://docs.goauthentik.io/install-config/install/docker-compose/).

## Bounded consumer comparison

Read-only inspection covered `/root/astral-plane/docs/authentik.md`, `deployment/authentik.compose.yml`, `deployment/authentik.images.env`, `src/host.rs:115`, and `src/authorization.rs:8`.
No private credential file or remote endpoint was read. The image environment file contains image identities only.
The existing deployment uses Authentik 2026.8.3, PostgreSQL 16, a worker, and external Caddy TLS.
Its host uses ROM's public `HostConfig` and `StudioHost`. Its ownership policy derives application ownership from `linked_user_id`.

Reproduce only the generic integration settings: exact per-provider issuer including its final slash, explicit endpoints, RS256, stable `user_id` subject, and strict callback.
Use fresh fixture users and a different authority. Do not copy application subject mappings, volumes, networks, branding, social sources, or secrets.
Record both provider-generated subject and ROM User identity. Account email changes must not attach another account's Resource.

The locally declared image candidates are:

- Authentik: `ghcr.io/goauthentik/server@sha256:ab9b4e8cc4ab3f8d1198d2db6aeea66bafea1963b3f2843589e0d163f97d9849`.
- PostgreSQL: `postgres@sha256:721873c34ceb9f8d8fc265984940dc982404c105f19ad51be9fdc5970a6080ea`.

These are consumer configuration observations, not independently admitted downloaded images.
Resolve platform manifests, signatures/provenance where supplied, source revisions, and advisory results before executing them.

## Admission boundary

Use an external Rust consumer against verified extracted ROM source, its own frozen Cargo lock, and only public APIs.
Run real browser redirects, actual authorization-code exchange, unchanged ID tokens, and unchanged JWKS through the host.
Also exercise `OidcIdTokenAdapter::authenticate`, `ProviderActivation::verify`, and `ActivatedIdentity::bind` with actual provider-issued material.
Use a generic owned Note Resource, not Astral Plane's application domain.
Run both SQLite and redb without changing source between adapter runs.

The failure matrix must include invalid signature, wrong issuer/audience/nonce, PKCE failure, callback replay, browser mismatch, expiry, denied account consent, current grant revocation, and CSRF.
Use actual provider settings or independent real issuance for token cases. Browser interception may alter a request, but must not generate replacement success responses.
The OAuth security review must retain exact redirect validation, PKCE, state binding, issuer binding, and confidential token exchange.
[OAuth Security BCP](https://www.rfc-editor.org/rfc/rfc9700.html), [OIDC Core validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation).

Require an isolated HTTPS lane before describing TLS admission. Test invalid certificate and wrong-host failures without disabling verification.
A loopback HTTP preflight is useful but has a separate result category.
Do not place private keys, passwords, authorization codes, token bodies, cookie values, or administrative bearer values in public evidence.
Retain public JWKS, certificate fingerprints, safe request categories, exact revisions, result counts, and resource observations.

R7 remains open until the isolated Authentik matrix, required deployment transport, independent review, and source fingerprint checks complete.
AP-UX-006 concerns session recovery. AP-UX-021 concerns admin and guest boundaries. This report does not close either consumer workflow.
R1–R6 and R8–R14 retain their separate acceptance obligations.

## Isolated prerequisite results on 2026-10-07

The coordinator allocated `/var/tmp/rom-010-authentik-20261007` and loopback ports 44389 through 44393.
These results belong to the source authoring lane. They do not admit a release artifact or establish a provider journey.

Actual host Podman 5.2.3 information queries passed against two explicit isolated vfs stores, with zero images and containers.
The launcher recorded actual Linux process group, session, start time, and boot identity before approving each command.
A separate `unshare` command established mount, network, and PID namespaces and exited successfully.
Neither result proves container networking, TLS, or provider readiness.

Actual public registry metadata verified these Linux/amd64 image manifests:

| Image | Requested index digest | Selected platform digest | Compressed layer bytes |
| --- | --- | --- | --- |
| Authentik | `sha256:ab9b4e8cc4ab3f8d1198d2db6aeea66bafea1963b3f2843589e0d163f97d9849` | `sha256:09782fe56675bc616a0324468f1698e2d9d83c978bc5e426686fb7563517a442` | 382343402 |
| PostgreSQL | `sha256:721873c34ceb9f8d8fc265984940dc982404c105f19ad51be9fdc5970a6080ea` | `sha256:1a66d744c1b459e13b05a8fca341da84cb63383e99ce262210efee5a319d4551` | 116062392 |

Authentik's image label reports version `2026.8.3`, but its revision label is empty. PostgreSQL has no selected OCI source labels.
The official Authentik tag independently resolves to source commit `e5a0d2f7572cb776eee7a3e9355937ce38973761`.
This does not bind that source commit to the downloaded image. Image source attestation and package vulnerability analysis remain unproved.
[Official source commit](https://github.com/goauthentik/authentik/commit/e5a0d2f7572cb776eee7a3e9355937ce38973761).

The official Authentik advisory API returned 47 public records. The latest records affect versions through 2026.8.1 and specify patches in 2026.8.2.
One structured record has an empty patch field; its description specifies fixes in 2025.12.5 and 2026.2.3.
This review does not establish that the image or its dependency graph has no vulnerabilities.
[Official advisories](https://github.com/goauthentik/authentik/security/advisories), [record with empty structured patch field](https://github.com/goauthentik/authentik/security/advisories/GHSA-4v4x-x5pr-8gp2).
The pinned Authentik source license and the PostgreSQL license page were retained with the metadata.
[PostgreSQL license](https://www.postgresql.org/about/licence/).

The acquisition filesystem is a new fixed-size 12 GiB ext4 backing file with a separately owned loop device and mountpoint.
Its block size is 4096 bytes. Initial available capacity was 12554715136 bytes; reserved bytes were 16777216.
Image store, runroot, temporary files, and downloads are inside that filesystem. The earlier empty-store evidence remains separate.

The first pull rejected an unsupported `--policy` flag before acquisition. The corrected digest-pinned pull reached the storage guard.
Vfs layer expansion used 11517329408 bytes. Available space was 1039507456 bytes when the stopped process finished draining.
The configured stop threshold was 1 GiB; the independent fixed filesystem enforced the hard limit throughout.
The owned process terminated with SIGTERM after 101142 milliseconds. PostgreSQL acquisition and container launch did not start.
All failed attempts, partial layers, mount, and backing file remain retained. A different storage configuration requires coordinator allocation.

Evidence resides under `/var/tmp/rom-010-authentik-20261007`: `namespace-prerequisite.json`, `bounded-filesystem-result.json`, `bounded-engine-info.json`, `metadata-redirect-admission/`, and `image-acquisition-retry-result.json`.
The final focused fixture suite passed 30 Node tests, with no failures or skips.
Its log is `/var/tmp/rom-010-identity-bounded-engine-green-20261007.log`.
Those tests and prerequisites do not close R7 or AP-UX-004, AP-UX-005, AP-UX-006, or AP-UX-015.

### Separate overlay acquisition checkpoint

The coordinator subsequently admitted a second 12 GiB store and a 32 GiB total retained experiment envelope.
The total comprises the original 12 GiB store, the new 12 GiB store, and 8 GiB for run evidence.
The original vfs resource RED, partial layers, backing file, and mount remain retained.

An actual private mount-namespace overlay probe passed read and write checks with new owned lower, upper, and work directories.
A new ext4 backing filesystem was attached to separately recorded `/dev/loop1` and mounted under the experiment's `overlay/acquisition` directory.
A fresh information query identified the expected isolated overlay store with zero images and containers.
No default store, consumer deployment, or existing project network was used.

Both Linux/amd64 image pulls succeeded and drained cleanly. Total acquisition time was 34885 milliseconds.
Actual stored manifest and configuration identities match the independently verified registry metadata above.
Authentik's image size is 1374065647 bytes. The PostgreSQL image reports version 16.15 and size 297130821 bytes.
The filesystem used 1884880896 bytes, with 10671955968 bytes available after both pulls.
The sparse backing file physically occupied 1986764800 bytes. Backing and mounted filesystem observations count the same stored data.
Do not sum them as independent physical allocations.

Evidence is in `overlay/`: `bounded-filesystem-result.json`, `bounded-engine-info.json`, `image-acquisition-retry-result.json`, and `stored-images-result.json`.
The additional driver-selection RED and GREEN are `/var/tmp/rom-010-identity-overlay-engine-red-20261007.log` and `/var/tmp/rom-010-identity-overlay-engine-green-20261007.log`.
The latest focused fixture suite passed 31 tests, with zero failures and skips.
Image source attestation and package vulnerability scanning remain unproved. No provider container or browser journey has started.

### Original-provider SDK compatibility checkpoint

The isolated Authentik/PostgreSQL stack reached readiness under recorded process, namespace, cgroup, and filesystem limits.
The provider and pasta relay share four CPUs, 4 GiB memory, and 512 processes.
Each browser attempt uses a separate owned 4 GiB cgroup. The run filesystem has a hard 8 GiB capacity.
Stopped containers, volumes, namespace mounts, failed attempts, and acquired images remain retained.

Original discovery and JWKS responses came from the configured synthetic Authentik provider.
Its issued ID-token header contains `alg`, `kid`, and `typ`. The 86-byte key ID exactly matches the original JWKS.
The JWKS also contains `x5c`, `x5t`, and `x5t#S256`. Those fields have not yet passed the StudioHost parser.
Image-to-source attestation and package vulnerability scanning remain unproved.

Early failures belonged to provisioning and browser automation. The provider initially had no permitted grant types.
Its supported API admitted `authorization_code` and the stock authentication flow.
A browser selector then matched the identification stage's password-manager input before the password stage appeared.
The retained request projection showed a second identification submission, without a password field.
Waiting for `ak-stage-password` and selecting its input produced the original code and PKCE callback.
These failures are not ROM compatibility REDs.

The public `rom-auth` OIDC adapter then rejected the untouched original token with `UnknownKey`.
The actual binary invocation exited with code 1 and drained. Its stderr contained zero bytes.
Both its OIDC header and shared key cache imposed an independent 64-byte key-ID limit.
This is an observed SDK compatibility RED. StudioHost certificate-metadata rejection remains a source prediction.

The corrective ROM profile accepts nonempty key IDs through 256 UTF-8 bytes and rejects Unicode control characters.
It preserves the complete opaque string without truncation, case folding, or Unicode normalization.
The shared contract governs JWT headers, OIDC headers, and complete refreshed key sets.
The eight-key limit, five-second refresh floor, signature checks, issuer binding, audience binding, and token deadlines remain unchanged.

JOSE specifies case-sensitive key-ID matching and leaves the identifier structure unspecified.
The 256-byte and control-character restrictions are ROM profile limits, not JOSE requirements.
[RFC 7515 section 4.1.4](https://www.rfc-editor.org/rfc/rfc7515.html#section-4.1.4),
[RFC 7517 section 4.5](https://www.rfc-editor.org/rfc/rfc7517.html#section-4.5).
OIDC requires exact issuer, audience, signature, expiry, and retained nonce validation.
[OIDC Core section 3.1.3.7](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation).

Five signed public-profile tests passed after an observed four-failure RED.
They cover the original 86-byte shape, 256/257-byte boundaries, UTF-8 byte counting, control rejection, and exact cache matching.
All related auth suites passed: 35 integration tests and five doctests. Clippy passed with warnings denied.
The Node fixture suite passed 39 tests, with no failures or skips.
The full local verifier remains an integration requirement; these focused checks do not replace it.

The same immutable token passed historical cryptographic replay at its retained callback-file host timestamp.
That timestamp did not come from an unverified token claim. Current-time verification correctly rejected that expired token.
A separately captured fresh original callback then passed current-time verification through the same frozen binary.
The frozen binary SHA-256 is `4cdbedd053d9103e613b461282ef66a07bfd1099e4d1574ba09d7cb1aeab0ed2`.
The standalone fixture and product declare Rust 1.99. The executed Rust and Cargo versions were 1.99.0.

Evidence resides in `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/sdk-authoring-20261007/`.
Its source inventories, locks, binary hashes, input hashes, actual process identities, cgroups, and results remain separate records.
`actual-sdk-result.json` is the original RED. The `current`, `historical`, and `fresh-current` after-key-fix records identify distinct cases.
The original callback and token remain private with mode 0600. Public evidence does not contain credential values.
Exact earlier browser scripts remain in the retained `capture-source-attempts/` evidence directory.

These runs used HTTP and borrowed producer Playwright tooling with Chromium. They are source-authoring reproductions only.
Trusted TLS, actual StudioHost callbacks, current-authority changes, two-engine coverage, and extracted-package admission remain open.
AP-UX-004 and AP-UX-005 still require original mutation and draft identity acceptance.
AP-UX-006 still requires actual session renewal, context retention, and principal separation.
R7 still requires production identity admission. AP-UX-015 concerns research deduplication, not production authentication.
The Astral Plane handoff has no acknowledged consumer reply.

### Host callback and proposed RSA metadata profile

The original public JWKS failed the unchanged private StudioHost parser with `Denied`.
Its fixture bytes match the retained original response: SHA-256 `bf47722ad1fd0ed9287cee5c54febde2a2ea40532a9cd028c4eaf176e0c7478a`.
The first public Host attempt failed before callbacks because the build-container asset path was absent on the execution host.
An explicit owned asset path corrected that fixture boundary. Both actual adapter hosts subsequently started and drained successfully.

Initial persistent Chromium launches failed with SIGTRAP under a long temporary path.
A separate default-browser attempt failed at the NixOS generic executable loader. Both failures remain retained.
The successful launcher reused the existing system Chromium and a short temporary directory inside the same hard run filesystem.
No sandbox or TLS bypass was added. A new socket-path preflight rejects long paths before browser launch.

The full public Host journey then completed provider identification, password entry, and the authorization-code callback.
Both SQLite and redb callbacks returned HTTP 401. Both current-session responses remained unauthenticated.
The frozen compiled source inventory and binary stayed unchanged. This is an observed Host compatibility RED in the HTTP authoring lane.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/host-authoring-ad7b11156fa0c89652f08af4f59e4a5f/`.
Trusted TLS, two-engine admission, current-authority mutations, and extracted-package proof remain open.

The selected correction keeps an RSA `n`/`e` verification profile. Host-approved issuer acquisition supplies the sole key source.
The shared opaque key-ID contract remains exact. Algorithm, use, key operations, key-count and response-size checks remain enforced.
Registered `x5c`, `x5t`, and `x5t#S256` fields are bounded unused metadata.
They cannot replace missing RSA components, select another key, authorize a URL fetch, or establish certificate trust.
The profile does not validate DER certificates, chains, certificate-to-key equality, or thumbprint-to-certificate equality.

RFC 7517 describes certificate metadata and requires consistency from its producer. It also permits ignoring unsupported JWK members.
Accepting bounded unused metadata does not prove those certificate properties. ROM does not claim a general-purpose JWK or PKIX implementation.
[RFC 7517 sections 4 and 4.7–4.9](https://www.rfc-editor.org/rfc/rfc7517.html#section-4).
Proposed local bounds are four certificate strings, 8192 bytes per string, and 16384 certificate-text bytes per key.
Base64 syntax and thumbprint encoding lengths may be checked without claiming certificate validation.
Reject null, wrong types, invalid encodings, empty chains, excessive metadata, duplicate fields, unsupported fields, and `x5u`.

| Canonical feedback | Source and generic contract | Regression or remaining proof | Original-consumer acceptance |
| --- | --- | --- | --- |
| AP-UX-006 | Studio authentication lifecycle; same-principal context retention with current authority | Real provider renewal, expiry, outage recovery, logout and principal switch; HTTP callback RED is narrower | Open; no acknowledged Astral Plane reply |
| AP-UX-004/005 | Mutation identity and navigation drafts across session changes | Receipt/current-authority behavior and client state retention require integrated application journeys | Open; isolated identity tests do not approve drafts |
| AP-UX-021/026 | Admin/guest boundaries and private observations | Current provider/link/User revocation, denied grants, principal-scoped private state | Open; no consumer reproduction accepted |
| AP-UX-015 | Lazy topic research and deduplication | Research identity and durable work contracts belong to separate AI/application acceptance | Open; not a production-authentication identifier |

The unchanged feedback marker is `1791379117`; the canonical intake contains 32 groups.
The earlier AP-UX-015 production-identity label was incorrect. R7 is the production-identity work package.

The corrective parser subsequently passed 28 Host unit tests and 13 integration tests, including original public JWKS bytes.
All-feature auth verification passed 35 integration tests and five doctests. Affected all-target Clippy passed with warnings denied.
The full provider Node suite passed 43 tests, including the socket-budget regression.
The four new metadata tests first produced two failures and two passes before the implementation changed.

A fresh public Host build captured 316 source files before and after compilation, with no changes during that fence.
Its SHA-256 is `1a77ca2a3df59ccc8d816ce91a368ba2f96745fc778556fc9e8f62b8912c19fe`.
New actual Authentik callbacks returned HTTP 303 on SQLite and redb. Both current-session responses were authenticated.
All Host and browser processes exited successfully and drained. Recorded cgroups reported no OOM or process-limit event.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/host-authoring-278cf655e14a129cbd858bffa016c015/`.
The final execution source and binary fence remained unchanged. Earlier binaries and failure evidence remain retained.
This GREEN is HTTP/Chromium source authoring evidence. It does not establish trusted TLS, other engines, lifecycle coverage, or extracted-artifact admission.

Subsequent resource-preflight work passed 48 Node tests. Actual filesystem observations replace the earlier profile-prefix flag in future browser records.
The new guard requires an empty cgroup descendant population after drain. It rejects incomplete or duplicate event records.
A later observation of both retained successful Host/browser cgroups found `populated 0`, with their recorded inode identities unchanged.
That later record is not an immediate original-run attestation. Earlier authoring results retain their original resource-evidence limits.

## Private browser trust prerequisite, 2026-10-07

The isolated tool lane acquired two pinned Debian NSS packages without global installation.
Their package hashes and source, license, advisory, and dependency records remain in private evidence.
The total retained NSS acquisition size was 22,901,946 bytes, below its 32 MiB cap.
The tool probe revalidated all ten projected executable/library files against the original package members.
It imported a synthetic CA into a fresh private NSS database and verified exact certificate DER readback.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/nss-private-trust-8b864cae6c63b27375327271/result.json`.

An actual local HTTPS fixture tested Chromium 138 and locked WebKit 2359 separately.
Chromium loaded the trusted synthetic certificate and rejected untrusted, wrong-SAN, and expired certificates with cause-specific errors.
WebKit reported `TLS support is not available` for the trusted case. This is a runtime prerequisite failure.
The initial negative classifier incorrectly admitted that runtime error for three negative cases.
An observed regression test corrected the classifier to require a certificate rejection.
The original failed run is unchanged. A separate record reclassifies all four WebKit cases as failures.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/tls-trust-8319e733874e2f181e01d214/result.json` and `subsequent-certificate-classification.json`.

Each case had an owned 4 CPU, 4 GiB, 512 PID cgroup and a 90-second deadline.
All case processes drained. The loopback listener was absent before and after each case.
No provider listener, global trust store, or certificate bypass was used.
These results establish the Chromium fixture trust prerequisite. They do not establish a production identity journey or artifact admission.

[GIO documentation](https://gnome.pages.gitlab.gnome.org/gtk/gio/overview.html) identifies glib-networking as its TLS backend implementation.
It also documents environment-scoped `GIO_EXTRA_MODULES` discovery.
The retained WebKit runtime contains no configured GIO TLS module path.
The runtime's pinned nixpkgs source selects glib-networking 2.80.1 with LGPL-2.1-or-later licensing.
That exact output is absent locally. No dependency acquisition or global runtime change has occurred.
Module compatibility, private WebKit trust, and both-engine production TLS remain open.

The worker's shared consumer handoff records AP-UX-006 session boundaries and AP-UX-021 privacy acceptance gaps.
No original-consumer acknowledgement has been received.

### Historical runtime supplement outcome

A fresh private Nix chroot store acquired the exact signed 35-path closure without a build or default-store mutation.
Its retained size was 106,586,813 bytes, below the 256 MiB tool envelope.
The initial verification stopped at the supervisor's 32 lifetime-entry guard, after per-path conversion subprocesses.
That failed record remains unchanged. Pure hash conversion then replaced those subprocesses with tested behavior.
A separate verification ran `nix store verify --recursive --sigs-needed 1` against the retained private store.
It checked all 35 NAR hashes, sizes, references, and paths against the frozen official metadata.
The TLS module's actual loader dependencies and hashes were recorded before browser execution.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/gio-verified-ca8ffbb6ca5c63505ee4ac82/result.json`.

The module was projected unchanged into an owned `GIO_EXTRA_MODULES` directory.
The reviewed Nix GnuTLS patch allowed private `NIX_SSL_CERT_FILE` trust configuration.
All eight trusted and certificate-negative browser cases then passed with Chromium and WebKit 2359.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/tls-trust-988eaf4ebe8c1c3c0ecdb5e8/result.json`.
No certificate bypass or global trust change was used. Each case drained its owned process cgroup and listener.

The pinned dependency remains GnuTLS 3.8.6.
[Current official advisories](https://www.gnutls.org/security-new.html) recommend newer versions for certificate-validation and name-constraint fixes.
The pinned source recipe has no reviewed backports of those fixes.
This proves historical synthetic-authoring compatibility only. Production dependency admission remains open.
Real provider HTTPS callbacks, current-authority browser lifecycle, and extracted-artifact admission also remain open.

The latest coordinated intake has 47 inspected references and includes AP-UX-033 guest invitations as application-owned behavior.
Explicit identity links, consent Resources, draft ownership, and idempotency remain generic ROM seams.
Synthetic `akadmin` authentication does not establish application administrator-only Studio authorization.
No original-consumer acknowledgement or acceptance has been received.

### Real-provider HTTPS dispatch checkpoints

The first four HTTPS cases read the original provider discovery document and JWKS over trusted private TLS.
They stopped before recording any callback. This was a browser fixture failure, not a ROM authentication rejection.
The immediate form visibility check was replaced with bounded form-or-callback waits.
Three pure tests cover delayed form readiness, direct SSO callbacks, and failed observations.
A fresh real-provider run then recorded callback status 303 for SQLite and redb in Chromium and WebKit 2359.
This observed change confirms the initial form-readiness problem. It does not complete session lifecycle acceptance.
The second run stopped after the callback, before its session check completed.
Redirect completion and current-session evidence require separate attribution before any product failure claim.

The two immutable results are retained under the isolated run filesystem:
`evidence/https-host-authoring-5b28011c20ecf6b06bb15845/result.json` and
`evidence/https-host-authoring-1bc0c5d984928e5f6bb3424e/result.json`.
Both runs retained unchanged selected native and fixture source fingerprints and the same compiled Host binary.
All four Host/proxy and browser process groups drained in each run.
Production patched-dependency admission, packaged execution, and original-consumer acceptance remain open.

A third fresh run separated callback, redirect, and session observations.
All four cases recorded callback 303 and current-session status 200 with the expected authenticated local user.
The session contained a CSRF token. Its value was not retained in public evidence.
The first browser protected read did not produce an observation; authorization and lifecycle acceptance remain open.
Evidence: `evidence/https-host-authoring-50eeefdd3419fb3c2ef21a99/result.json` beneath the isolated run filesystem.
No ROM current-session rejection was demonstrated by these results.

### Executed HTTPS current-authority lifecycle

A fresh source-authoring run passed all four SQLite/redb and Chromium/WebKit 2359 combinations.
Each case completed four real provider callbacks, including fresh login after each current-authority change.
An ordinary protected Resource was readable only with the authenticated linked local User.
Wrong-Origin requests returned 401. Missing and wrong CSRF tokens returned 403 without protected values.
Public `Command` mutations disabled and re-enabled the User, identity link, and identity provider.
Disable commits advanced revision 2; enable commits advanced revision 3.
Revoked reads returned 401. Re-enabling a Resource did not revive the invalidated browser session.
A fresh provider login restored the authorized read. Local Host logout returned 204 and subsequent reads returned 401.
This tests Host-local logout; provider session termination remains untested.

Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-45fe16fe6cae02a424412229/result.json`.
The result records unchanged selected native and fixture source fingerprints and the actual rebuilt Host binary.
The current build identity is `evidence/host-corrected-build-048d49b41ef305ffdcfc99f1/build-identity.json` beneath the isolated run filesystem.
Its sole native change from the preceding build is the corrected JWT feature condition on the key-ID helper export.
Independent no-feature, JWT, OIDC, and introspection test configurations passed.
Their exact scope and commands are retained in `evidence/auth-feature-matrix-e4587207f92752f13360b946/result.json`.

The browser fixture exposed two further defects after form readiness was corrected.
It called Fetch's numeric `Response.status` property as a function, then omitted the Host's `/api` route prefix.
Actual Response regressions and the observed browser 404 were retained before correction.
The corrected browser read uses one shared route declaration and returns only status and protected-value booleans.
The maintained provider Node suite passed 76 tests. These helper tests do not replace the four executed browser cases.

Each Host/proxy and browser cgroup drained before the provider stop request.
A new versioned launcher used an exclusive private stop file with actual device, inode, birth, owner, mode, and content checks.
The launcher stopped its exact three owned containers; the original launcher and all failed evidence remain unchanged.
The terminal provider snapshot briefly recorded `populated 1` before the final wrapper exited.
A separate post-exit snapshot verified `populated 0` and retained actual limits and resource events.
Evidence: `evidence/provider-post-terminal-drain-a888ba27d4af222c5ec179e0.json` beneath the isolated run filesystem.

These results remain historical-runtime source-authoring evidence. They do not admit GnuTLS 3.8.6 for production use.
Patched TLS dependencies, provider-image source attestation, and extracted-artifact execution remain open.
Actual provider key rotation, restart, expiry, outage, denied provider grants, and provider-wide logout remain open.
Application administrator-only policy and original Astral Plane AP-UX-006/021 acceptance also remain open.

### Proposed patched TLS supplement; not acquired

The lowest-cost candidate uses Debian's existing security backports with the retained GLib module and browser revision.
The [official Debian advisory](https://security-tracker.debian.org/tracker/DSA-6281-1) identifies GnuTLS `3.8.9-3+deb13u4` as fixed.
Its [changelog](https://metadata.ftp-master.debian.org/changelogs/main/g/gnutls28/gnutls28_3.8.9-3%2Bdeb13u4_changelog) records the 3.8.13 security patch set.
The [libtasn1 tracker](https://security-tracker.debian.org/tracker/CVE-2025-13151) identifies `4.20.0-2+deb13u1` as fixed for its stack-overflow issue.
The retained Nix recipes select GnuTLS 3.8.6 and libtasn1 4.20.0 without these reviewed patches.
These findings justify replacement; they do not prove runtime compatibility or a clean dependency closure.

| Proposed immutable input | Official bytes | SHA-256 |
| --- | ---: | --- |
| [libgnutls30t64_3.8.9-3+deb13u4_amd64.deb](https://packages.debian.org/trixie/amd64/libgnutls30t64/download) | 1469024 | `18a8bdfd91c7e3bcb01719d55a2b56849c7160b34f1f52b1c4fdfdd41bf1352b` |
| [libtasn1-6_4.20.0-2+deb13u1_amd64.deb](https://packages.debian.org/trixie/amd64/libtasn1-6/download) | 50112 | `23fec6e06583ce2bad9b2c04c9b485e90440e259b1abf8677cd80d3ce60831ad` |

Use only the corresponding `https://deb.debian.org/debian/pool/main/` files, without redirects or fallback versions.
Retain official source descriptors, patch changelogs, and copyright files before execution.
The [GnuTLS copyright record](https://metadata.ftp-master.debian.org/changelogs/main/g/gnutls28/gnutls28_3.8.9-3%2Bdeb13u4_copyright) declares LGPL-2.1-or-later for the core library.
It also describes stronger license requirements for the linked GMP library; retain the complete notices.
The [libtasn1 copyright record](https://metadata.ftp-master.debian.org/changelogs/main/libt/libtasn1-6/libtasn1-6_4.20.0-2%2Bdeb13u1_copyright) supplies its declared library and auxiliary-code licenses.
No Rust dependency or MSRV change is proposed.

The GnuTLS [declared dependencies](https://packages.debian.org/trixie/libgnutls30t64) require glibc 2.38, Nettle 3.10, and GMP 6.3.0 or newer.
The retained runtime versions meet these stated minimums. Actual ELF symbols and loader resolution still require verification.
Read-only `readelf` inspection found basename `libgnutls.so.30` and `DT_RUNPATH` in the admitted GIO module.
However, the retained MiniBrowser wrapper overwrites `LD_LIBRARY_PATH`; an inherited environment override alone will not suffice.
Use a new private wrapper that preserves original asset paths and prepends only admitted crypto paths and their recorded dependencies.
Keep the original WebKit runtime and wrapper unchanged. Record the new supplement as a runtime-graph deviation until reviewed.
Verify actual loaded library paths in the owned browser process group; a static loader probe alone cannot establish runtime selection.

The proposed acquisition cap is 32 MiB of additional retained tool material, within the existing private 12 GiB overlay filesystem.
Keep the existing aggregate GIO tool cap of 256 MiB and the total retained 32 GiB environment unchanged.
Use a 10-minute acquisition ceiling, 30-second request deadlines, and the existing 4 CPU/4 GiB/512-process supervisor limits.
Admit archive paths, types, links, and sizes before selecting regular library bytes; do not execute package scripts or install globally.
Project unchanged regular library bytes under their SONAME filenames rather than extracting arbitrary archive symlinks.
Stop if either pinned hash, dependency set, platform, loader requirement, or retained-byte limit differs.
No additional dependency acquisition is implied by this proposal.

Current [p11-kit advisories](https://security-tracker.debian.org/tracker/source-package/p11-kit) also affect the retained 0.25.5 dependency.
Some require RPC access; one documented overflow is limited to 32-bit systems.
That narrower applicability does not establish a clean closure or permit an unreviewed replacement dependency.
Complete the dependency threat-profile review before production admission.
After actual ABI and library-selection checks, rerun trusted and certificate-negative cases in both browser engines.
Then repeat the real provider lifecycle from fresh evidence. Preserve all historical-runtime results separately.

### Executed private patched-crypto acquisition

The approved two-package acquisition completed with both expected hashes on 2026-10-07.
Evidence: `evidence/crypto-tools-e08d62bfc46542ea409ca49c/result.json` beneath the isolated run filesystem.
Outer supervision: `evidence/crypto-supervision-57babe89383cac9bd75153e4/result.json`.
The download and tools ran inside an owned 4 CPU/4 GiB/512-process cgroup. Both groups drained.
Retained crypto material measured 9,279,075 bytes; retained GIO material measured 106,766,941 bytes.
These remain below the separate 32 MiB and combined 256 MiB caps.
All archive members passed bounded Debian admission before regular library projection. No package scripts or global installation executed.
The static loader selected the projected GnuTLS and libtasn1 libraries. This does not prove actual browser selection.

The first actual eight-case probe passed Chromium's four cases. WebKit's four cases failed the new library-selection admission.
Evidence: `evidence/tls-trust-d63bb8f4f3209358db3a5584/result.json`. Preserve these failures.
A second diagnostic run also failed: `evidence/tls-trust-299059ae295a9ca803b0bb11/result.json`.
Expected navigation or certificate errors alone cannot satisfy actual patched-library admission.
The map collector incorrectly required a child cgroup instead of permitting the direct owned cgroup.
An additional regression accepts exact owned root and launcher membership. It rejects other roots and arbitrary descendants.
The corrected collector still requires both selected crypto libraries and rejects old copies.

The retained Nix GnuTLS has a private `NIX_SSL_CERT_FILE` patch. Debian's library does not supply that extension.
The [upstream trust-store implementation](https://raw.githubusercontent.com/gnutls/gnutls/3.8.9/lib/system/certs.c) uses its configured system trust store.
Actual trusted WebKit navigation rejected the certificate with the Debian supplement and the previous environment-only trust configuration.
No certificate-validation bypass or global trust change is proposed.
Private trust configuration for the patched runtime remains a fixture prerequisite.

The [CVE-2026-2100 record](https://security-tracker.debian.org/tracker/CVE-2026-2100) excludes p11-kit 0.25.5; vulnerable code appeared in 0.25.6.
[CVE-2026-18938](https://security-tracker.debian.org/tracker/CVE-2026-18938) is limited to 32-bit systems; this fixture uses amd64.
[CVE-2026-13757](https://security-tracker.debian.org/tracker/CVE-2026-13757) affects the retained version and requires access to its RPC Unix socket.
Verify actual module configuration and process/socket exposure before claiming scoped non-exposure.
The retained GLib 2.82.1 recipe has no reviewed backports for later [GString](https://security-tracker.debian.org/tracker/CVE-2025-6052), [character insertion](https://security-tracker.debian.org/tracker/CVE-2025-4373), or [buffered-stream](https://security-tracker.debian.org/tracker/CVE-2026-0988) fixes.
Two crypto-library replacements do not admit the whole GLib, Chromium, or WebKit graph.

### Remaining provider lifecycle contracts from source review

The following experiments remain proposed and unexecuted.
Host credentials enforce original token expiry. The OIDC key and proof deadline is at most 30 seconds.
An outage test must distinguish cached access before the deadline from failed revalidation afterward.
Source review predicts that transport failure maps to `Denied` and removes the session. Actual behavior remains unmeasured.
Record same-cookie recovery separately from fresh login. Preserve AP-UX-006/021 consumer acceptance requirements.

The pinned [provider model](https://raw.githubusercontent.com/goauthentik/authentik/e5a0d2f7572cb776eee7a3e9355937ce38973761/authentik/providers/oauth2/models.py) admits token-validity and signing-key settings.
Its [ID-token code](https://raw.githubusercontent.com/goauthentik/authentik/e5a0d2f7572cb776eee7a3e9355937ce38973761/authentik/providers/oauth2/id_token.py) uses grant expiry.
Propose a synthetic 30-second lifetime with real wall-clock expiry. Do not infer trusted time from token claims.
The [JWKS code](https://raw.githubusercontent.com/goauthentik/authentik/e5a0d2f7572cb776eee7a3e9355937ce38973761/authentik/providers/oauth2/views/jwks.py) exposes the current signing key without automatic historical-key overlap.
Rotation must prove new-key callback success and old-session rejection after the proof deadline.
Upstream source supports this design inference; image-to-source attestation remains open.
The [authorization code](https://raw.githubusercontent.com/goauthentik/authentik/e5a0d2f7572cb776eee7a3e9355937ce38973761/authentik/providers/oauth2/views/authorize.py) rejects grants absent from provider configuration.
Observe an actual error callback and no authenticated Host session. Restore configuration and prove a fresh successful callback separately.

[Provider-wide logout](https://docs.goauthentik.io/add-secure-apps/providers/oauth2/frontchannel_and_backchannel_logout/) requires explicit RP front-channel or back-channel support.
Existing local Host logout does not establish that support or end the provider session.
Restart requires a controlled new-birth adoption seam; the existing engine deliberately rejects changed process identity.
Keep restart, outage, expiry, rotation, denied grants, provider logout, patched runtime, artifact execution, and original-consumer acceptance open.

### Executed patched-library attribution; private trust still fails

The corrected map collector recorded both projected libraries in actual owned WebKit processes, with verified births and cgroup membership.
Evidence: `evidence/tls-trust-065e54be6c2c62fa536a113b/result.json` beneath the isolated run filesystem.
All four WebKit cases selected the exact patched GnuTLS and libtasn1 paths. No old crypto copies appeared.
All eight case groups drained. The overall probe failed because trusted WebKit navigation rejected the certificate.
The three certificate-negative observations cannot distinguish hostname or expiry rejection while the trusted-CA prerequisite fails.
Chromium's four cases passed with its existing private NSS trust configuration.

The actual Debian library contains `/etc/ssl/certs/ca-certificates.crt` as its compiled trust-store path.
The host path is a symlink to an immutable Nix CA bundle. Do not change either host path or bundle.
[WebKit source](https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WebCore/platform/network/soup/SoupNetworkSession.cpp) offers `WEBKIT_TLS_CAFILE_PEM` only in developer mode.
That identifier is absent from the actual retained WebKit library; this source option does not establish an available runtime seam.

The proposed next fixture uses a new mount namespace with private propagation.
Bind an exclusive private directory onto `/etc/ssl/certs` read-only inside that namespace.
Its only certificate bundle contains the synthetic CA, or no CA for the untrusted negative.
Verify process birth, group, cgroup, namespace inode, mount information, bundle identity, and host trust identity before and after execution.
Keep the existing 90-second case ceiling, 4 CPU/4 GiB/512-process limits, and hard 8 GiB filesystem.
No global trust, immutable store, shared namespace, or certificate-validation bypass is part of this proposal.
Execution awaits the coordinator's exact namespace allocation.

The maintained provider Node suite passed 83 tests after these fixture additions.
Log: `/var/tmp/rom-010-identity-crypto-node-20261007.log`.
These helper tests do not replace actual private trust, provider lifecycle, packaged execution, or consumer acceptance gates.

### Executed patched-crypto TLS prerequisite

The private mount namespace restored trusted WebKit navigation with the unchanged Debian library bytes.
The first namespace run stopped three negative cases before browser launch because fast mount commands lost target-identity attribution.
Evidence: `evidence/tls-trust-8b8bd16c57e93c825c0c456e/result.json`. This fixture failure remains preserved.
A persistent owned Node bridge now waits for exclusive file acknowledgement after actual bridge-birth admission.
It invokes only the exact pinned mount tool and records its result, hash, arguments, and spawned PID.
The evidence does not claim continuous birth admission for that short-lived tool PID.
Signals remain limited to owned bridge/wrapper groups; the tool inherits the verified namespace and cgroup.

The final fresh eight-case probe passed: `evidence/tls-trust-ffda05d57c88b265a3fcc9a4/result.json`.
Both engines passed trusted navigation followed by untrusted CA, wrong SAN, and expired certificate rejection.
Each WebKit case verified both patched crypto libraries in actual process maps, without old copies.
Each WebKit case verified a different mount namespace from the host and a read-only owned CA directory.
The kernel reused namespace inode numbers between serial drained cases. The evidence does not claim globally unique inode numbers.
Host namespace, CA realpath, and CA SHA-256 remained unchanged. Every case cgroup drained.
The 14 fixture-source hashes matched before and after this final probe.
The previous successful probe lacks this full helper fence; preserve its distinct evidence and narrower source claim.

The maintained provider Node suite passed 86 tests.
Log: `/var/tmp/rom-010-identity-crypto-namespace-node-20261007.log`.
This establishes the isolated patched-crypto certificate prerequisite within its limits.
It does not admit the remaining GLib/browser dependency graph or repeat the real provider lifecycle with this supplement.
Fresh provider execution, image source attestation, extracted-artifact admission, and original consumer acceptance remain open.
