# Protected Studio service files

These files prepare the real ROM 0.0.2 preview. They do not activate it.
The existing static mock remains in service until the new artifact passes acceptance.

## Artifact layout

The immutable release directory must contain these files:

```text
bin/rom-demo
studio/dist/index.html
studio/dist/assets/...
provider/preview-provider.mjs
provider/https-provider.mjs
provider/human-server.mjs
provider/human-interaction.mjs
provider/package.json
provider/package-lock.json
provider/node_modules/...
```

Bind the accepted directory read-only at `/opt/rom-studio`.
Record hashes for the binary, assets, fixture sources, lockfile, and installed dependency tree.
Do not serve files from the worktree or Cargo target directory.

The prepared container uses UID and GID `44173` for the services.
Create `/var/lib/rom-studio-preview/data` with that ownership and mode `0700`.
Keep Resource data and attachments in this directory.

## External configuration

Copy the example JSON files into `/var/lib/rom-studio-preview/config` as `studio.json` and `provider.json`.
These files contain trusted URLs and credential paths. They contain no secret values.
Generate the client secret outside Git. Store it at `/var/lib/rom-studio-preview/private/client-secret`, owned by root, with mode `0600`.
Both services load the same secret through systemd credentials.

Import the prepared module with the accepted release directory and this argument list:

```nix
import ./host-module.nix {
  releaseDirectory = "/var/lib/rom-studio-preview/releases/0.0.2-ACCEPTED-HASH";
  studioArgs = [
    "studio-profile" "sqlite" "/var/lib/rom-studio/preview.sqlite" "44173"
    "/opt/rom-studio/studio/dist" "/run/rom-studio-config/studio.json"
  ];
}
```

The profile command is supplied by the demo launcher. Its separate acceptance must pass before deployment.
The provider listens on `127.0.0.1:44174`. Studio listens on `127.0.0.1:44173`.
The gateway preserves both public mount paths.

## Acceptance limits

Build and validate the NixOS configuration before activation.
Keep the old module and previous system closure for rollback.
Follow `/etc/nixos/ai-server/AGENTS.md` before changing gateway access.
Preserve LAN SSH, WireGuard, DNS, other services, and Windows disks.

Verify the actual HTTPS issuer and callback with certificate checks enabled.
Verify current authentication, generic mutations, live snapshots, and attachments through Caddy.
Restart the service and container. Verify retained Resource data and new login after session loss.

The demonstration provider stores grants and signing keys in memory.
A provider restart requires new authentication. It does not create production account authentication.
The login page explicitly identifies demonstration accounts.

The 45-second service stop limit bounds shutdown. Forced termination can leave a durable operation for recovery.
It does not cancel already committed Resource changes.
MacBook VPN access and trust in Caddy's internal CA require separate client-side verification.
