# Protected Studio preview deployment

This plan uses inspected local infrastructure. It does not claim that the new Studio is deployed.
The current preview returns HTTP 200 but serves the historical static mock.

## Existing infrastructure

The `rom-studio` NixOS container has autostart and shares the host network.
Its nginx listener uses `127.0.0.1:44173`. A read-only mount supplies `/srv/studio` from `/var/lib/rom-studio-preview/site`.
The host Caddy listeners bind `10.66.0.2` on ports 80 and 443.
The existing Studio proxy removes `/rom-studio/` before forwarding to nginx.

The actual Rust host expects its configured base path. Its proxy must preserve `/rom-studio/` when forwarding requests.
Changing only the static HTML file cannot supply authentication, Resource operations, or live queries.

## Deployment sequence

1. Complete actual host, provider, browser, and accepted-work shutdown tests.
2. Produce the verified source archive and exact Studio asset archive.
3. Install the accepted binary and assets into a versioned preview directory.
4. Keep database and optional BlobStore data outside the versioned asset directory.
5. Configure the Rust host with the public HTTPS origin and the approved provider endpoints.
6. Replace the static nginx upstream with the actual host service.
7. Preserve the base path in the Caddy proxy.
8. Verify HTTP assets, login callbacks, generic operations, and live snapshots over the VPN address.
9. Restart the preview service and container. Verify retained Resource data and renewed authentication.

The preview must use immutable accepted assets. It must not mount the source worktree or its build cache into the serving service.
The service must restart automatically. Its shutdown must drain accepted work through the host contract.
Keep the previous mock deployment available until the new preview passes its server-side checks.

## Identity and TLS conditions

The current human provider fixture permits numeric loopback HTTP callbacks only.
The current demo launcher derives its public origin from its loopback listener.
These defaults are suitable for local acceptance but cannot establish a working MacBook VPN login.

A VPN preview needs an explicit HTTPS origin, reachable authorization endpoint, and exact public callback.
Its configured issuer must match the registered IdentityProvider Resource and verified token claims.
The server's OIDC HTTP client must trust the configured HTTPS certificate chain.
Do not disable certificate verification to accommodate a private CA.

If a fixture provider is used, label it as a demonstration provider. It is not a production identity server.
Expose its endpoints through the existing protected gateway. Do not open unrelated public ports.
Approved network endpoints and secrets remain host configuration. StudioSettings can select only an already-approved enabled provider.

## Evidence boundaries

Local inspection found the container, loopback listener, and protected gateway described above.
A local HTTPS GET with certificate checking disabled returned 200 for the historical mock.
That probe does not establish certificate trust, authentication, or reachability from the owner's MacBook.
The release report must identify separately the server-side checks and any independently verified client-side access.

The NixOS instructions require preserved LAN SSH and rollback protection before networking changes.
This deployment must preserve WireGuard, firewall, DNS, unrelated services, and Windows disks.
If networking changes become necessary, apply those instructions before activation.
