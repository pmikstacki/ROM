# Core Observability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Diagnose ordinary durable Resource work across actions, receipts, events, reactions and external attempts without exposing data or coupling commits to exporters.

**Architecture:** Core emits fixed records to a bounded Tokio channel. The host drains records and owns its exporter. Domain-separated HMAC tokens preserve correlation after restart with a protected host key.

**Tech Stack:** Locked Tokio 1.53.1; sha2 0.10.9; candidate RustCrypto hmac 0.12.1; actual SQLite and redb conformance fixtures.

**Spec:** [Core observability design](../../research/rom-0.1.0-core-observability-design-2026-10-08.md).

## Global constraints

- Preserve Resource → action → atomic state/receipt/event/work semantics.
- No exporter, user callback or tracing subscriber runs under a core or persistence gate.
- Never await diagnostic capacity. Count dropped records without changing a business result.
- Key and options do not implement Debug or Serialize. Records contain no raw identity, payload, idempotency key or error text.
- Disabled diagnostics skips record construction, token hashing and timing.
- Preserve public paths; put implementation and tests in named modules.
- The owner permits autonomous technical choices and parallel work. No new approval loop is required.
- Stage the plan and tests first. Do not modify source modules or start builds until the coordinator releases the current verifier and source witness.
- Do not claim tests or overhead measurements were executed from this staged plan.

## File ownership and responsibilities

| File | Responsibility | Owner |
| --- | --- | --- |
| `crates/rom/src/diagnostics/mod.rs` | Module declarations and public exports only | Diagnostics agent, after release |
| `crates/rom/src/diagnostics/types.rs` | Fixed records, stages, outcomes and stats | Diagnostics agent |
| `crates/rom/src/diagnostics/channel.rs` | Validated capacity, reader/sink, loss counters | Diagnostics agent |
| `crates/rom/src/diagnostics/correlation.rs` | Protected key and domain-separated HMAC | Diagnostics agent |
| `crates/rom/src/diagnostics/recording.rs` | Bounded operation collector and checked elapsed timing | Diagnostics agent |
| `crates/rom/src/diagnostics/tests.rs` | Internal cryptographic vectors and collector limits | Diagnostics agent |
| `tests/persistence/tests/diagnostics.rs` | Actual two-adapter behavior and integration regressions | Diagnostics agent |
| `Cargo.toml`, `Cargo.lock`, `crates/rom/Cargo.toml`, `crates/rom/src/lib.rs` | Dependency admission and public exports | Coordinator |
| `execution/state.rs`, `execution/builder.rs` | Optional sink storage and registration | Coordinator |
| `execution/command.rs`, `reactions/claims.rs`, `reactions/worker.rs`, `channels/execution.rs`, `execution/lifecycle.rs` | Instrumentation at real ownership/outcome boundaries | Coordinator until reassigned |

## Public and internal interfaces

Names below are selected preparation interfaces. The coordinator requires an explicit host-supplied session.

```rust
pub struct DiagnosticKey(/* protected [u8; 32], no Debug/Serialize */);
impl DiagnosticKey { pub fn new(key: [u8; 32]) -> Self; }
pub struct DiagnosticOptions(/* protected key, capacity, session */);
impl DiagnosticOptions {
    pub fn new(key: DiagnosticKey, session: [u8; 16]) -> Self;
    pub fn capacity(self, capacity: usize) -> Self;
}
pub struct Diagnostics;
impl Diagnostics {
    pub fn bounded(options: DiagnosticOptions)
        -> Result<(DiagnosticSink, DiagnosticReader)>;
}
impl Builder { pub fn diagnostics(self, sink: DiagnosticSink) -> Self; }
impl DiagnosticReader {
    pub async fn recv(&mut self) -> Option<DiagnosticEvent>;
    pub fn try_recv(&mut self) -> Option<DiagnosticEvent>;
    pub fn close(&mut self);
    pub fn stats(&self) -> DiagnosticStats;
}
```

