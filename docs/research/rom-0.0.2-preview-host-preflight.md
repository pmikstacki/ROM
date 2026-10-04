# Protected preview host preflight

This is a read-only inspection on 2026-10-04.
No service, firewall, interface, WireGuard, DNS, or host configuration changed.
No private key, client-secret value, or password hash was read or printed.

## Required access checks

`/etc/nixos/ai-server/AGENTS.md` requires key-only SSH on both `enp8s0` and `wlp7s0`, independently of WireGuard.
It forbids SSH for the console-only `local_root` account.
Before networking activation, it requires a timed rollback and fresh Ethernet, Wi-Fi, and VPN SSH sessions.
Rollback can be cancelled only after independent access checks succeed.
Windows disks, unrelated services, and root-only secrets must remain intact.

Current effective SSH configuration has listeners on IPv4 and IPv6 port 22.
Password and keyboard-interactive authentication are disabled. Root permits key authentication; `local_root` is denied.
This configuration inspection does not prove independent remote ingress.

## Current service and gateway wiring

- `/etc/nixos/ai-server/hosts/bdziam/default.nix` imports `modules/rom-studio.nix`.
- `modules/rom-studio.nix` starts the shared-network, autostart `rom-studio` container.
- Its read-only `/srv/studio` mount uses `/var/lib/rom-studio-preview/site`.
- `modules/studio-container.nix` starts nginx on `127.0.0.1:44173`.
- Both existing gateway hosts use `handle_path /rom-studio/*`, which removes the required Rust-host base path.
- `modules/local-chat.nix` and `modules/vpn-dns.nix` retain the VPN-bound `10.66.0.2` HTTPS listeners and internal TLS.
- The gateway falls back to the unrelated service on `127.0.0.1:3210`.

The container and Caddy services are active.
Port 443 listens only on `10.66.0.2`; nginx listens on `127.0.0.1:44173`.
Port 44174 has no listener yet.

The copied checkout at `/root/ai-server` does not contain the two ROM preview module files.
Do not replace `/etc/nixos/ai-server` from that older checkout or assume that their trees match.
Only the exact preview modules and accepted deployment files should change at cutover.

## Available autonomous probes

The configured external SSH host is `bdziam.dev`, user `root`, using `/root/.ssh/id_ed25519_bdziam` with `IdentitiesOnly yes`.
The configured host key exists in `/root/.ssh/known_hosts`.
Two fresh, read-only probes used BatchMode, strict host-key checking, a five-second connection deadline, and disabled connection multiplexing.
Both reached authentication and failed with `Permission denied (publickey,password)`.
No alternative account was guessed and no remote authorized keys were changed.

Historical files describe `ssh -J root@bdziam.dev root@10.66.0.2` as the Mac/VPS probe route.
The jump route cannot currently authenticate with the configured local credential.
The setup script is `/root/ai-server/scripts/setup-mac-wireguard.py`; running it would modify peer configuration and is outside this preflight.

The current addresses are Ethernet `192.168.76.250`, Wi-Fi `192.168.76.225`, and WireGuard `10.66.0.2`.
All three local route probes select `lo`.
Connecting from this host to those addresses cannot prove independent Ethernet, Wi-Fi, or VPN ingress.
No existing authorized external LAN probe client was found in the inspected scripts or SSH configuration.

The Playwright tool identifies its browser as Linux x86_64 headless Chromium, not the owner's MacBook.
It can support server-side browser acceptance. It cannot establish MacBook routing or independently verify the required LAN SSH paths.

The public certificate at `/root/ai-server/docs/bdziam-local-ca.crt` matches the active Caddy CA certificate file at:
`/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt`.
Their SHA-256 file digests match.
A local GET of `https://10.66.0.2/rom-studio/` with `--cacert` returned HTTP 200.
This verified certificate checking for the existing static mock, not new Studio login or remote reachability.

## Prior rollback procedure

The active system closure is:
`/nix/store/vfgnvw7w6cdl2whamr87xagbjvvy0h7i-nixos-system-bdziam-24.11.20250630.50ab793`.

