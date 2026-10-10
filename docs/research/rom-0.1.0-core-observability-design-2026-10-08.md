# Core observability across durable work

Date: 2026-10-08. Status: source investigation and proposed design. No implementation or performance experiment was run for this report.

## Recommendation

Add an opt-in, bounded diagnostic channel to the ordinary Resource execution path. Keep the exporter in the host.
Use the locked Tokio channel implementation. Use fixed records and `try_send`; never call a subscriber or user observer under a core gate.

Correlate action ownership, receipt resolution, committed events, reaction claims and external delivery attempts.
Retain correlation after restart through domain-separated HMAC tokens and a protected, host-supplied key.
The session-only alternative cannot satisfy restart correlation by itself.

The owner permits autonomous technical choices. Dependency review, conformance tests and release checks remain required; they are not permission requests.
This recommendation does not close R9. RuntimeStatus and the AI observer do not provide this complete ordinary-action path today.

## Inspected source

The investigation used the following current seams:

| Source | Existing behavior and integration constraint |
| --- | --- |
| [execution/command.rs](../../crates/rom/src/execution/command.rs) | Admission, receipt lookup, validation, action execution, conditional commit and final disclosure have separate outcomes. Several return paths hold a gate. |
| [execution/lifecycle.rs](../../crates/rom/src/execution/lifecycle.rs) | Runtime-owned work retains capacity after caller cancellation. RuntimeStatus is a snapshot, not causal history. |
| [reactions/claims.rs](../../crates/rom/src/reactions/claims.rs) | Persisted cause, claim, retry epoch and child work preserve causal identity. Receipt replay can complete a claim without another mutation. |
| [channels/execution.rs](../../crates/rom/src/channels/execution.rs) | DeliveryStarted is persisted before the external callback. Callback outcome and DeliveryFinished persistence have distinct failure boundaries. |
| [AI telemetry](../../crates/rom-ai/src/telemetry.rs) | FlowObserver catches observer panic. Its documentation correctly states that this cannot preempt a blocking callback. |

The core lockfile contains Tokio 1.53.1, tracing 0.1.44, tracing-core 0.1.36 and sha2 0.10.9.
Tokio is already a core dependency with `sync`. Tracing is transitive; hmac and OpenTelemetry are absent from this lockfile.
Do not infer adoption from a transitive dependency or a research reference.

The reviewed Cargo.lock SHA-256 is `f6ac31a111d67e8c4d1cc8bc0438ac0cf02bd87e851c78c86d6e2756b1241bf3`.
The source baseline was HEAD `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`; concurrent work can change later source.

## Compare the small seams

| Approach | Benefit | Risk or cost | Decision |
| --- | --- | --- | --- |
| Bounded Tokio channel, drained by host | Reuses locked dependency; async receiver; no exporter call inside core | Records can be lost; queue allocation and bookkeeping need measurement | Recommended |
| Bounded std sync channel | Small standard-library seam; host can drain from a dedicated thread | Blocking `send` is unsafe here; async host needs a bridge | Valid alternative, no reason to add a second core path |
| Direct tracing events | Familiar spans and host integrations | Subscriber executes on the emitting thread; user subscriber can block or panic | Use only after the host drains records |
| Direct user callback | Simple initial interface | Callback can block, panic, re-enter ROM or retain unbounded data | Reject for core diagnostics |