`DiagnosticOptions` defaults to capacity 512. Accept 1..=4096. Reject invalid capacity before intake opens.
`session` identifies a host stream; it is not an authentication identity.
A host-provided session supports cross-process uniqueness without inventing a new core random-number mechanism.
Reject an all-zero session. Host-provided nonzero bytes do not prove uniqueness; the host must supply a distinct stream identity.

`DiagnosticToken` wraps `[u8; 32]` and supports equality, Debug and Serialize for redacted records.
`DiagnosticSink` supports Clone but not Debug or Serialize. Neither reader nor sink retains Runtime or storage.

```rust
pub struct DiagnosticEvent {
    pub version: u16,
    pub session: [u8; 16],
    pub sequence: u64,
    pub operation: DiagnosticToken,
    pub root: DiagnosticToken,
    pub parent: Option<DiagnosticToken>,
    pub work: Option<DiagnosticToken>,
    pub claim: Option<DiagnosticToken>,
    pub stage: DiagnosticStage,
    pub outcome: DiagnosticOutcome,
    pub attempt: u32,
    pub depth: u32,
    pub elapsed_ns: Option<u64>,
    pub stage_elapsed_ns: Option<u64>,
    pub event_count: u32,
}
pub enum DiagnosticStage {
    Admission, Authorization, Validation, Execution, Commit,
    Receipt, Event, Reaction, ExternalAttempt, Work, Shutdown,
}
pub enum DiagnosticOutcome {
    Started, Succeeded, Replay, Denied, Invalid, Conflict,
    NotCommitted, Unknown, Panicked, TimedOut, Retryable,
    Permanent, Cancelled, Stopped,
}
pub struct DiagnosticStats {
    pub enqueued: u64,
    pub dropped_full: u64,
    pub dropped_closed: u64,
    pub dropped_collector: u64,
    pub dropped_sequence: u64,
}
```

Counters saturate. Sequence identifies allocation attempts in this stream; concurrent receive order can differ.
Loss creates gaps. It does not establish business journal order.
Diagnostic stages are observations, not durable recovery authority.
Use a per-operation collector with 32 fixed records. Its overflow increments `dropped_collector`.
No collector allocation scales with fanout or chain depth.

Internal recorder consumes receipt identity and optional existing `Cause`/`ClaimKey` for stable operation/root/parent/work tokens.
A separate claim token includes generation, so lease owners can be distinguished without principal names.
The work token excludes generation and remains stable after a new lease.
Parent references use the Work namespace. Operation/receipt, Root, Work and Claim are distinct identity domains. Lifecycle uses a separate session-bound domain.
Record stage facts only; do not serialize those raw inputs.
Expose `DiagnosticSink::operation(identity: &str, cause: Option<&Cause>, claim: Option<&ClaimKey>, attempt: u32) -> OperationRecord` internally.
`OperationRecord::stage(stage: DiagnosticStage, outcome: DiagnosticOutcome, elapsed_ns: Option<u64>, event_count: u32)` accumulates fixed facts.
`OperationRecord::publish(self)` sends them after guarded closures return.
`OperationRecord::timer(&self) -> Option<Instant>` and `elapsed_ns(&self) -> Option<u64>` skip timing after reader closure.
`elapsed_since(Instant) -> Option<u64>` uses checked local duration and integer conversion.
Use `DiagnosticSink::work(&WorkClaim) -> OperationRecord` for source mapping and external work; it derives the same root/parent/claim domains.
Do not publish from a destructor that can run while a gate remains held.

## Review focus

1. Lost acknowledgement must emit Unknown and later same-key receipt resolution, without inventing a rollback or repeating a mutation.
2. Source and child work must retain a root token across restart; rotated keys must change tokens without granting new authority.
3. Full queues and blocked exporters must not retain action permits or delay Runtime shutdown.
4. Receipt disclosure denial after owner change must emit no old owner, payload or raw idempotency identity.
5. Late completion after caller cancellation must describe owned work, not a fictitious cancelled commit.

All five conditions have tests assigned below. No reduction to RuntimeStatus or AI-only telemetry is permitted.

