# ROM Studio host

This optional crate serves built Studio assets and generic ROM HTTP operations. It does not change the Resource runtime.

Create a `HostConfig` with an exact public origin, base path, asset directory, and explicit host configuration reader.
Register `User`, `IdentityProvider`, `IdentityLink`, and optional `StudioSettings` Resources with application policies.
Install the `IdentityGate` on that Runtime. Allow the configuration reader explicitly. Never obtain this Actor from browser input.

Approve each OIDC provider in `OidcProviderConfig`. The provider Resource must match the approved issuer, client ID, and OIDC profile.
The host pins authorization, token, and JWKS endpoints. Mutable Resources cannot introduce network trust sources.
HTTPS is required. Numeric-loopback HTTP requires the explicit test exception.

`StudioHost::new(runtime, config)` validates configuration and loads a bounded asset inventory.
Use `serve(listener, stop)` to serve and drain owned work. Use `router()` only when the enclosing host owns equivalent shutdown.
Call `shutdown()` before the enclosing host closes Runtime. Application callers must keep the host and Runtime clocks consistent.

Browser sessions use opaque HttpOnly cookies. Unsafe requests require exact Origin and `x-rom-csrf`.
The server retains the original finite ID token and verifies it again when its 30-second proof expires.
This does not extend token expiry or issue a refresh grant. Current User, IdentityLink, and IdentityProvider activation is checked again.
An established stream retains its original Actor lease. On `identity_expired`, revalidate the session and reopen a fresh live query.
Logout cancels every stream owned by that session. Sessions are in memory; a host restart requires a new login.

`StudioSettings` is an ordinary Resource with `primary_provider: Option<String>`.
Use `HostConfig.settings(id)` to read its provider selection. An unapproved or disabled selection returns no primary provider.
Endpoint and secret approval remains in host configuration.

Use `HostConfig.blobs(service)` to add authenticated binary transport. The service must use the exact same Runtime instance.
Register `rom_blob::definition()` and explicitly allow its trusted worker through the application identity gate.
The host uses the service's validated chunk limits. Accepted uploads remain under service supervision when callers disconnect.
Reservation, upload, and detachment require Origin and CSRF checks. Downloads use current session and Resource authorization.
Browser responses use Resource projections. They do not expose raw object receipts or backend errors.
An uncertain provider publication returns `outcome_unknown`. A retry verifies the existing immutable object before attachment.
Use `GET blobs/attachment?id=...` for downloads. Query encoding preserves Resource IDs such as `.` and IDs that contain `/`.
`GET blobs/capabilities` requires current Blob Resource and store-field discovery authority.
Use `HostConfig.blob_store_discovery(...)` to permit disclosure of configured store names. The default denies all names.
Capabilities describe available transport operations. They do not grant Resource mutation or read permissions.
Real TCP tests pause accepted authentication and upload work, then disconnect both callers and stop intake.
Shutdown waits for both private test barriers before closing Runtime. A separate test verifies draining after an authentication panic.
These tests do not prove operating-system signal handling in the demo process. That acceptance check remains separate.
Observation waits check logout and the original Actor lease while current identity binding waits for storage.
They do not renew an existing stream Actor. Accepted identity checks retain their supervision permit until completion.
The real-provider integration tests require Node and the installed `demo/provider-fixture` dependencies.

The RS256 JWKS profile verifies keys from RSA `n` and `e` components supplied by the approved issuer endpoint.
Key IDs use the shared opaque ROM profile: nonempty, at most 256 UTF-8 bytes, and no Unicode control characters.
Matching preserves exact case and Unicode representation. JOSE does not prescribe this local identifier length limit.

Registered `x5c`, `x5t`, and `x5t#S256` members are bounded unused metadata.
The host checks their JSON types and encoding syntax. It does not validate certificates, chains, key equality, or certificate thumbprint equality.
These fields never replace RSA components or establish another trust source. `x5u` and unsupported members are rejected; no certificate URL is fetched.
Certificate arrays contain one through four strings, each at most 8192 bytes, with at most 16384 text bytes per key.
Thumbprints use base64url encoding with decoded lengths of 20 or 32 bytes. These checks do not prove their certificate association.
The complete JWKS response remains limited to 65536 bytes and eight keys.
Algorithm, use, key-operation and duplicate-key checks remain enforced. These finite limits define ROM's profile, not general JWK or PKIX support.
