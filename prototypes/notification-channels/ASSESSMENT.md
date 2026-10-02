# Implementer assessment

## Result and evidence

The named async-function seam works without provider branches in dispatch. A separate `notification-core` crate contains the callback registry, checked/versioned payload envelope, stable DeliveryId, semantic Outbox port, outcome vocabulary, and retry orchestration. Its observed Cargo dependency tree contains Tokio and serialization crates, with no rusqlite dependency. The SQLite implementation is confined to `notification-sqlite-probe`.

On 2026-10-02 the complete verifier exited successfully: formatting, Clippy `-D warnings`, all-target build, twelve tests, and the ordinary-function example. Nine initial tests first failed against explicit unimplemented skeletons, then passed after implementation. Three additional contract checks cover lease fencing, repeated worker loss, and differently typed named functions. No claim of full production conformance follows from these tests.

An initial compile failed because rusqlite does not decode unsigned 64-bit integers directly. The adapter now converts its signed SQL clock value with range checking. This is an adapter concern; the semantic interface retains unsigned clock ticks. An attempted host-side Python edit could not run because Python is absent; the edit was applied using the patch tool. Neither failure remains in the final verifier.

## What the failure tests establish

The SQL rollback test deliberately inserts two intentions with one primary key in the same transaction after changing Resource state. SQLite rejects the second insertion; connection reopen sees the previous state and no intention. This exercises actual SQL atomicity rather than a callback pretending that persistence failed.

Claims are durable before callback invocation. Each claim increments a durable generation/attempt number and sets a lease deadline. Normal retries preserve DeliveryId. Acknowledgment updates require the current generation and leased state, so an old worker cannot overwrite a newer claim's result. If a worker disappears on its final attempt, lease expiry moves the intention to failed/unknown rather than creating unlimited retries.

The ambiguous window is explicit: the local recording receiver applies its effect and signals a gate, then waits before returning Accepted. The harness inspects the database and confirms that no receipt exists, aborts the worker task, closes/reopens the adapter, and retries after lease expiry. Without recipient deduplication there are two effects. With the receiver's own DeliveryId set there is one. Both have two attempts. The outbox cannot guarantee exactly-once external effects; a lease also cannot fence an external receiver by itself.

Timeout maps to Unknown, not rejected. Unknown and Retryable use the same bounded retry policy in this experiment. Permanent and exhausted work remain inspectable in the database without an invented acceptance receipt. There is no provider receipt-lookup reconciliation or manual-review workflow yet.

## What to carry forward

Preserve small ordinary async functions, named registration, versioned checked payloads, stable delivery identity, explicit Accepted/Retryable/Permanent/Unknown results, and durable attempt bookkeeping. Integrate intention creation into the same generic persistence operation that commits Resource state/revision/action receipt/events. Keep database transactions closed during callback execution. Reuse the existing bounded execution/supervision machinery rather than introducing a channel-specific task runtime.

Before reuse as a production component, replace String errors with a stable semantic error type, provide semantic payload validation, receipt size limits/redaction, explicit provider retry-after and unknown-outcome reconciliation policies, and host-setup validation. Separate sender acceptance from eventual delivery callbacks. Add backlog/retention/dead-letter administration, real clock policy, jitter, blocking-driver isolation, real subprocess crash tests, concurrent-worker stress, and storage fault injection.

Do not generalize the local receiver's deduplication set to a durable provider guarantee. Do not generalize database reopen to machine-crash durability. Do not treat the scratch SQLite schema as ROM's persistence abstraction. No provider SDK or provider has been selected, and nothing was sent outside this process.

## References and provenance

- Parent design inputs: `/root/ROM/docs/research/capability-prototype-brief.md` and `/root/ROM/docs/research/storage-and-notification-adapters.md`.
- [rusqlite 0.40.2 Connection](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Connection.html) and [Transaction](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Transaction.html): adapter APIs exercised by the build/tests.
- [Tokio timeout](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html): cooperative timeout boundary; the paused-clock test executes it.
- [SQLite transactions](https://www.sqlite.org/lang_transaction.html): rollback behavior underlying the real constraint-failure experiment.
- [Transactional outbox guidance](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html): intent persistence does not eliminate duplicate delivery.