The journal records a prior `rom-studio-rollback.timer` on 2026-10-02.
Its service command invoked the prior system's `bin/switch-to-configuration test`.
The timer was deactivated after the earlier cutover. Its transient unit file no longer exists.
No rollback, restore, or network-probe timer is currently armed.
The exact old timeout and full transient unit definition were not recovered.
Do not claim that the prior timer currently protects a new activation.

`/root/ai-server/docs/wifi-verification.md` records a previous generation rollback and separate Ethernet-restore timer.
`/root/ai-server/scripts/wifi-generation-test.sh` is the detached generation helper used in that earlier network test.
It explicitly alters a route and restarts Wi-Fi/WireGuard.
Do not reuse that helper for an application-only Studio cutover.
Its historical “VPN-only SSH” guidance is superseded by the current host AGENTS requirement to preserve LAN SSH.

## Minimal cutover candidate

Use the accepted artifact identity supplied by the coordinator.
Keep credentials and mutable Resource/blob data outside the immutable release directory.
Replace the old nginx container configuration with the prepared Rust/Node services.
Replace only the preview gateway paths with prefix-preserving `handle` routes.
Retain the existing gateway TLS, listener addresses, fallback application, SSH, firewall, DNS, and WireGuard settings.
Use one canonical public origin consistently across Caddy and both trusted profiles.

A whole-system NixOS switch can activate network setup even when the intended change concerns only the application.
The historical Wi-Fi report records a VPN interruption during such activation.
If the coordinator chooses whole-system activation, the required independent checks remain unresolved and must not be replaced by self-probes.
A deliberately scoped application-service/container and Caddy reload avoids an intentional interface or WireGuard change, but requires its own reviewed rollback procedure.
This report does not classify that operation as satisfying the independent networking-activation checks.

Build the candidate without activation first. Retain the current closure and exact old module files for rollback.
Do not overwrite unrelated host configuration from the older checkout.

## Unresolved independent verification

Fresh external Ethernet, Wi-Fi, and VPN SSH verification cannot currently be established autonomously through the inspected mechanisms.
The configured external SSH credential is refused, and local/self probes do not test those paths.
MacBook VPN routing, DNS, CA trust, and browser login remain separately unresolved.
These are execution limits, not requests for new owner approval or evidence that the source release failed.

## Coordinator's application-only boundary

The coordinator confirmed the current AGENTS condition applies before activating networking changes.
An isolated application container update and Caddy application-path replacement can proceed within the existing deployment authorization when network settings remain identical.
Missing MacBook access is not a source-release or application-deployment blocker under that boundary.
It remains an explicitly unverified client result.
The failed external SSH probes remain evidence; they must not be rewritten as passing ingress checks.

The cutover must avoid a whole-system switch that restarts network setup.
It must not alter SSH, interface addresses, Wi-Fi, routes, firewall rules, WireGuard peers, DNS, or the existing HTTPS listener/TLS configuration.
Only the accepted application processes, their private mounts/data, and the two application paths may change.
The runtime gateway must retain its unrelated fallback routes.

Before a scoped cutover, compare the current and candidate evaluated configuration for these boundaries:

- `services.openssh`, including enabled listeners, port, key-only settings, and `DenyUsers`.
- Networking interfaces, route/default-gateway/DHCP settings, wireless configuration, and network-manager/networkd options.
- Firewall/nftables settings, WireGuard/wg-quick settings, and DNS/nameserver settings.
- Existing host Caddy bind addresses, TLS ownership, and unrelated routes.
- Network-related systemd service definitions and running states.

Compare complete settings in memory or record only their hashes. Do not print credentials or private-key values.
Do not use `wg show ... dump`, which can include private key material.
Retain a diff showing only the intended container/service and gateway-path changes.
If any protected boundary changes, stop application-only classification and apply the host's networking-activation rules.

At runtime, record the same protected boundary checks before and after scoped service activation.
A service restart and data-recovery test must not restart the host network or Windows guest.
The exact accepted artifact identity, scoped activation command, rollback command, and gateway configuration backup remain coordinator-owned inputs.
No candidate activation command was run during this preflight.
