# Scoped Studio preview activation design

This procedure is prepared, not executed. The coordinator must supply the final accepted artifact identity first.
It updates the application container and gateway configuration without switching the host system or changing host network settings.

## Inspected mechanics

The parent unit is `/etc/systemd/system/container@rom-studio.service`.
Its `EnvironmentFile` reads `/etc/nixos-containers/rom-studio.conf`.
That file pins `SYSTEM_PATH` to the old static-preview closure.
The parent `ExecReload` invokes that selected closure's `switch-to-configuration test` inside the container.
Its `ExecStart` also uses the selected closure for container initialization.

`nixos-container update` sets `/nix/var/nix/profiles/per-container/rom-studio/system` and reloads the parent unit.
It does not replace the pinned `SYSTEM_PATH` in the existing declarative environment file.
A plain update therefore does not establish that this container will run or restart into the new configuration.
The current per-container profile directory has no system selector.

Use a persistent, application-only parent-unit drop-in to select the exact accepted `/nix/store` closure.
Keep all existing bind and network flags. The override contains only `SYSTEM_PATH`.
Also set the per-container profile for retained identity and garbage-collection protection.
The profile directory is remapped inside the container; its host-side absolute profile path is not a valid shared `SYSTEM_PATH`.
An exact store closure works in both host and container namespaces.

The existing parent unit binds `/var/lib/rom-studio-preview/site` read-only at `/srv/studio`.
The candidate `existing-container.nix` uses `/srv/studio/releases/RELEASE_NAME` as the accepted artifact.
It creates a root-owned `/opt/rom-studio` link to that exact version.
Data, configuration, and private credentials remain in the retained container root, outside the artifact.
No additional parent bind mount or network flag is required.

## Evaluation evidence

`task4/scoped-container-evaluation.json` records equal prior/candidate hashes for the selected container networking boundary.
`task4/scoped-network-units-final.json` records equal script/serviceConfig hashes for:
`dhcpcd`, `firewall`, `network-local-commands`, and `network-setup`.
The evaluated candidate has the same network-related service names as the existing container configuration.
Its only new processes are the application host and demonstration provider.
Both prepared module files pass syntax parsing.
These checks are evaluation evidence, not an activated-service result.

## Accepted inputs and preparation

Use an accepted source checkout for `existing-container.nix` and `service-container.nix`.
Record the locked nixpkgs input, artifact directory name, binary hash, complete asset inventory, provider dependency identity, and candidate container closure.

Build only the container closure. For example, use this expression with the final release name and accepted source path:

```nix
let
  flake = builtins.getFlake "/etc/nixos/ai-server";
  container = flake.inputs.nixpkgs.lib.nixosSystem {
    system = "x86_64-linux";
    modules = [
      (import /ABSOLUTE_ACCEPTED_SOURCE/deployment/studio-preview/existing-container.nix {
        releaseName = "ACCEPTED_RELEASE_NAME";
        studioArgs = [
          "studio-profile" "sqlite" "/var/lib/rom-studio/preview.sqlite" "44173"
          "/opt/rom-studio/studio/dist" "/run/rom-studio-config/studio.json"
        ];
      })
    ];
  };
in container.config.system.build.toplevel
```

```sh
nix --extra-experimental-features 'nix-command flakes' build --impure --file CONTAINER_EXPRESSION --out-link CONTAINER_CANDIDATE_LINK
```

This command does not activate the host or container.
Install the immutable accepted artifact under `/var/lib/rom-studio-preview/site/releases/ACCEPTED_RELEASE_NAME`.
Keep the old mock files at the site's root for rollback.
Make the accepted artifact root-owned and non-writable by the service.

The retained container root is `/var/lib/nixos-containers/rom-studio`.
Create these separate directories there before activation:

| Container path | Required ownership and mode | Use |
| --- | --- | --- |
| `/var/lib/rom-studio` | UID/GID `44173`, `0700` | Resource database and attachments |
| `/var/lib/rom-studio-config` | root-owned; readable configuration files | `studio.json` and `provider.json` |
| `/var/lib/rom-studio-private` | root-owned, `0700` | External `client-secret`, mode `0600` |

The candidate tmpfiles rules recreate `/run/rom-studio-config` and `/run/rom-studio-private` links at container startup.
The service credential paths remain the same as the prepared examples.
Systemd loads the external secret before dropping to the service UID.
The secret must never enter Git, the accepted artifact, the Nix store, command arguments, or readiness logs.

## Scoped parent activation

Retain the old container closure, original application drop-ins if present, and current gateway configuration.
Do not overwrite unrelated overrides.
The existing Caddy directory includes a generated `overrides.conf`; application overrides must sort after it, for example `zz-rom-studio-preview.conf`.

The prepared container drop-in is `scoped-container-profile.conf`.
Install it as `/etc/systemd/system/container@rom-studio.service.d/zz-rom-studio-profile.conf`.
Install `container-system.example.env` as `/var/lib/rom-studio-preview/container-system.env`.
Generate its only value from the canonical accepted container closure:

```text
SYSTEM_PATH=/nix/store/EXACT_ACCEPTED_CONTAINER_CLOSURE
```

Do not install the placeholder. Resolve the candidate link and record the exact store path before writing the file.

Before these commands, resolve both recorded container closure variables to canonical `/nix/store` paths.
Check that each required `init` and `bin/switch-to-configuration` executable exists.
After all accepted files and private configuration are ready:

