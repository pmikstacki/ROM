# Disposable transport trials

Question: should ROM's semantic boundary be a Tower service stack, focused Rust methods, or focused methods with optional Tower adaptation?

Run in the existing development container from any host directory:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/transport-trials && ./prototypes/transport-trials/verify.sh'
```

With Rust 1.99.0 available locally, run `./prototypes/transport-trials/verify.sh` from this worktree. The verifier runs formatting, locked tests and Clippy with warnings denied; two compiler jobs and a prototype-specific target directory are enforced. First use downloads the locked dependencies. No server, database, broker or network listener is started.

The executable artifact is the Rust test binary, deliberately driven with deterministic gates and assertions. Fifteen tests include intentional negative controls that **pass when they detect** inadequate lifetime accounting. Tower 0.5.3, Tokio 1.53.1, tokio-util 0.7.19 and Rayon 1.12.0 were current registry versions when checked on 2026-10-02; these are experimental pins.

`Core::invoke`, `Core::observe` and `Core::subscribe` remain distinct. `TowerInvoke` is optional composition over `invoke`; Tower is not given authority, durability or work ownership. One in-memory `Resource` is the only domain entity. Commands, contexts, receipts, gates and cursors are supporting values.

This is not production ROM. Admission is volatile; outcome replay and journal history do not survive process restart. A fixed integer command represents semantic input; `bytes` is a supplied accounting charge, **not measured heap usage or a wire-decoder limit**. The outcome table stops admitting distinct keys at 16 rather than silently evicting identity. Runtime shutdown, supervisor panics, multiple writers, real codecs and remote effects remain outside the probe.

See [the full assessment](../../docs/research/transport-trial-results.md) for the observed comparisons, queue inventory, primary sources, recommendation and remaining decisions.
