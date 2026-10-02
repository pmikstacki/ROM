# Named notification channel probe

Throwaway adapter experiment, 2026-10-02. No email, HTTP request, cloud provider, network listener, or external delivery is used. `Resource` is the only domain entity; delivery IDs, intents, leases, and receipts are internal work metadata.

The same dispatcher accepts named ordinary async Rust functions with different decoded payload types. Its crate has no SQLite or provider SDK dependency. SQLite lives in a separate scratch adapter and atomically stores a Resource change plus notification intentions. Workers claim committed work, release the database transaction, call the registered function, and record the outcome afterward.

```rust,ignore
async fn custom_send(delivery: Delivery<AccountNotice>) -> Outcome {
    // Translate to your integration here; preserve delivery.id on retries.
    Outcome::Accepted(format!("local:{}", delivery.id.0))
}
app.register("account-updates", "account-notice/v1", custom_send)?;
let intent = app.intent("delivery-id", "account-updates", "account-notice/v1", &notice)?;
scratch.transition("account-1", "updated", &[intent])?;
```

Run the complete ordinary-function example in `sqlite/examples/custom_function.rs`. It uses an in-memory database; the recovery tests use temporary on-disk databases. Strings and the scratch transition method are deliberately not a proposed final Resource authoring API.

From the host:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/notification-channels/prototypes/notification-channels && ./verify'
```

`verify` checks formatting, Clippy with warnings denied, all-target builds, all workspace tests, the runnable example, and the core dependency tree. Exact direct pins: Tokio 1.53.1, Serde 1.0.229, serde_json 1.0.151, rusqlite 0.40.2 with bundled SQLite. Cargo.lock pins transitives. Verified on Rust 1.99.0 (`b940084d7`), Cargo 1.99.0 (`5f94df478`) in `rom-dev`.

The twelve executable checks cover:

- A real SQL uniqueness failure after updating Resource state rolls back state and all intents; reopen observes the old state.
- Committed state and pending work survive connection/worker reconstruction. Accepted receipts survive reopen and prevent further attempts.
- Retryable results attempt at ticks 0, 10, and 30, never at 9 or 29; the third failure is terminal. Every attempt keeps the same DeliveryId.
- Permanent failure is terminal after one attempt and never stores a provider receipt.
- Registration rejects duplicate names, missing channels, wrong schema versions, incompatible payloads, and oversized payloads.
- Receiver acceptance followed by worker cancellation before acknowledgment produces two attempts and two effects without receiver deduplication.
- The same failure window produces two attempts and one effect with a receiver-owned in-memory deduplication set.
- A hanging callback times out under Tokio's paused clock; the persisted outcome is unknown, pending, and has no receipt.
- A channel version change fails old work without invoking the replacement callback.
- An expired worker cannot overwrite a newer worker's receipt with a stale claim generation.
- Three successive lost workers exhaust the persisted attempt budget and leave an inspectable unknown failure.
- Two differently typed ordinary functions run through the same dispatcher under different names.

Delivery semantics are **durable at-least-once attempts within a finite budget**, not eventual delivery or exactly-once effects. Acceptance means the callback reports downstream acceptance. The receiver deduplication control has only the receiver's lifetime/retention guarantee.

## Bounds and deliberate limits

One dispatch step handles one intention. Configuration requires 1–16 attempts, positive backoff at most 86,400 clock seconds, a positive callback timeout shorter than the lease, and caps exponential delay at 86,400 seconds. Payloads are limited to 64 KiB and each scratch transition to 32 intents. Registration checks a version string and Serde decoding; it does not implement semantic business validation or schema migration. Callback execution must cooperate with asynchronous cancellation: synchronous blocking or CPU loops cannot be preempted by Tokio timeout.

Retry timestamps use explicit persisted clock ticks supplied by the harness, measured from attempt start. This proves deterministic eligibility/backoff and restart behavior without wall-clock sleeps. Production needs a real clock policy, deadline budget, jitter, and clock-skew handling. The outbox backlog, retained terminal records, and receipt size are not globally bounded here. The worker is caller-driven, not a complete supervised background service.

SQLite calls are synchronous and intentionally small; this scratch driver runs them directly. Production must isolate blocking I/O. WAL/FULL is configured, but tests exercise SQL rollback, worker cancellation, and connection reopen—not SIGKILL, OS crash, torn writes, power failure, disk-full recovery, migration, or distributed failover.

This outbox is **separate from the parallel persistence-adapter probe**. It does not yet reuse that protocol or implement Resource revision checks, action receipts, events, authorization, or command idempotency. Duplicate enqueue IDs are rejected by the database; the surrounding Resource action's command receipt protocol must eventually decide whether a repeated command should return its original outcome.

See `ASSESSMENT.md` for observations and implementation decisions.
