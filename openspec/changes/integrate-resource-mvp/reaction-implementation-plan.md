# Durable reaction package plan

Source baseline: maintained `d228c13`; fixed semantics from `reaction-chain-results.md` and the integrated MVP specification. Native Rust 1.99, shared Tokio/Rayon, driver-free core; one runtime owns writes. Work is infrastructure belonging to Resource transitions, not another managed domain model.

## Scope and public authoring

Add typed source-to-target reaction registrations with stable name/version, explicit service Actor, generated source-field dependencies and a pure callback mapping a committed source snapshot to bounded target action inputs. The framework freezes mapped commands, including expected revision and stable step idempotency, before submitting them through the same generic action pipeline. No callback repository or per-kind worker is required. Unknown dependencies invalidate conservatively; no-op commits schedule no work.

The generic invocation package supplies `Invocation`, tagged `Operation`, and `Runtime::invoke`; reaction execution adds private causal context below that interface. Transport owns journal protocol types; this package implements agreed storage hooks in SQLite/redb as part of one explicit format update.

## Task 1 — Durable work contract and shared ledger

Create bounded serializable work/causal values and one deterministic ledger transition implementation. Storage transactions load, mutate, and save this ledger. Concrete adapters must not duplicate claim, fencing, retry, or budget semantics. Persist root/depth/age policy, total attempts, lease generations, mapped command meaning and inspectable terminal causes. New obligations enter the same transaction as state/event/receipt/effects. Ledger count and serialized-byte caps refuse additional obligations before commit rather than dropping them.

Write shared real-database tests for atomic work insertion, rollback, reopen, lease expiry and stale claim fencing, fixed input across retry and bounded ledger capacity. Introduce a new storage format version; old/foreign formats fail explicitly. No implicit migration.

## Task 2 — Typed routing and bounded worker

Register typed reactions through Builder, validate resource/action/dependency identities and freeze registration names/versions. Commit routing compares actual changed fields and creates only eligible obligations. A bounded supervised worker claims incrementally and authorizes source/target as the configured service. It computes callback proposals on the shared Rayon pool. Before execution, it persists mapped steps. It invokes the shared action path with inherited causal metadata. Cancellation cannot drop accepted work. Expired claims recover after restart. Permanent policy/input/conflict failures stop visibly. Transient/unresolved work retries under finite attempt/work/age budgets. Upstream commits never roll back because a downstream action fails.

Write end-to-end tests for A→B, no-op/dependency suppression, downstream transient/permanent failures, current service revocation, actual database reopen before execution and between downstream commit/ack, cancellation/recovery, converging revisits and oscillation/fan-out limits.

## Task 3 — Adapter integration, documentation and verification

Implement the transport package's journal hooks with persisted commit ordering/generation under the same native transaction. Coordinate finite receipt/journal capacity and backup boundaries with the coordinator; explicitly hand back anything not implemented. Run shared tests on both SQLite and redb, existing full native verifier, core no-default/dependency gates and packaged examples. Record source-pinned results, exact default limits and unsupported profiles. A coordinator-owned independent review precedes main integration; no publication or implicit production migration occurs.

## Provisional bounded profile

Callbacks map immutable source snapshots to custom actions on existing target Resources. Mapping and action steps are independently durable and fenced. Expected revision and canonical input freeze on materialization. A later revision conflict is an inspectable terminal condition, not a retry with a changed identity. Ordering is per claimed obligation with stable action identity, not a global event-processing guarantee. Retry intervals are deterministic bounded host policy; jitter, operator requeue, compensation, schema migration and distributed writers are separate later profiles.

The worker may use a bounded serialized ledger for this finite MVP profile. This deliberately centralizes budget correctness and trades scale for a small tested seam; it must reject its configured capacity rather than becoming an unbounded metadata blob.
