# Independent compensation-demo review

Date: 2026-10-03. Reviewed commit
`7526c82768789225969683f9bcddf909ff8f8cd5`, compared with
`fae5508b1753e18c01d75042a2c5d177bd730005`, in the isolated
`compensation-demo` worktree. The tracked checkout was clean. Cargo.lock SHA-256:
`35a128d90a9c31bc03f0e073cdba52e819bffb565bfb05cfd34a8261bf112e7d`.

**Result: no outstanding P1/P2 finding in the agreed demo scope.** The implementation
uses ordinary Resource declarations, actions and a Reaction. It demonstrates an
explicit application-authored compensation after a simulated confirmed payment
rejection. It adds no generic terminal-worker-failure hook, automatic rollback,
payment-provider integration, privileged retry route or new execution engine.

## Source and contract assessment

`demo/src/compensation.rs` keeps business behavior in the demo module. Bootstrap
persists Checkout's target/token before the forward reservation. `record-payment`
accepts explicit outcomes and rejects contradictory transitions after a terminal
outcome. The mapper emits at most one token-specific release, only for
`confirmed_rejected`; unknown/transient/success outcomes do not select release.

Release removes one map entry and leaves total stock and other tokens unchanged.
It does not restore a stale aggregate snapshot. Existing core materialization
binds the target revision and maps a conflicting target command to stopped work,
so a later stock change is not silently overwritten. Current service authority
remains enforced by the existing runtime, rather than a demo-specific bypass.
Original command identity uses the existing receipt mechanism.

The README explicitly limits this to a synthetic shared workshop: its session
can directly mutate ordinary Resources, and reservation tokens must not be
recycled. The declarations are not an adversarial checkout/payment policy or a
mandatory cross-resource state machine. These limits are appropriate to the
agreed scope and should remain visible if this example is reused.

The tests distinguish forward commits from subsequent recovery, preserve another
reservation, count effects, reject contradictory terminal outcomes, exercise
service revocation, and reopen both real database adapters. The code fits the
repository's public-API example model and introduces no per-kind routes,
repositories or custom queue. No documented coding-standard blocker was found.

## Independently executed evidence

Native `rom-dev`, Rust 1.99.0, existing idle target
`/var/tmp/rom-compensation-target`, two build jobs, dev/test debug information
disabled, locked offline dependency resolution:

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-compensation-target
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_NET_OFFLINE=true
cargo test --locked -p rom-demo --test compensation --test workshop
cargo test --locked -p rom-cli --test loopback
cargo run --locked -p rom-demo -- smoke sqlite
cargo run --locked -p rom-demo -- smoke redb
```

All passed: one compensation test internally exercises four adapter/authority
combinations, three workshop tests and three real-binary CLI loopback tests.
Both smoke commands exercised compensation over actual TCP alongside the existing
demo journey. These are this reviewer's executions, separate from the
implementer's reported `./demo/verify` run. I did not rerun all workspace quality
gates, formatting, Clippy or rustdoc for this review.

An additional independent Node harness launched the real `rom-demo serve` process
on a fresh database and ephemeral local port, then ran the README's actual `rom`
CLI commands with an auth file and JSON stdin, for **both SQLite and redb**:

1. Reserve three units under `checkout-a`, then two under `checkout-b`, using
   expected Stock revisions 1 and 2.
2. Record `unknown` with Checkout revision 1; read confirms both tokens remain.
3. Record `confirmed_rejected` with Checkout revision 2; poll the asynchronous
   result until Stock revision 4, total 10, reservations exactly `{"checkout-b":2}`.
4. Replay the identical rejection command with the original expected revision and
   idempotency key; Stock remains revision 4 with B intact.
5. Send SIGINT and require a successful process exit within five seconds.

Both workflows passed. This separately checks that the published CLI flags/input
shape work in the served application and respects the README's asynchronous-read
caveat. Local harness source was retained in the isolated review worktree as
`compensation-cli-review.mjs`; synthetic databases/auth files were retained under
`/var/tmp/rom-compensation-target/independent-compensation-cli-*` in `rom-dev`.
No external payment or messaging service was contacted.

## Optimization assessment and evidence limits

The performance statements are exact counts for a fixed scenario, not latency or
throughput claims: creation/unknown each claim one mapper, successful rejection
claims mapper plus target action, denied service claims only the mapper, and
original-identity replay claims no new work. The successful recovery adds one
Stock event; replay adds none. The focused tests execute these assertions.

The mapper performs no collection query and emits at most one target. However,
`reserve` sums the existing reservation map, so its domain work is linear in the
number of reservations; normal resource cloning/encoding and durable ledger work
also remain. “No scans” should be understood as no new collection scan in this
composition, not constant total runtime cost. Existing core capacity/retry limits
still apply. No peak memory, allocation, elapsed-time or throughput benchmark was
performed, and no new caching/indexing framework is justified by this demo.

This review establishes the scoped maintained demo behavior and documentation,
not a generic compensation policy, durable external payment outcome oracle,
process-crash experiment, automatic reconciliation service or production
throughput profile. Broader failure injection belongs to the separately retained
compensation prototype and must not be credited to these demo tests.
