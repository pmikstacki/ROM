# ROM 0.0.2 preview activation

Date: 2026-10-05 UTC.

## Release identity

The preview runs source revision `8652fcd2d296f2bfe3102cdc0dfe977fecf1b034`.
Its immutable host directory is `/var/lib/rom-studio-preview/site/releases/rom-0.0.2-8652fcd2d296`.
The container selects it through `/opt/rom-studio`.

| Item | SHA-256 |
| --- | --- |
| `bin/rom-demo` | `81599303a927451874908fe571ed87304431b3c77f547a1ef4da21010655cd7a` |
| `studio/dist/index.html` | `80b809e279e0c3948b078542142a60e05de6cd5b4e3573f826ae6cd5b49a16f3` |
| `provider/package-lock.json` | `e97914580496c3856a62dd4b643d83b8d151f3feed7a97a928f695b4684c752e` |

The binary was built from the recorded checkout with `cargo build --release -p rom-demo --features studio --locked`.
The Studio assets came from the accepted release archive. The provider package lock and runtime source hashes match the accepted source archive.
The new directory is root-owned and read-only. Its `SHA256SUMS` file passed verification.
The prior preview directories remain available for rollback.

## Activation and checks

The container closure is `/nix/store/jirkqmll45pap3drhmm2aqfvgkkvq5ic-nixos-system-rom-studio-24.11.20250630.50ab793`.
It changes the application release name while keeping the existing container and network configuration.
The existing host Caddy routes retain `lb_try_duration 5s`.

| Check | Result |
| --- | --- |
| `caddy.service` and `container@rom-studio.service` | Active |
| In-container `rom-studio.service` and `rom-studio-provider.service` | Active |
| Nginx in the container | Inactive |
| `https://10.66.0.2/rom-studio/` | HTTP 200; served index hash matches the accepted release |
| `https://10.66.0.2/rom-studio/auth/providers` | HTTP 200 |
| `https://10.66.0.2/rom-studio/api/discover` without a session | HTTP 401 |
| Provider discovery endpoint | HTTP 200 |
| Chromium login through the VPN gateway | Demo login succeeded; the inventory Resource loaded |
| Live query through the VPN gateway | HTTP 200; the browser stopped the live query without page errors |
| Full `container@rom-studio.service` restart | Both app services returned active; Studio/provider returned HTTP 200 |
| Preview database | 106,496 bytes before and after restart |

The HTTPS checks used `/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt`.
The rollback script and prior closure are retained at `/var/lib/rom-studio-preview/rollback-rom-0.0.2-8652fcd2d296/`.
A ten-minute application rollback timer was armed before activation and canceled after checks passed.

The [desktop screenshot](evidence/rom-0.0.2/preview-activation-8652/studio-desktop.png)
and [mobile screenshot](evidence/rom-0.0.2/preview-activation-8652/studio-mobile.png)
show the accepted Studio after demo login.

These checks establish host-side VPN routing and release activation. They do not establish access from a remote Mac or iPhone.
The demonstration provider keeps grants and signing keys in memory. A provider restart requires a new login.
