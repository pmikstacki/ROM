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

Blob upload composition is separate from this authentication and asset slice. Do not assume that these tests prove combined blob shutdown.
The real-provider integration tests require Node and the installed `demo/provider-fixture` dependencies.
