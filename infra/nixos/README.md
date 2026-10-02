# Persistent development container

The current development host runs the native NixOS container `rom-dev`, with autostart and persistent state. It shares the host network to support dependency downloads and mounts the host checkout at `/workspace/ROM`. No ROM server, broker, database, SSH service or public listener is configured.

`container.nix` declares its packages and user. The initial environment uses the host's existing NixOS 24.11 package source (Rust 1.82, Node 22); this is a bootstrap environment, not a chosen ROM minimum Rust version or supported production OS. Select and pin an updated toolchain before implementing the Rust workspace.

## Use on the configured host

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM && ./scripts/check'
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM && ./scripts/build'
nixos-container root-login rom-dev
nixos-container status rom-dev
```

Build intentionally returns an explanatory error until a Cargo workspace exists. The mounted checkout is shared: edits inside it are edits to the host repository. Container root is for trusted development, not untrusted plugin isolation. Native Rust plugins are trusted code.

## Recreate on a NixOS host

Use an available, deliberately selected Nixpkgs source and adjust the checkout path. The module itself does not pin Nixpkgs.

```sh
NIX_PATH=nixpkgs=/path/to/nixpkgs nixos-container create rom-dev \
  --auto-start --nixos-path /path/to/nixpkgs/nixos \
  --config-file /absolute/path/to/ROM/infra/nixos/container.nix
```

Before starting, configure `/etc/nixos-containers/rom-dev.conf` for the intended network. The current host uses:

```ini
PRIVATE_NETWORK=0
HOST_ADDRESS=
LOCAL_ADDRESS=
HOST_BRIDGE=
HOST_PORT=
AUTO_START=1
EXTRA_NSPAWN_FLAGS="--bind=/absolute/path/to/ROM:/workspace/ROM"
```

Then establish the persistent systemd startup dependency and start it:

```sh
systemctl add-wants multi-user.target container@rom-dev.service
systemctl daemon-reload
nixos-container start rom-dev
```

Container state is stored by NixOS under `/var/lib/nixos-containers/rom-dev`. Stopping or restarting preserves it; destroying a container does not. Host rebuilds that regenerate service links should retain or reapply the startup dependency; a future declarative host module can own it.

Verified during bootstrap: tool versions, outbound HTTPS, local OpenSpec checks inside the container, and a marker file surviving container restart. Host reboot was not performed.

Reference: [NixOS container management](https://nixos.org/manual/nixos/stable/#ch-containers).