```sh
nix-env -p /nix/var/nix/profiles/per-container/rom-studio/system --set "$CANDIDATE_CONTAINER_SYSTEM"
printf 'SYSTEM_PATH=%s\n' "$CANDIDATE_CONTAINER_SYSTEM" > /var/lib/rom-studio-preview/container-system.env.new
chmod 0600 /var/lib/rom-studio-preview/container-system.env.new
mv -fT /var/lib/rom-studio-preview/container-system.env.new /var/lib/rom-studio-preview/container-system.env
systemctl daemon-reload
systemctl reload container@rom-studio.service
```

Reload applies the selected container configuration through the existing parent mechanism.
No whole-host `switch-to-configuration` or `nixos-rebuild switch` command is part of this procedure.
Verify that nginx is stopped and the two application services are active before gateway cutover.
Verify the new loopback listeners and actual `/rom-studio/` root route.
A later `nixos-container restart rom-studio` must select the same candidate closure and preserve data.

## Scoped gateway activation

Retain a protected copy of the complete current Caddyfile.
Prepare a candidate from that copy; do not replace it with a minimal file that drops other projects.
Replace only `handle_path /rom-studio/*` with the prefix-preserving Studio route.
Add the approved `/rom-studio-provider/*` route.
Retain listener addresses, internal TLS, logging, unrelated routes, and the fallback service.
Verify that the current and candidate adapted JSON differ only in these application route subtrees.

Use the existing installed Caddy binary, currently:
`/nix/store/n44y8dmc96wcipdm9ckqfid1vgl5j38m-caddy-2.8.4/bin/caddy`.
Resolve and record the actual binary again at activation time.
The protected active candidate can be stored at `/var/lib/rom-studio-preview/caddy/active.caddyfile`.
Make it readable by the existing Caddy service without exposing private configuration to other users.

Validate the complete candidate before loading it:

```sh
"$CADDY_BINARY" adapt --config "$CADDY_CANDIDATE" --adapter caddyfile
"$CADDY_BINARY" validate --config "$CADDY_CANDIDATE" --adapter caddyfile
```

Retain adaptation output in a protected file; do not dump unrelated credential-bearing configuration to the chat.
Run validation with the existing Caddy storage/home context so it uses the existing CA and does not create a replacement CA.

Persist restart behavior with an application drop-in that clears and replaces `ExecStart` and `ExecReload`.
Retain the existing binary, service user, environment, and other service properties.
Point only the two commands at the protected active Caddyfile:

```ini
[Service]
ExecStart=
ExecStart=CADDY_BINARY run --config /var/lib/rom-studio-preview/caddy/active.caddyfile --adapter caddyfile
ExecReload=
ExecReload=CADDY_BINARY reload --config /var/lib/rom-studio-preview/caddy/active.caddyfile --adapter caddyfile --force
```

`CADDY_BINARY` is a placeholder for the recorded absolute installed binary, not an environment variable in the installed unit.
Place this application drop-in after the generated `overrides.conf` in lexical order.
Then perform only:

```sh
systemctl daemon-reload
systemctl reload caddy.service
```

The existing Caddy process receives the candidate routes. No Caddy restart or host network activation is needed for this change.
The persisted drop-in supplies the same candidate after a service restart or normal host boot.

## Application-only rollback

Prepare a local rollback script before activation. It must not call the whole-host switch helper or restart WireGuard.
The script must use recorded absolute paths and retained protected snapshots, rather than browser input or untrusted shell text.

Restore the old container closure through the same per-container profile:

```sh
nix-env -p /nix/var/nix/profiles/per-container/rom-studio/system --set "$OLD_CONTAINER_SYSTEM"
printf 'SYSTEM_PATH=%s\n' "$OLD_CONTAINER_SYSTEM" > /var/lib/rom-studio-preview/container-system.env.new
chmod 0600 /var/lib/rom-studio-preview/container-system.env.new
mv -fT /var/lib/rom-studio-preview/container-system.env.new /var/lib/rom-studio-preview/container-system.env
systemctl reload-or-restart container@rom-studio.service
```

Restore the retained original complete Caddyfile into the active application configuration and reload Caddy.
The parent application override can remain: its restored exact store path selects the old closure and preserves restart behavior.
Keep new Resource data, attachments, accepted artifacts, and failed-attempt evidence; rollback must not delete them.
If drop-ins existed before this attempt, restore their exact prior contents instead of deleting unrelated overrides.

An optional transient application rollback timer can run this prepared script, for example after ten minutes.
Cancel that timer after the local application acceptance succeeds.
This timer protects application cutover. It is not a substitute for the independent access checks required for actual networking changes.

## Future declarative maintenance

`host-existing-bind.nix` is the declarative counterpart. It preserves the original read-only parent mount and names the accepted inner configuration.
Maintain that exact release identity in the host source without activating a whole-host switch during this application cutover.
The exact parent closure selector and Caddy application drop-ins are explicit persistent service configuration, not a temporary shell environment.

At a later approved host-generation integration, remove the scoped drop-ins only after the generated container closure and Caddy routes supply the same accepted application behavior.
Do not remove them during a reboot-only test or leave the generated old mock closure as the restart selector.
Record that transition so future Caddy changes from other projects do not remain hidden behind a stale application snapshot.

## Acceptance boundary

Before and after activation, confirm unchanged host network/SSH settings and unrelated gateway behavior.
Verify HTTPS with the existing public CA and certificate checks enabled.
Run actual login, two generic Resources, live queries, attachments, logout, service restart, and container restart.
Remote MacBook reachability remains a separate unverified result until independently observed.
No activation or final-artifact acceptance is claimed by this design.
