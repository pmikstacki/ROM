# Persistent ROM Studio preview

The NixOS host runs the native `rom-studio` container independently of Codex and
shell sessions. It starts at boot and serves the published HTML mock with nginx.
This is a design preview with synthetic data, not a running ROM backend. Changes
made inside the mock reset on page reload.

## Open from a MacBook

Connect to the existing home WireGuard VPN, then open either:

- <https://10.66.0.2/rom-studio/>
- <https://bdziam.home.arpa/rom-studio/> with the existing VPN DNS configuration.

These reuse the existing Caddy gateway and its internal CA. The Mac must trust
that gateway's CA. No SSH port forwarding, public deployment or new firewall
opening is involved. The page is available while the host and VPN are available;
this is not a promise of availability during host shutdown or network outages.

## Configuration and deployment

`studio-container.nix` defines nginx on host loopback port 44173.
`studio-host.nix` defines the container, its read-only content mount, autostart and
the two gateway routes. The module is specific to the existing bdziam gateway;
another host must adjust its virtual-host names and existing TLS configuration.

The served content lives at `/var/lib/rom-studio-preview/site/index.html`, outside
the checkout and prototype worktree. Only this directory is mounted in the
container. Publishing uses an atomic rename, so readers get the old or new file.

The source and rendered standalone document are on the
[`codex/prototype-studio-mock` branch](https://github.com/pmikstacki/ROM/tree/codex/prototype-studio-mock/prototypes/studio-mock).
After rendering a new standalone document, publish from the host:

```sh
sudo ./scripts/publish-studio-preview /absolute/path/to/index.html
```

Publishing does not restart the container. The response disables caching so a
normal reload picks up the published mock. The publisher accepts trusted local
preview files; it is not an upload endpoint or HTML sanitization service.

On bdziam, the two modules are installed beside each other under
`/etc/nixos/ai-server/modules/`, with `studio-host.nix` named `rom-studio.nix`, and
the host imports the latter. Update both copies deliberately when modifying the
repository modules. Build and inspect the host configuration before activation;
follow the host's existing rollback and networking requirements. This deployment
changed no firewall, WireGuard, DNS or SSH settings.

## Operations and verification

```sh
systemctl status container@rom-studio
systemctl restart container@rom-studio
journalctl -u container@rom-studio
```

Verified on 2026-10-02:

- The NixOS configuration built; Caddy validated the combined configuration.
- Container and nginx started; its listener is only `127.0.0.1:44173`.
- Both HTTPS routes returned 200, with TLS checked against the existing CA.
- The served document matched the published file byte for byte.
- Chromium loaded the mock through HTTPS without page-script errors. Browser
  rendering used a test certificate override; CA validation was tested separately
  with curl, not inferred from that override.
- A container restart preserved the file and restored the HTTPS response.
- The container is enabled at boot and the host system profile/boot entry include
  it. No full host reboot or MacBook-side connectivity check was performed.
- The previous temporary preview service was stopped.

Host activation re-reported a pre-existing `beskid-build-image.service` failure:
its rootless build lacks `newuidmap` on PATH. Logs show the same error before this
deployment. It is unrelated to Studio and was not repaired as part of preview
hosting. Existing gateway routing still responded after this change.