### Task 1: Admit dependency and stage the channel contract

- [ ] Refresh RustSec and check the actual resolved hmac/sha2/digest graph, licenses, required features and ROM MSRV.
- [ ] Record why hmac 0.12.1 matches digest 0.10; do not adopt newer digest merely because it is latest.
- [ ] Stage actual-adapter tests for queue overflow, reader disposal and source redaction in `tests/persistence/tests/diagnostics.rs`.
- [ ] After the coordinator releases builds, run `cargo test -p rom-storage-conformance --test diagnostics` and retain the initial missing-interface result.
- [ ] Implement `Diagnostics::bounded`, reader/sink and fixed types in the assigned named modules.
- [ ] Implement validated 1..=4096 capacity and independent loss counters. Full/closed enqueue errors never become action errors.
- [ ] Run focused diagnostics tests; require business state changes to succeed while the queue is full or closed.

### Task 2: Prove restart-stable redacted correlation

- [ ] Add internal HMAC-SHA256 vectors using fixed test keys and independently published expected results.
- [ ] Test length-delimited encoding and distinct receipt/work/root/parent domains.
- [ ] Add actual SQLite/redb lost-acknowledgement tests using the existing post-commit test-support fault.
- [ ] Require the first result to be Unknown; reopen; require the same receipt token and no second event.
- [ ] Add same-key/different-key session and rotation tests. Session changes do not alter stable root/work tokens.
- [ ] Implement HMAC through RustCrypto. Do not hand-write cryptography or expose the key through Debug/Serialize.
- [ ] Audit generated record output against raw principal, Resource ID, kind, payload and idempotency sentinels.

### Task 3: Instrument real command and work boundaries

- [ ] Instrument admission in `invoke_row`; transfer operation recorder into the owned I/O closure with its permit.
- [ ] Wrap `run` with an outer publication boundary; put existing logic in a guarded inner closure or named helper.
- [ ] Record authority/validation/execution/commit/receipt outcomes independently. Record Unknown without a success event claim.
- [ ] Record committed event count only when receipt commit is confirmed. Preserve no-op zero-event behavior.
- [ ] Instrument claimed reactions and mapper materialization with existing cause/claim fields.
- [ ] Instrument frozen-action receipt replay without reporting a new child mutation.
- [ ] Instrument notification authority, persisted DeliveryStarted, external callback outcome and durable DeliveryFinished separately.
- [ ] Keep queue publication outside gate-held sections. Never instrument with subscriber or callback calls.
- [ ] Run the actual two-adapter chain test. Require linked source, child and external work, with separate operation tokens.

### Task 4: Prove lifetime, authority and exporter isolation

- [ ] Add a test that cancels a caller after supervised work enters its real action callback; require eventual committed state and truthful terminal records.
- [ ] Add an owner-change test: a previously valid receipt remains durable, but unauthorized disclosure is denied without leaking prior fields.
- [ ] Add pending-chain reopen: first runtime materializes work; second runtime finishes it with unchanged key and new session.
- [ ] Add a host exporter that sleeps or panics after a real record; require actions and shutdown to remain independent.
- [ ] Drop reader; retain it closed through shutdown; prove database reopen and permit accounting still succeed.
- [ ] Use checked Instant durations. Test unavailable duration and count overflow in internal deterministic fixtures.
- [ ] Run `cargo test -p rom-storage-conformance --test diagnostics` and affected command/work suites.

### Task 5: Measure and admit the complete slice

- [ ] Run finite equal workloads with diagnostics disabled, enabled-undrained, enabled-drained and host-exported, on both adapters.
- [ ] Record p50/p95/p99, allocations, hashing cost, queue high-water, drop counts, RSS and net elapsed cost separately from DB time.
- [ ] Include ordinary success, replay, no-op, rejected requests, fanout and external attempt workloads.
- [ ] Record source identity, lockfile, compiler, seeds, commands and finite disk/memory/workload budgets.
- [ ] Compile an independent consumer from accepted public paths without transport features or a global subscriber.
- [ ] Run formatting, Clippy, workspace checks and `./scripts/check` under coordinator admission.
- [ ] Request independent review of redaction, post-gate publication, restart tokens and disabled overhead.
- [ ] Update operator documentation with host drainage, loss semantics, key rotation and bounded shutdown evidence.

