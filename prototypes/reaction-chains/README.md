# Disposable after-commit reaction-chain simulation

This deterministic Rust program compares ten protection policies across eleven scenarios, with nineteen assertions and negative controls. It has no third-party dependencies. Resource is the sole domain entity; events, delivery keys, budgets and queue records are supporting protocol/runtime values.

From the host:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/reaction-chains && ./prototypes/reaction-chains/verify.sh'
```

With Rust 1.99.0 locally, run `./prototypes/reaction-chains/verify.sh` in this worktree. The verifier checks formatting, locked tests, Clippy with warnings denied and an exact regeneration comparison against [results.csv](results.csv). Builds use two jobs and this prototype's target directory. To inspect generated CSV, run `cargo run --locked --quiet` in this directory.

**All database calls and durability are modeled.** No SQL is executed, no real process is crashed, and no throughput/latency benchmark is claimed. A modeled checkpoint keeps resource state, events, action receipts and remaining work together; restart discards the ready queue and redelivers unacknowledged work. The result checks consequences of those assumptions, not that a production adapter implements them.

The CSV contains 110 policy/scenario rows plus the header. Projection coalescing, out-of-order recovery, incorrect dependency declarations and finite monotone closure have additional dedicated assertions in `src/lib.rs`. Counts include the initial root action. `modeled_db_roundtrips` sums one charge per receipt lookup, resource load and transaction; metadata scans, queue/checkpoint operations and network/disk costs are excluded. `redundant_modeled_roundtrips` counts those charges on equal-state/replayed operations; it does not mean all of those reads are safely removable. `watchdog=true` means the test stopped with pending work after 128 delivery steps, **not convergence**.

See [the assessment](../../docs/research/reaction-chain-results.md) for policy choices, concrete counterexamples, source comparisons and remaining integration tests.
