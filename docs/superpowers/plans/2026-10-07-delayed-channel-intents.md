# Durable delayed channel intents

Status: implementation plan, not executed scheduling acceptance.

Goal: automatically resume confirmed transient waits through existing ROM Work, without an application scheduler.

## Investigation and selection

The source already stores WorkRecord.due and skips pending work until due <= Clock::now.
WorkLedger::Finish currently applies fixed exponential retry delay. Channel::intent cannot select a first eligible time.
Builder::build does not start workers. Runtime::start_work/start_reactions starts the existing generic worker explicitly.

Selected public seam: Channel::intent_at(payload, not_before_unix_seconds). Existing intent(payload) remains immediately eligible.
Freeze an optional not_before field in Intent and PendingWork. Initialize due to max(cause.started_at, not_before).
Preserve the frozen floor through retry, operator recovery and backup restore. No second scheduler or per-run timer is introduced.
A confirmed provider refusal may commit Waiting and a delayed next intent atomically. Unknown external outcomes still require reconciliation.

The [Tokio timer documentation](https://docs.rs/tokio/latest/tokio/time/fn.sleep.html) describes cancellable in-memory waiting.
It does not supply durable scheduling across process restart. The persisted ledger remains authoritative.
[AWS retry guidance](https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/) distinguishes bounded retries, timeouts, and overload amplification.
ROM keeps one retry owner. Provider adapters must not add hidden retries. Jitter may extend a confirmed wait, never shorten its floor.

## Invariants

- No claim, delivery-start record, provider call or resource success event occurs before the frozen floor.
- The mutation and its delayed intent commit atomically. Rejected or replayed mutations do not enqueue another intent.
- Stored intent identity includes frozen timing data in equality checks. Changed timing under the same mutation key cannot overwrite it.
- Current service authorization, definition version, lease fencing and reconciliation remain decisive at delivery.
- Delays cannot raise max-age, max-attempt, root-work, fanout, record or byte budgets.
- A deadline before eligibility causes bounded terminal handling, not an early provider call.
- Unix seconds come from the trusted runtime clock. Millisecond provider Retry-After rounds upward with checked arithmetic.
- Restoring or explicitly retrying work cannot reset due below the frozen floor. Cancellation does not imply undoing an external effect.

## Compatibility decision before implementation

Current native/archive format is 8. An old reader could ignore new optional fields and erase the scheduling floor.
Inspect rom-backup, SQLite and redb format admission and explicit upgrade paths before changing persisted models.
Select a new format marker when required; old binaries must reject the new database before mutation.
Upgrade populated format-8 data into a fresh destination. Preserve original source bytes, identities, receipts, work and replay boundaries.
Do not claim binary rollback can read newly scheduled data. Document data restore separately from image rollback.
Public struct literal compatibility changes must be recorded for the pre-1.0 upgrade. Preserve supported method paths.

## Owned implementation and test sequence

Assign source ownership before edits. The implementation spans channel intentions, Work models/ledger/control/restore and adapter format conformance.
Keep unrelated captured-validator behavior and evidence unchanged. No AI-specific types belong in these modules.

1. Add red tests for early claim, due-time claim, exact replay and changed delayed intent on SQLite and redb.
2. Add red restart, expired-lease, operator retry and archive restore cases with original frozen floor.
3. Add negatives for stale claims, denied service, changed channel version, overflow, max-age and exhausted budgets.
4. Implement the small scheduling seam and format admission/upgrade required by the selected compatibility contract.
5. Run shared adapter conformance and source-preserving populated format-8 upgrade tests.
6. Run a worker-driven bounded integration test: build/bind before start, commit delayed intent, restart, advance trusted fixture clock, observe one delivery.
7. Assert cancel/unknown reconciliation preserves existing effect semantics; no app timer or polling loop is allowed.
8. Run affected compile, backup/upgrade/retention and full local checks through the declared toolchain before integration.

Use existing target with jobs=2 and incremental disabled after coordinator allocation. No paid provider or production data is involved.
Record command, source/lock/compiler identity, failures, finite bounds and actual database/work observations.
The existing worker checks idle state every 100ms. Its eligibility check prevents early delivery, but this is not a real-time timing guarantee.

## AI composition boundary

FlowBuilder binds callbacks to its runtime before starting the worker. Recovered work cannot dispatch against an unattached host.
Confirmed 429 commits one delayed next tick and acknowledges the current tick. A duplicate callback resolves the checkpoint before dispatch.
An unknown request or uncertain provider accounting never creates a fresh retry merely because its delay elapsed.
Automatic continuation is part of the public ergonomics goal; manual operator resume is reserved for unresolved evidence or policy intervention.
