# Protected preview deployment acceptance

These commands are prepared. They were not run against a deployed 0.0.2 service.
Activation requires the final accepted source, binary, asset inventory, and provider dependency identity.

The [scoped activation procedure](rom-0.0.2-scoped-preview-activation.md) is the operative procedure for the existing preview.
It supersedes the whole-host build and gateway activation sequence retained below.
Use its existing read-only bind, retained container directories, exact closure selector, and application-only rollback.
Do not combine the alternative layouts or perform a whole-host switch for this application cutover.

## Review findings

The prepared service keeps data outside the immutable release directory.
Only `/var/lib/rom-studio` is writable by the Rust service.
The provider uses volatile demonstration state. Restart requires new login; it must not resurrect browser sessions.
Both processes use a dedicated UID/GID, bounded stop time, `SIGTERM`, and restart on failure.
The container starts automatically and binds loopback upstreams only.
The gateway adds no firewall, DNS, WireGuard, or SSH settings.

Both services load credentials through systemd. No secret value enters the Nix store, Git, example JSON, or process arguments.
The external private directory must be root-owned. The retained data directory must belong to UID/GID `44173`.
The accepted artifact directory must be root-owned, read-only to the service, and separate from data and credentials.

Replace the old static-preview module; do not import the new module as an additive overlay.
The old module starts nginx on `127.0.0.1:44173` and uses a prefix-stripping proxy.
Keeping it alongside the Rust service would cause a port collision or invalid routing.

Use one canonical public host. Its Studio origin, issuer, authorization URLs, callback, and Caddy listener must agree.
The prepared example uses `bdziam.home.arpa`. Set `publicHost` and both external profiles consistently if the accepted preview uses `10.66.0.2` instead.
An old URL on another gateway host is not evidence that the canonical preview failed.

No source-review blocker remains in these boundaries.
Final deployment depends on artifact acceptance, scoped rollback preparation, and actual application checks, rather than a new owner approval.
Independent client checks remain necessary before any claim of MacBook access.
Host networking changes remain subject to the host's separate access and rollback instructions.

## Record the accepted inputs

Record the source revision/tree, Cargo.lock hash, executable hash, extracted Studio inventory, and provider source/dependency hashes.
Retain the admitted release manifest and its independent verifier result.
The current source-and-assets manifest does not by itself hash the separately installed native executable.
Bind that executable to the final release build and record its hash before installation.
Do not select an arbitrary binary from an old shared target.

Use the independent artifact verification procedure in `scripts/release-artifacts/README.md`.
Run it against the completed output from the accepted source checkout.
Do not substitute failure-stage evidence or historical source-only artifacts.

## Inspect the host before replacement

Read `/etc/nixos/ai-server/AGENTS.md` again.
Compare the current `rom-studio.nix` and `studio-container.nix` with the recorded prior versions.
Preserve unrelated local edits and all root-only secrets.
Record the current system closure and old preview configuration for rollback.

```sh
readlink -f /run/current-system
systemctl status container@rom-studio.service --no-pager
nixos-container run rom-studio -- systemctl status nginx.service --no-pager
ss -ltn '( sport = :44173 or sport = :44174 )'
```

These commands inspect service state. Their success does not authorize changing SSH or WireGuard.

## Historical whole-host proposal — superseded

This section retains the earlier proposal for comparison. Do not execute it for the selected application-only cutover.
Use the container-only build and scoped activation commands in the operative procedure instead.
Its fresh Ethernet, Wi-Fi, and VPN SSH prerequisites concern networking changes, not the selected application-only scope.

Install the accepted files into the versioned release directory.
Create external data, configuration, and private directories with the ownership described above.
Keep client-secret values out of command output and logs.
Build the candidate NixOS configuration without switching the running system.

```sh
nixos-rebuild build --flake /etc/nixos/ai-server#bdziam
```

Before gateway activation, arm the host's timed rollback procedure.
Verify fresh Ethernet, Wi-Fi, and VPN SSH access as the host instructions require.
Cancel rollback only after the independent access checks and application checks succeed.
Do not invent successful client access from a local listener or an existing SSH connection.

## Probe the actual HTTPS gateway

Use the configured public CA certificate with verification enabled.
The CA certificate is public trust material; its private key must remain protected.
Set `PREVIEW_ORIGIN` to the canonical origin and `PREVIEW_CA_CERT` to the trusted CA certificate path.

```sh
curl --fail --cacert "$PREVIEW_CA_CERT" "$PREVIEW_ORIGIN/rom-studio/" --output /var/tmp/rom-preview-index.html
curl --fail --cacert "$PREVIEW_CA_CERT" "$PREVIEW_ORIGIN/rom-studio/auth/providers"
curl --fail --cacert "$PREVIEW_CA_CERT" "$PREVIEW_ORIGIN/rom-studio/auth/session"
curl --fail --cacert "$PREVIEW_CA_CERT" "$PREVIEW_ORIGIN/rom-studio-provider/.well-known/openid-configuration"
```

Check the exact issuer and public endpoint URLs in discovery.
Unauthenticated session status must not disclose credentials or grant Resource access.
Verify a real login through the public authorization endpoint and exact callback.
Use Chromium and WebKit with trusted certificates. Do not disable certificate verification to obtain a passing result.

After login, test discovery, two different generic Resources, mutation, live removal after filtering, and an attachment round trip.
Test current denial and logout. Closed sessions must not retain live data or accepted authority.
Retain source identity, origin, backend, asset identity, and test outcomes with the browser evidence.

## Test lifecycle and retained data

```sh
nixos-container run rom-studio -- systemctl restart rom-studio.service
nixos-container restart rom-studio
nixos-container run rom-studio -- systemctl status rom-studio.service rom-studio-provider.service --no-pager
```

Verify retained Resource and attachment data after each restart.
Old browser sessions must require new authentication. The provider's volatile state does not establish durable identity sessions.
Test automatic restart with a trusted local service failure after the normal restart checks.
Record expected recovery separately from clean `SIGTERM` draining.

The service stop limit is finite. A forced kill can leave durable work to recover.
It cannot erase a committed mutation or authorize a duplicate action.

Finally, verify the canonical URL from the owner's MacBook over the VPN.
Record that client result separately. Local server-side HTTPS checks do not prove MacBook routing, DNS, or CA trust.
