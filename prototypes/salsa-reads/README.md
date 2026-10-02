# Throwaway Salsa derived-read probe

Question: can ROM reuse a pure read of the existing Task resource while correctly invalidating it after resource or policy changes?

**Observed answer: yes for this explicitly modeled, in-process snapshot projection.** Eight tests cover memo reuse, field precision, changes to list membership, actor/policy invalidation, resource insertion/deletion/reinsertion, revision replay rejection, equal-result propagation, and a deliberately incorrect untracked-policy control. This does not prove a faster end-to-end service or justify replacing ROM's resource/storage authority.

Run from this directory:

```sh
cargo run --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

On the current host, invoke commands through the native NixOS development container:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/salsa-reads/prototypes/salsa-reads && cargo run --locked'
```

`Snapshot` represents one already committed resource and policy view. `plain_open_ids` recomputes directly from it. `RegisteredTaskReads::apply` projects the same snapshot into private Salsa inputs; `open_ids` and `open_count` expose ordinary owned Rust results. The name describes this concrete Task read adapter: there is no generic ROM registration implementation in this probe, and this is not a proposed author-facing Salsa API.

The read is deliberately small: list the IDs of incomplete tasks owned by the enabled actor, and count them. Title is a separate stored input field but is not read. Each task has separately tracked owner/done/title fields; the catalog tracks collection membership; actor identity and enablement are explicit tracked inputs. The title result therefore applies to ID/count reads, not a list that returns titles or a whole-record input model.

`wrong_open_ids` is a deterministic, test-only-used negative control: it reads an external atomic permission flag without declaring a Salsa dependency. The test passes when it reproduces the **incorrect** stale authorized result after external revocation. Do not copy this function into production. Mainline reads use `ActorPolicy` instead.

Resource writes, commits, durable events, action execution, idempotency and effect delivery are outside Salsa. This code performs no I/O or durable effects in tracked queries. Body counters are observation-only test instrumentation and never influence results. It retains only in-memory data; dropping the adapter loses all its state and the source snapshot can rebuild it.

See [ASSESSMENT.md](ASSESSMENT.md) for observations, bookkeeping and boundaries, and [results/checks.txt](results/checks.txt) for executed checks and demo output. Dependency resolution is pinned by both exact Salsa version and Cargo.lock. No throughput/latency claims or Moka workload comparisons are made.
