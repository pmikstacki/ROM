# Explicit compensation demo plan

The approved scope contains two ordinary Resource declarations and one Reaction. A durable
Checkout records its reservation target/token before reserving stock. An explicit
simulated payment outcome of confirmed_rejected schedules a token-specific release.
Unknown/transient outcomes do not schedule a release. This is application composition, not a generic
worker-failure hook or automatic rollback. There is no external payment integration.

Implementation: `src/compensation.rs` owns declarations/actions/seed; `lib.rs`
registers and seeds them through existing paths. The implementation uses one mapping, no scans, and no custom
route/queue/orchestration engine. README supplies a generic CLI walkthrough.

Assertions for SQLite and redb cover these outcomes:

- A/B reservations survive unknown.
- Confirmed rejection releases only A.
- Original identity replay adds no event.
- Clean reopen retains B.
- Revoked current service authority blocks release.

Existing TCP smoke and serve
startup regression must pass. Compare baseline/targeted journal and work counts.
Keep the runtime's bounded durable worker behavior explicit.

Implemented verification: `./demo/verify` passes (format, warning-free Clippy,
all demo tests, actual TCP smoke SQLite/redb, serve/reopen/SIGINT both adapters,
rustdoc). Focused compensation tests exercise four adapter/authority combinations.
Discovery assertions explicitly include the new metadata in demo and CLI tests.

Measured fixed-scenario cost: checkout creation = one mapper; unknown outcome =
one mapper and zero Stock events; rejection = mapper + action and one Stock event;
receipt replay = zero claims/events. The mapper emits at most one target and does
not query/scan collections. Existing core ledger/work/retry limits remain in force.
