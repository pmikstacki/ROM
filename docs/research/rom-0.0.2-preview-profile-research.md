# Protected preview profile research

This report covers source research, host changes, and prepared deployment files.
It does not claim that the new preview is deployed or reachable from the owner's MacBook.

## Decision

Retain the existing VPN gateway and container. Replace the static mock with the accepted Rust host and immutable Studio assets.
Keep the database and attachment directory outside the release directory.
Use a separate demonstration identity provider behind the same HTTPS gateway.

The host configuration approves a public HTTPS issuer and public authorization endpoint.
A separate trusted profile can route token and JWKS acquisition to exact numeric-loopback HTTP endpoints.
This profile changes network routing. It does not change token issuer validation, current IdentityProvider checks, or current User binding.
It cannot come from browser requests or Resource configuration.

## Primary sources

Caddy's `handle_path` removes the matched prefix. The Rust host expects `/rom-studio/` in its request path.
The prepared gateway therefore uses `handle`, without a URI rewrite.
The provider mount also retains its prefix. [Caddy path handling](https://caddyserver.com/docs/caddyfile/directives/handle_path).

Caddy reverse proxy sends upstream requests and supplies forwarding headers.
The existing gateway owns TLS. Both new upstream listeners remain numeric loopback HTTP.
No firewall, WireGuard, DNS, or SSH change is required by the prepared module.
[Caddy reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy).

The upstream identity provider documents mounted request URLs and TLS-offloading proxy support.
The shared fixture preserves `req.originalUrl`, removes only its approved mount before dispatch, and enables proxy handling for its HTTPS profile.
The strict loopback fixture keeps its previous defaults.
[oidc-provider mounting and TLS-offloading documentation](https://github.com/panva/node-oidc-provider/blob/main/docs/README.md#trusting-tls-offloading-proxies).

The public issuer must match the verified ID token issuer. A loopback acquisition route does not establish identity by itself.
The existing Rust verifier still checks the signature and claims through approved keys.
[OpenID Connect ID token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation).

Systemd supervises services and defines restart and stop limits.
The prepared units use `Restart=on-failure`, `SIGTERM`, and a 45-second stop limit.
A stop limit bounds process shutdown; it does not promise that every external failure completes within that time.
[Systemd service reference](https://github.com/systemd/systemd/blob/main/man/systemd.service.xml).

Caddy's internal CA requires client trust. Serving HTTPS does not install that trust on the MacBook.
Certificate verification must remain enabled. Server checks and MacBook access remain separate acceptance results.
[Caddy local HTTPS](https://caddyserver.com/docs/automatic-https#local-https).

## Host implementation

`TrustedLoopbackBackchannel::new(issuer, token_endpoint, jwks_endpoint)` creates host-only approval data.
`HostConfig::provider_backchannel(authority, profile)` binds it to an approved provider.
Registration rejects unknown or duplicate authorities, issuer mismatches, and non-HTTPS public origins or issuers.
Internal endpoints must be canonical numeric-loopback HTTP URLs without credentials, queries, or fragments.
Token and JWKS endpoints must share their internal origin.
Existing public endpoint approval still requires the issuer's public origin.

Only token and JWKS requests use the selected internal routes.
Public authorization, callback, and issuer values remain unchanged.
Redirects remain disabled in the bounded HTTP client. No TLS verification switch was added.

The new `startHttpsProvider` fixture requires a canonical HTTPS issuer with one mount segment.
Its exact callback must use the same public origin and the expected Studio callback path.
The service entry point reads a bounded external profile and bounded external client-secret file.
It prints readiness metadata, without credentials. It accepts no HTTP configuration controls.
The provider uses demonstration accounts and volatile grants, keys, and sessions. It is not a production identity server.

## Executed checks

- Host RED compile: the new approval type and builder did not exist. Evidence: `evidence/rom-0.0.2/task4/backchannel-red.log`.
- Host test and Clippy gate: 31 tests passed; warnings were denied. Evidence: `evidence/rom-0.0.2/task4/backchannel-final.log`.
- Fixture tests: six tests passed. Existing code/PKCE cases, HTTPS issuer claims, mount rejection, external credentials, and SIGTERM were covered.
  Evidence: `evidence/rom-0.0.2/task4/preview-fixture-final.log`.
- Native actual-provider regression: three host integration tests passed after factory extraction. The main test covers SQLite and redb.
  Evidence: `evidence/rom-0.0.2/task4/fixture-native-recheck-final.log`.
- Both prepared Nix files passed syntax parsing. This is not a NixOS build or service activation result.

The HTTPS fixture test uses a controlled proxy-header simulation over loopback HTTP.
It verifies a real signed token with its public HTTPS issuer and performs token/JWKS acquisition without forwarded HTTPS headers.
It does not prove a complete Rust-host browser login through the deployed Caddy gateway.

## Remaining deployment acceptance

1. Freeze and accept the exact binary, Studio assets, and provider fixture dependency tree.
2. Record artifact hashes and source binding. Install them into a versioned, read-only release directory.
3. Create external data, configuration, and private credential directories with restricted permissions.
4. Build the prepared NixOS configuration without activation. Compare unrelated host configuration before replacement.
5. Before gateway activation, preserve a rollback route and verify the required independent SSH paths.
6. Verify login, generic mutations, live snapshots, and attachments through the actual HTTPS gateway with certificate checks enabled.
7. Verify service and container restart. Resource data must remain; old host sessions must require new authentication.
8. Record MacBook VPN reachability and CA trust separately when independently observed.

No deployment, network activation, disk deletion, or Git staging was performed for this slice.

## Client-secret encoding correction

The source review found a valid-provider interoperability error in `client_secret_basic`.
OAuth requires form encoding each credential before the Basic header encoding.
The previous host passed raw credentials to reqwest. Special characters could cause a valid provider to reject login.
[RFC 6749 section 2.3.1](https://www.rfc-editor.org/rfc/rfc6749#section-2.3.1).

The host now uses a shared encoding helper. Its network test checks the exact received Authorization header for synthetic special-character credentials.
A real fixture-provider test also accepts a form-encoded secret containing colon, space, percent, plus, and exclamation characters.
The fixed host suite has 32 passing tests and a successful Clippy gate.
Evidence: `evidence/rom-0.0.2/task4/basic-encoding-red.log` and `evidence/rom-0.0.2/task4/preview-host-final.log`.
The fixture suite has seven passing tests in `evidence/rom-0.0.2/task4/preview-fixture-final-2.log`.
Only synthetic public test credentials appear in these assertions.

The prepared container module also passed NixOS module evaluation using the host's locked nixpkgs input.
Its evaluated service configuration is retained in `evidence/rom-0.0.2/task4/preview-nix-evaluation-final.json`.
This evaluation does not build or activate the container.