Tokio 1.53.1 `try_send` obtains capacity without awaiting it. Full and closed queues return errors.
This is a capacity-wait guarantee, not a hard real-time bound on scheduling or allocation.
The exact implementation is [Tokio's locked bounded channel source](https://docs.rs/tokio/1.53.1/src/tokio/sync/mpsc/bounded.rs.html).

Standard `SyncSender::send` can block when the buffer is full. `try_send` returns immediately with Full or Disconnected.
Successful enqueue does not guarantee eventual consumption. See [Rust SyncSender](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html).

The installed tracing-core 0.1.36 `Dispatch::event` calls `subscriber.event_enabled` and `subscriber.event` directly.
The source SHA-256 is `d2a0584196c7e053006490baebfa61c6b3145138e3209f82f44d1afbe066acb6`.
This is source evidence for the isolation concern, not a benchmark or a claim that every subscriber blocks.
For async host spans, [tracing 0.1.44 Instrument](https://docs.rs/tracing/0.1.44/tracing/instrument/trait.Instrument.html) enters context when polling and dropping the instrumented future.

OpenTelemetry recommends bounded memory and no default application blocking. It recognizes telemetry loss under overload and bounded flush timeouts.
These principles support the selected boundary; they do not require an OpenTelemetry dependency in ROM.
See [OpenTelemetry performance](https://opentelemetry.io/docs/specs/otel/performance/).
Exporter errors must not replace business results. See [OpenTelemetry error handling](https://opentelemetry.io/docs/specs/otel/error-handling/).

## Proposed public API

The following names are proposals, not existing public API:

```rust
let (diagnostics, mut reader) = Diagnostics::bounded(options)?;
let runtime = Builder::default().diagnostics(diagnostics).build(storage)?;
// The host drains reader and selects its exporter independently.
```

`DiagnosticOptions` contains validated queue capacity and a protected correlation key.
Use a default capacity of 512 records and a documented maximum of 4096 for the first experiment.
Reject zero and excessive capacity at configuration time. Do not silently resize the queue.
These limits are proposed experiment parameters, not measured production defaults.

`DiagnosticReader` provides `recv`, `try_recv`, `close` and `stats`.
`DiagnosticSink` is cloned internally by accepted work. Neither handle contains a Runtime or storage handle.
The host reader must not keep the database alive after Runtime shutdown.

`DiagnosticEvent` contains fixed-size values: schema version, runtime epoch, local sequence, opaque operation/root/parent/work tokens, stage and outcome.
It can also contain bounded attempt/depth counters, event count and monotonic elapsed time.
Use enums for outcomes. Do not include arbitrary strings, error Display, Resource values, principal names, raw identifiers or idempotency keys.
Do not call application Debug or codecs to produce records.

Add named `diagnostics` modules. Keep root files as exports and declarations.
Keep existing public paths stable. Share one recorder across commands and durable work; avoid separate AI-only or adapter-only correlation machinery.
Existing AI APIs remain compatible. A host can bridge their observations separately without asserting that this completes ordinary core instrumentation.

## Lifetime and failure rules

1. With diagnostics disabled, skip record construction, token hashing and timing calls.
2. When enabled, collect fixed-size stage facts without invoking host code.
3. Release all core and persistence gates before queue publication.
4. Use `try_send` only. Never await diagnostics capacity from an action or worker.
5. Count Full and Closed separately through bounded, saturating counters.
6. After reader closure, use a disabled fast path for later records.
7. If the exporter fails, preserve the original action, receipt and delivery outcomes.

A fixed per-operation collector needs its own explicit capacity.
Bound it independently of request size, chain length and callback output.
If it overflows, count lost records; do not allocate another vector or wait.
Review destructor order: a collector's Drop must not publish while an earlier return still owns a gate.
Prefer explicit outer-scope publication after the guarded closure returns.

Admission belongs to ROM, not to the caller's future. Caller cancellation is a separate wait outcome.
Emit terminal execution status when owned work actually finishes, not when the caller stops waiting.
Emit bounded shutdown/drain stages from existing lifecycle ownership. Do not add an exporter to Runtime's drain condition.

The host closes and drains the reader after Runtime shutdown, with a host-selected finite timeout.
It may discard remaining telemetry on timeout. It must not detach an unbounded queue or retain Runtime through an exporter task.
A synchronous blocking exporter needs a dedicated host boundary; an async timeout cannot preempt a blocked thread.

A diagnostic stream is lossy. It is not the durable journal, receipt store or a recovery authority.
Per-operation sequence and loss counters reveal gaps. Concurrent operations can interleave; do not promise global causal ordering from enqueue order.
OpenTelemetry's batching processor also bounds its queue and separates export processing.
See [OpenTelemetry trace SDK batching](https://opentelemetry.io/docs/specs/otel/trace/sdk/#batching-processor).

## Correlation without raw identities

An unkeyed hash is not an adequate privacy boundary for guessable IDs or idempotency keys.
Use full HMAC-SHA256 output, with separate domains for operation/receipt, work, causal root and generation-sensitive claims.
A parent field references the Work namespace so it equals the preceding work token. Separate parent hashing would break that link.
Encode input components with explicit length boundaries and a schema version. Do not concatenate ambiguous strings.

The host supplies a protected 32-byte key when it opts in.
The key type must not implement Debug or Serialize. Never emit, archive or log the key through diagnostics.
Use fixed test keys only in test fixtures. Do not add a production default key.
The host owns secret storage, access control, key replacement and the exporter.

Use existing persisted receipt identity and WorkCause fields to derive stable links.
Restart must recover the same links with the same key and canonical identity.
A new runtime epoch separates process-local sequence and elapsed time from stable causal links.
Key rotation changes the correlation namespace; document the loss of cross-key correlation.
No token grants permission, authenticates a user or supplies a public Resource identifier.

Session-only opaque counters avoid another cryptographic dependency.
They correlate work within one runtime, but lose receipt and pending-work links after restart.
Keep this alternative explicit; do not use it to claim completion of the full R9 requirement.

The candidate is RustCrypto hmac 0.12.1 because its digest 0.10 dependency matches the locked sha2 0.10.9 graph.
The published package supports Rust 1.41 and offers MIT or Apache-2.0 licensing.
A newer hmac release exists; version choice here follows graph compatibility, not a claim that 0.12.1 is latest.
See [hmac package metadata](https://docs.rs/crate/hmac/0.12.1), [versioned manifest](https://github.com/RustCrypto/MACs/blob/hmac-v0.12.1/hmac/Cargo.toml) and [sha2 manifest](https://docs.rs/crate/sha2/0.10.9/source/Cargo.toml).

A read-only advisory check used local RustSec commit `ef6173cbc5c50ec8166f9a5b28f07834144373ee`, dated 2026-10-03.
That snapshot contains no hmac or digest package advisory directories. Its sha2 advisory is RUSTSEC-2021-0100, patched at 0.9.8.
The locked 0.10.9 is outside that affected version. See [the sha2 advisory](https://rustsec.org/advisories/RUSTSEC-2021-0100.html).
The live advisory index search also found no exact “in hmac” entry. This limited check is not a current complete dependency audit.
Before adoption, refresh the advisory database and audit the resolved graph, features, licenses and ROM's declared MSRV.
No dependency was installed or accepted by this investigation.

Do not label opaque HMAC tokens as W3C trace IDs automatically.
The host adapter can create valid trace/span IDs and attach causal links.
OpenTelemetry defines events and links separately; those structures fit retries and asynchronous reactions.
See [OpenTelemetry trace API](https://opentelemetry.io/docs/specs/otel/trace/api/).

Use fixed stage and outcome values as metric labels.
Keep per-operation tokens out of metric labels to prevent unbounded cardinality.
Treat trace correlation as sensitive operational data, even without raw payloads.

## Timing and truthful outcome boundaries

Measure local stages with `Instant` and checked elapsed durations.
If a duration cannot be established, emit an unavailable timing flag; do not invent a positive duration.
Do not compare Instants across processes or restarts.
Rust documents platform-specific suspend behavior and rare monotonicity failures.
See [Rust Instant](https://doc.rust-lang.org/std/time/struct.Instant.html).

The host can attach a wall-clock anchor for export. Do not replace ROM's persisted Clock or retry rules with telemetry timestamps.
Measure queue wait, validation, execution, storage commit, receipt resolution, reaction mapping and external attempt separately.
Record only stages that actually occurred. An early rejection must not manufacture execution or commit timing.

| Boundary | Required diagnostic distinction |
| --- | --- |
| Admission versus caller cancellation | Owned work can continue after waiting ends. |
| Receipt found versus new action | Replay resolves a result without a second mutation. |
| Commit Ok versus Unknown | Unknown may have committed; never report confirmed rollback or success from this response alone. |
| State change versus no-op | Report actual committed event count, including zero. |
| Reaction claim versus mapper output | A stopped or denied claim is distinct from materialized child work. |
| DeliveryStarted versus callback completion | An external attempt is not a confirmed accepted delivery. |
| Callback outcome versus DeliveryFinished persistence | Callback return and durable recording can fail independently. |
| Reopen and recovery | Reuse the stable root/work token; allocate a new local attempt sequence and runtime epoch. |

Current authority checks still govern action execution and receipt disclosure.
Diagnostics must not expose raw current-owner values or disclose a stored result to an unauthorized exporter.
Emit authority outcome categories only. Observe durable work ownership through its opaque claim/work links.

## Executable acceptance plan

Run the same maintained cases on actual SQLite and redb, through public core APIs.
Do not substitute an in-memory fake for durable receipt or work recovery.

| Case | Required proof |
| --- | --- |
| Ordinary success chain | Action, receipt, event, reaction, child action and external attempt share the intended root links and ordered stage facts. |
| Retry after response loss | Inject Unknown after commit; reopen; replay resolves the same receipt token without another mutation/event/effect. |
| Pending work after restart | Recover the same root/work token with the same protected key; attempt numbering reflects durable work. |
| Authority change | Denied execution and denied receipt disclosure preserve business behavior and emit no owner names or payloads. |
| Rejection, conflict and callback panic | Distinct bounded outcomes; no fictitious commit success; diagnostics cannot poison a core gate. |
| Queue overload | Stop draining a tiny queue; actions and worker recovery still finish; exact dropped-record counters increase. |
| Reader disposal | Drop or close reader; normal actions and shutdown continue; no retained database owner or capacity permit. |
| Caller cancellation | Cancel after admission; action remains owned; terminal execution is recorded only on actual completion. |
| Exporter isolation | Host exporter sleeps, errors or panics; database result and Runtime shutdown remain independent. |
| Redaction | Credential, principal, raw ID, idempotency, Resource and tool-argument sentinels occur nowhere in record or host trace output. |
| Key and encoding rules | Fixed-key vectors; domain separation; ambiguous component boundaries; same-key restart; different-key and rotation behavior. |
| Queue and collector limits | Reject invalid options; bound buffered memory; count overflow; never allocate proportional to chain length. |

Also test concurrent interleaving and dropped stage records.
A reader must identify incomplete histories rather than reconstruct an invented full trace.
Test diagnostics with disabled, saturated and active readers. Ensure hostile application Debug implementations are never called.

Benchmark disabled, enabled-undrained, enabled-drained and host-exported paths with the same workload and source candidate.
Report p50/p95/p99 latency, elapsed cost per action, allocations, queue high-water, drops and RSS.
Measure hashing and queue overhead separately from database transaction cost.
Include ordinary success, replay, no-op, rejected input, reaction fanout and external attempts.
Set finite workload counts and memory limits before each run. Record compiler, lockfile, adapter, seed and baseline.

No overhead is established yet. Do not describe the channel as free or negligible before these measurements.
Release admission needs actual redaction, lifecycle and restart evidence, plus the bounded overhead results.

## Implementation preparation update

The coordinator executed the maintained missing-interface RED in `/var/tmp/rom-010-diagnostics-initial-interface-red.log`.
It exited 101 for missing Diagnostic types and `Builder::diagnostics`, as intended.
This is a missing-interface result, not an executed behavior failure or GREEN acceptance.
The current advisory snapshot is `b8a1a33e246a0a9a3b5f377248c41a503defec74`, dated 2026-10-07.
The selected hmac 0.12.1 archive is 42,657 bytes, SHA-256 `6c49c37c09c17a53d937dfbb742eb3a961d65a994e6bcdcf37e7399d0cc8ab5e`.
Read-only inspection found no hmac/digest advisory directory in that snapshot. Final resolved-graph audit remains coordinator-owned.
Implementation modules and unit cases are staged separately under `/var/tmp/rom-010-diagnostics-stage/diagnostics/` while core source is frozen.
At that preparation boundary, no staged code had been compiled or admitted. The host must supply an explicit nonzero 16-byte session.
Parent links reuse the Work identity namespace. Claim tokens include generation while Work tokens remain stable.

## Work integration observations

The coordinator released the source freeze and admitted the module for native characterization.
The initial public-interface RED remains retained. It is not a behavior failure.
Native run 75413 passed seven maintained cases and exposed missing chain observations.
The work instrumentation now observes mapper execution and materialization separately from delivery start, external attempt and delivery status persistence.
An external Unknown does not become success because its delivery status was saved.
The expanded fixture tests this distinction on both real adapters and after reopen.
Run 27901 passed eight cases; the new fixture failed on a missing expected revision before reaching diagnostic assertions.
The fixture now supplies revision 1. No product change was justified by that failure.
Run 62592 passed the diagnostic unit cases and found Clippy unchanged-error observer warnings.
The observers now use `inspect_err`. Re-execution and independent review remain required.

`elapsed_ns` is cumulative local recorder time. `stage_elapsed_ns` is a checked supervised stage interval.
For Reaction, this interval includes pool queue wait, mapper execution, and return transit.
Both use monotonic process-local timing. Queue age remains unavailable unless a host fixture records actual enqueue and claim times.
Diagnostic receive order is not durable journal order. Sequence values identify allocation order and loss.

The bounded ignored command benchmark compares disabled, enabled-drained and enabled-undrained modes.
Its fixed workload has 64 creates and 512 retained-receipt replays per mode, three rounds and two adapters.
It includes host drain cost in the drained mode. It does not establish production SLOs or network exporter cost.

## Executed command characterization before lifecycle additions

The corrected maintained suite passed nine cases, with one overhead case ignored.
Evidence: `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-diagnostics-work-corrected.log`.
Each applicable case ran on SQLite and redb. The configuration rejection case has no adapter dependency.
The log confirms no failures. This is a focused behavioral result, not full release acceptance.

The coordinator's subsequent run 33089 completed Clippy, the explicit overhead case and the core regression suite.
The overhead log contains 18 measured rows and one passing ignored-case invocation.
Evidence: `/var/tmp/rom-010-diagnostics-overhead.log` and `/var/tmp/rom-010-diagnostics-core-regressions.log`.
The native Clippy log is `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-diagnostics-clippy-final.log`.
The regression log includes 115 unit cases and two passing doctests, plus its listed integration targets.
These checks preceded the new lifecycle helper and cancellation case.

| Adapter and mode | Whole-workload wall time, three rounds (ms) | Replay p50 range (µs) | Replay p99 range (µs) |
| --- | --- | --- | --- |
| SQLite, disabled | 424.99 / 418.02 / 417.75 | 565.00–601.35 | 624.89–758.79 |
| SQLite, drained | 451.52 / 442.76 / 438.11 | 603.61–619.91 | 778.81–841.28 |
| SQLite, undrained | 432.00 / 440.77 / 423.84 | 588.46–612.71 | 651.69–767.51 |
| redb, disabled | 452.78 / 457.57 / 457.00 | 564.61–568.29 | 671.51–726.44 |
| redb, drained | 465.82 / 477.93 / 480.26 | 567.58–606.62 | 627.90–722.29 |
| redb, undrained | 456.18 / 470.87 / 594.76 | 555.30–604.82 | 642.58–4542.83 |

Summed drained wall time increased 5.68% on SQLite and 4.14% on redb against their respective disabled rounds.
The drained mode includes host drainage. These values are observations from this debug-profile fixture, not portable overhead guarantees.
Modes ran in fixed disabled → drained → undrained order. There were three rounds without randomized scheduling or isolated system load.
The third redb undrained round has a larger tail. This run cannot attribute that tail to diagnostics or exclude scheduler noise.
The observed undrained increases were 2.84% on SQLite and 11.30% on redb, including that tail.

Each drained round exported 3,328 records without loss. Each capacity-one undrained round enqueued one and dropped 3,327 records.
All modes preserved 64 durable creation events and the replay results. Dropped observations did not alter business semantics.
The fixture does not measure allocations, RSS, production concurrency, a network exporter, or a reaction-heavy workload.
A final-source rerun is required after lifecycle integration before using these values for release admission.

## Explicit shutdown observations

The shutdown fixture's first run failed to compile because `unwrap_err` required an unrelated Resource Debug implementation.
The fixture now matches the cancellation error directly. That failure is retained in the native shutdown-red log.
The corrected intended RED failed at the missing Shutdown/Started observation.
Evidence: `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-diagnostics-shutdown-behavior-red.log`.

The coordinator added Started publication after the lifecycle close lock is released.
A terminal record is published only after the current waiter observes the existing drain condition or its error.
The implementation invokes no exporter, adds no scheduler, and publishes no observations from `Drop`.
The separate Lifecycle HMAC domain uses the explicit host session. It does not represent an action or uniquely identify waiters.
Cancellation can leave Started without a terminal record. `RuntimeStatus` remains the sampled current-state authority.
The terminal `stage_elapsed_ns` measures that waiter's checked local interval; its `elapsed_ns` belongs to its new recorder.

The post-change diagnostics log reports ten passing maintained cases and one ignored overhead case.
Evidence: `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-diagnostics-shutdown-green.log`.
The native post-change Clippy log completed successfully:
`/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-diagnostics-shutdown-clippy.log`.
The held-action case passed on both adapters. It preserved two durable events after caller and waiter cancellation.
These checks do not replace the final full verifier, independent redaction review, packaged consumer test, or human adoption evidence.
The earlier overhead values remain measurements of the earlier source candidate.