## Preparation status

The research design is written and the coordinator selected its direction.
The coordinator retained the actual missing-interface RED (exit 101) in `/var/tmp/rom-010-diagnostics-initial-interface-red.log`.
At preparation time, the staged modules had not been compiled. See the integration update below for executed checks.
Existing source integration remains coordinator-owned until file ownership is reassigned.

## Consumer usability evidence

AP-UX-007/008/013/024 require understandable stage, correlation and recovery visibility, not only internal counters.
Exercise an installed consumer that drains the public API and distinguishes Unknown, recovered receipt, pending reaction and external acceptance.
Require bounded readable categories without raw IDs or payloads. Preserve all 33 tracked AP-UX groups; 030 and 032 are withdrawn.
Passing the native tests does not establish original Astral Plane adoption or human usability. Record those checks separately.

## Work integration update

The source freeze was released after the identity provider matrix completed.
The coordinator admitted the diagnostic module and resolved hmac 0.12.1 in the lockfile.
Native run 75413 compiled the public interface. Seven maintained cases passed; the chain case exposed missing Reaction observations.
The owned work modules now collect Reaction, Materialize, DeliveryStart, ExternalAttempt and DeliveryFinish observations.
They publish only after their guarded inner helper returns.
The recorder retains cumulative local time in `elapsed_ns`.
It records measured callback duration separately in `stage_elapsed_ns`. Neither value measures queue age or cross-restart elapsed time.

Native run 27901 passed the eight previous cases. The added external-Unknown fixture failed before its diagnostic assertions.
Its action omitted the existing revision after creating the Resource. The fixture now supplies `at_revision(1)`; no product correction was needed.
Native run 62592 passed the diagnostic unit cases, then reported Clippy unchanged-error observer warnings.
The observers now use `inspect_err` without changing results or suppressing warnings.
These corrections require a new native run. This update does not claim full release acceptance.

The ignored overhead fixture uses two real adapters, three rounds and three modes.
Each mode performs 64 creates and 512 retained-receipt replays.
It reports latency percentiles, wall time, exported records and loss counters.
The drained mode includes host drainage cost. These measurements do not cover allocations, RSS, production load or a network exporter.
The coordinator must retain source identity and the native execution log before reporting results.

The corrected maintained diagnostics run passed nine cases, with one overhead case ignored.
Run 33089 then passed Clippy, the explicit 18-row overhead trial and core regressions.
The research report records exact distributions, fixed execution order and measurement limits.
These results precede lifecycle additions and cannot admit later source by inference.

### Explicit shutdown waiter scope

Add `DiagnosticSink::shutdown_record()` with a separate Lifecycle HMAC domain and the supplied stream session.
These links identify lifecycle observations in one host stream. They do not identify unique waiters or exactly-once drain transitions.
Publish Started after the close lock is released. Publish a terminal category only after that waiter observes the existing drain condition.
Do not emit from `Drop` or add a diagnostic scheduler. Cancellation can leave Started without a terminal observation.
Use `RuntimeStatus` as the sampled state authority after cancellation.
The new actual-adapter test holds an accepted action, cancels the waiter, then verifies a later waiter and exactly two durable events.
The helper and test are staged for the coordinator's missing-behavior RED; lifecycle implementation remains coordinator-owned.

The first shutdown fixture run failed an unrelated Debug compile bound. The corrected fixture retained a genuine missing-Started RED.
The coordinator's lifecycle implementation now publishes outside lifecycle locks and retains explicit waiter semantics.
The post-change maintained suite passed ten cases, with one overhead case ignored. The native Clippy check completed successfully.
The research report contains evidence paths and the precise cancellation boundary.
Final-source performance, the full local verifier, independent review and packaged consumer evidence remain separate admission requirements.
