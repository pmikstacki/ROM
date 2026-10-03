# Persistent development container

For the separately hosted Studio mock, see [Persistent ROM Studio preview](studio-preview.md).

The current development host runs the native NixOS container `rom-dev`, with autostart and persistent state. It shares the host network to support dependency downloads and mounts the host checkout at `/workspace/ROM`. No ROM server, broker, database, SSH service or public listener is configured.

`container.nix` declares its packages and user. The container retains the host's existing NixOS 24.11 package source and Node 22. Rust is independently pinned to **1.99.0** through a fixed, hash-checked rust-overlay revision. The default Rust profile includes matching Cargo, rustfmt, and Clippy. This development toolchain is not a declaration of ROM's minimum supported Rust version or supported production OS.

Rust 1.99.0 was verified against the [official stable channel manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml) on 2026-10-02 (release date 2026-10-01). The checked-in pin uses [rust-overlay commit 368fee9beaab04ca6fe7af28db63caa9badb22fa](https://github.com/oxalica/rust-overlay/tree/368fee9beaab04ca6fe7af28db63caa9badb22fa), including its component hashes. It explicitly selects `stable."1.99.0".default`. It does not follow a moving `latest` value. See the [overlay's toolchain documentation](https://github.com/oxalica/rust-overlay/tree/368fee9beaab04ca6fe7af28db63caa9badb22fa#cheat-sheet-common-usage-of-rust-bin).

## Use on the configured host

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM && ./scripts/check'
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM && ./scripts/build'
nixos-container root-login rom-dev
nixos-container status rom-dev
```

Build intentionally returns an explanatory error until a Cargo workspace exists. The mounted checkout is shared: edits inside it are edits to the host repository. Container root is for trusted development, not untrusted plugin isolation. Native Rust plugins are trusted code.

## Apply the pinned toolchain to the existing container

On the configured host, rebuild and switch only the container using its existing Nixpkgs source:

```sh
NIX_PATH=nixpkgs=/nix/store/6cxhnnjrig63fp8krlwm2h64kfbcnh0n-nixos-24.11.20250630.50ab793/nixos \
  nixos-container update rom-dev \
  --nixos-path /nix/store/6cxhnnjrig63fp8krlwm2h64kfbcnh0n-nixos-24.11.20250630.50ab793/nixos/nixos \
  --config-file /root/ROM/infra/nixos/container.nix

nixos-container run rom-dev -- sh -lc 'cargo --version && rustc --version && rustfmt --version && cargo clippy --version'
```

The store path identifies the existing Nixpkgs snapshot on this host. On another host, supply its deliberately selected source as in the recreation instructions below. The Rust overlay and release pins live in `container.nix`, but the complete OS package set is not independently locked by this module. `update` preserves the container root filesystem and current network, bind mount, and autostart configuration. No host OS rebuild is required. To upgrade Rust later, verify the official stable manifest. Update both the overlay revision/hash and the explicit Rust version. Rebuild the container. Rerun the checks.

## Recreate on a NixOS host

Use an available, deliberately selected Nixpkgs source. Adjust the checkout path. The module itself does not pin Nixpkgs.

```sh
NIX_PATH=nixpkgs=/path/to/nixpkgs nixos-container create rom-dev \
  --auto-start --nixos-path /path/to/nixpkgs/nixos \
  --config-file /absolute/path/to/ROM/infra/nixos/container.nix
```

Before you start the container, configure `/etc/nixos-containers/rom-dev.conf` for the intended network. The current host uses:

```ini
PRIVATE_NETWORK=0
HOST_ADDRESS=
LOCAL_ADDRESS=
HOST_BRIDGE=
HOST_PORT=
AUTO_START=1
EXTRA_NSPAWN_FLAGS="--bind=/absolute/path/to/ROM:/workspace/ROM"
```

On this host `/etc/systemd/system` is an immutable Nix-managed link. Establish the persistent startup dependency in the writable systemd control path. If the link already exists, skip its creation. Start the container:

```sh
mkdir -p /etc/systemd/system.control/multi-user.target.wants
ln -s /etc/systemd/system/container@.service \
  /etc/systemd/system.control/multi-user.target.wants/container@rom-dev.service
systemctl daemon-reload
nixos-container start rom-dev
```

Container state is stored by NixOS under `/var/lib/nixos-containers/rom-dev`. Container stops and restarts preserve the state. Container destruction does not preserve it. The startup dependency lives outside the immutable system unit tree and was verified with `WantedBy=multi-user.target`. A future declarative host module can own it.

Verified during bootstrap: tool versions, outbound HTTPS, local OpenSpec checks inside the container, and a marker file surviving container restart. Host reboot was not performed.

Verified after the 2026-10-02 toolchain update:

- `cargo 1.99.0 (5f94df478 2026-08-27)` and `rustc 1.99.0 (b940084d7 2026-09-28)` resolve through `/run/current-system/sw/bin` inside the container.
- `rustfmt 1.10.0-stable` and `clippy 0.1.99` report the matching `b940084d7e` compiler revision.
- A fresh, dependency-free edition-2024 binary passed `cargo check --offline`, `cargo fmt --check`, `cargo clippy --offline -- -D warnings`, and `cargo run --offline`.
- The existing container remains active; `PRIVATE_NETWORK=0`, the checkout bind mount, `AUTO_START=1`, and systemd's `WantedBy=multi-user.target` remain in place. This update used a configuration reload, with no container destruction or host reboot.

Reference: [NixOS container management](https://nixos.org/manual/nixos/stable/#ch-containers).
