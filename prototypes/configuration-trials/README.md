# Disposable configuration crate trial

**Throwaway experiment, not ROM production code or an adopted API.** Compares config-rs 0.15.27 and Figment 0.10.19 behind one compiled Resource registry. `User`, `AppSettings`, and `IdentityProvider` use the same validator and import/action path. The test-only host grant table is a policy stub, not authentication.

From the host, run:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/configuration-trials/prototypes/configuration-trials && ./verify'
```

Inside a Rust 1.99 environment, `./verify` is sufficient. It runs formatting, Clippy with warnings denied, and the locked tests with two build jobs and this prototype's own target directory. After dependencies are fetched, `CARGO_NET_OFFLINE=true ./verify` repeats offline. Cargo.lock captures all selected dependency versions; no Rust toolchain is installed by this script.

`tests/loader_behavior.rs` observes actual library semantics. `tests/resource_wrapper.rs` checks ROM-owned policy and publication logic using both loaders. Tests create/remove uniquely named files in the container's temporary directory; all managed Resource state remains in memory. They do not contact a live identity provider, database or secret manager. A pair of channel-gated worker threads models remote completions deterministically, without sleeps or a network dependency.

[Results, recommendations and remaining decisions](../../docs/research/configuration-trial-results.md).
