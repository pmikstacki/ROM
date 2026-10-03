# Reproducing immutable experiments

Agent worktrees are reused for maintained integration. An old report's worktree path records the location of its original run. The path does not necessarily still contain that experiment. Published prototype commits remain the source of truth.

Run from a full ROM clone with the documented Rust 1.99 toolchain:

```sh
./scripts/run-prototype chains
./scripts/run-prototype configuration
./scripts/run-prototype transport
./scripts/run-prototype auth
```

On this NixOS host, prefix the chosen command with:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && CARGO_NET_OFFLINE=true ./scripts/run-prototype chains'
```

The script exports the exact source commit into a fresh temporary directory. It runs that commit's verifier and retains its sources/results for inspection.
It never checks out or resets an agent's working branch. Cargo offline requires
the dependencies already cached; OpenSSL CLI is also needed by the auth fixture.
The coordinator executed the new wrapper for chains: 19 tests and exact CSV
regeneration passed. Other unchanged verifier commands refer to the independently
verified commits recorded in their respective reports.

| Name | Published source | Assessment |
| --- | --- | --- |
| `chains` | `c95a2a2d` | [Simulation results and limitations](reaction-chain-results.md) |
| `configuration` | `66854f5708` | [24-test comparison](configuration-trial-results.md) |
| `transport` | `789201201c6388a44d9d33fe2045d0dd075f33cc` | [15-test comparison](transport-trial-results.md) |
| `auth` | `58d4c9deb` | [Disposable profile probe](auth-provider-probe-results.md) |

These preserve historical experiments, including limitations superseded by
maintained fixes. To test the current library use `./scripts/check`, not an old
prototype verifier. In particular, the maintained auth crate has stricter claim
validation and a different cryptographic backend than the disposable probe.
