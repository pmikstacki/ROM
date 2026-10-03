# Maintained durable reactions and bounded storage

Date: 2026-10-02. Original baseline `d228c13`, storage stage `8d7cd9c`, runtime stage `aab1062`. The integration commit containing this updated report applies that runtime to main `3e1354d`, including native User/provider identity, field permissions and HTTP/journal support. This package implements part of the integrated MVP. It does not establish completion of the entire MVP or choose a production database.

## Author-facing result

A host registers typed reactions once alongside Resource definitions:

```rust,ignore
let service = Actor::trusted("host", "copy")
    .with_kind(PrincipalKind::Service);
let runtime = Runtime::builder()
    .resource(Source::definition().policy(source_policy).allow_all_fields())
    .resource(Mirror::definition().policy(mirror_policy).allow_all_fields().action(SET))
    .reaction(Reaction::new("copy-enabled", 1, service, SET, |source| {
        Ok(vec![Target::new("mirror", source.value.as_ref().unwrap().enabled)])
    }).depends_on(Source::enabled_field()))
    .build(storage, shared_rayon_pool)?;
let worker = runtime.start_reactions()?;
// Ordinary typed execute/invoke commits automatically enqueue matching work.
runtime.shutdown().await?;
worker.join().await?;
```

`Reaction<S,T,I>` couples a source type, typed target action and typed mapping function. The callback sees an immutable source snapshot and returns existing target IDs and inputs. ROM supplies routing, revision reads, identity, persisted mapping, claim/recovery, authorization, mutation submission and supervision. The external `tests/persistence/tests/runtime_reactions.rs` consumer demonstrates distinct Source and Mirror types through this path on both SQLite and redb, without Resource-specific repositories, controllers or workers.

`Builder::reaction_limits` configures one persisted work policy. `Runtime::process_reactions(max_steps)` offers a finite supervised batch for host scheduling and deterministic tests. `start_reactions` starts one generic runtime-owned Tokio loop; its polling interval is 100 ms and each batch is at most 32 claims and the configured total-work limit. A second loop is rejected. Shutdown waits for the loop and any accepted blocking batch; canceling an observer cannot cancel accepted work.

## Commit and recovery semantics

A changed Resource commit atomically records state, event, idempotency receipt, ordinary effect intents and eligible reaction obligations. Semantic no-ops record their receipt but produce no reaction work. Explicit source-field dependencies compare actual before/after values; omitted dependencies conservatively route every semantic change. Invalid dependency names reject registration.

Mapping and action execution are separate durable leased steps. Mapping runs on the host-owned Rayon pool. It rechecks current service authority and complete historical/current source field authorization. It reads target revisions and persists the complete generic Invocation. Its expected revision, input and idempotency remain frozen on retry. A target revision conflict is an inspectable stop; this implementation never silently changes the command meaning to win a retry.

The target action passes through the shared `run` mutation pipeline. `Bundle::completed_work` fences the claim generation and lease and marks the work Done in the same transaction as the target state, receipt and any next reaction obligations. Already committed upstream changes remain committed when downstream work fails. An actual lost target acknowledgment therefore reopens with both target state and work completion present. The recovery path also resolves a durable action receipt before attempting another execution. Compensations are not implicit.

Leases carry monotonically increasing generations. After restart, expired work is claimable. An old claim cannot materialize children or commit a target. Claiming charges both the step attempt and root work budget. A budget-exhausted recovery claim can resolve a receipt but cannot execute a new action. Resource revisits are allowed; convergence is demonstrated, and real oscillation stops at the depth budget.

Service identities are explicitly `PrincipalKind::Service` and come from host registration. Persisted work stores the stable principal key, reaction name and version, not a deserializable Actor or an event-author credential. The current registered service authority is checked on every step. Changed registration version/service identity stops old work. Denial, conflict, invalid input, missing resources, excessive fanout and callback panic are recorded terminal causes. Transient storage/application NotCommitted failures retry with exponential delay under the budgets. Unknown acknowledgments leave the lease for recovery. An actual redb uncertain commit requires reopening its poisoned adapter.

## Finite profile and storage format

Defaults are provisional host-configurable policy, not final product retry targets:

| Limit | Default |
| --- | ---: |
| Maximum causal Resource-transition depth | 16 |
| Root work attempts, including mapping | 256 |
| Attempts per step | 3 |
| Chain age | 3,600 seconds |
| Base retry delay / lease duration | 1 / 30 seconds |
| Children per callback; matching registrations per commit | 16 |
| Persisted work records / encoded ledger bytes | 1,024 / 1 MiB |
| Durable receipts / effect intents | 100,000 / 100,000 |
| Retained journal events / encoded event bytes | 1,024 / 4 MiB |

SQLite/redb `open_with_limits` accepts `StorageLimits`; a reopen must match persisted storage limits. The work ledger similarly rejects changing persisted reaction policy. Capacity exhaustion rejects an upstream bundle atomically, rather than committing state without its obligations. Done and stopped work records remain retained, so this finite profile eventually needs host maintenance or an explicit archival design; no unsafe automatic receipt/work eviction was added.

Both adapters now use **format 2** and reject format 1 explicitly. No production data migration is implied. Native bundle transactions also persist journal generation, global commit position and retention floor. Journal pages count whole encoded `JournalEvent` bytes, preserve per-resource ordering, and return HistoryGap for wrong generation/kind, future position or a cursor below retained history. Journal retirement removes the corresponding adapter event rows atomically. Transport owns the authorized public journal and explicit resynchronization interface.

The shared `StorageState` and `WorkLedger` avoid separate SQLite/redb claim semantics. They serialize and clone the entire bounded metadata structure during updates. This produces substantial write amplification and CPU/allocation cost as the journal/work ledger fills. It is a small correctness-oriented profile, not an indexed high-throughput outbox. Even an idle claim currently persists the metadata. Indexing records independently and eliminating idle writes are prioritized follow-up work before throughput-oriented deployment.

## Verification evidence

Native `rom-dev`, Rust 1.99.0, two Cargo jobs, worktree-local target. `./scripts/check` passes strict OpenSpec validation, formatting, warning-free clippy, workspace tests, rustdoc warnings, driver-free/no-derive core checks, executable consumer and five compile-fail fixtures. The package adds five ledger/storage tests and nine runtime reaction tests. Existing subprocess crash tests now include the new sixth metadata-write checkpoint as well as pre/postcommit boundaries. SQLite initialization persists its format and initial metadata in one transaction.

Focused evidence includes actual database reopen before retry, target commit acknowledgment loss followed by reopen, whole-bundle rollback at precommit including obligations/journal/counters, saturated-ledger rejection with no upstream state or receipt, generation-fenced leases, finite work resolution, service revocation, callback fanout rejection, no-op/dependency suppression, converging revisits, bounded oscillation, canceled batch drain with durable remaining work, and single-worker shutdown. Storage inspection is a trusted administrative surface, not a user transport endpoint.

The actual-adapter two-resource test counts calls from immediately before source creation through complete downstream processing, excluding target seeding and inspection:

| Adapter | load | receipt | commit | reaction_update |
| --- | ---: | ---: | ---: | ---: |
| SQLite | 9 | 5 | 2 | 4 |
| redb | 9 | 5 | 2 | 4 |

These are real generic Storage calls, separate from the earlier simulation, with one mapping step and one action step. They demonstrate equivalent work, not database performance. No elapsed-time comparison is claimed under concurrent builds.

## Limits and next work

This profile has one ROM writer owner per database. Native callbacks are trusted, pure and bounded; ROM cannot forcibly terminate a blocking or infinite native callback, so hard deadlines require process isolation. External effect delivery is not implemented by this reaction worker; ordinary Intent entries remain atomically recorded. Exactly-once external side effects are not claimed.

Typed reactions currently target custom actions on existing Resources. Callback source tombstones stop with Denied until an explicit deleted-source authorization profile exists. Generic create/delete chain targets, bulk target revision selection, operator retry/requeue, compensation, dead-letter export, randomized retry jitter, schema/definition migration and independent indexed work records remain follow-up designs. Definition versioning is host responsibility; changing a function while reusing its version cannot be detected automatically. Private causal metadata is attached below public Invocation and cannot be supplied through that wire shape.

The author API still requires stable reaction name/version, explicit service Actor, source dependencies and a mapper returning IDs. That is business mapping rather than per-Resource infrastructure, but mapping errors and missing targets stop a whole mapping step; per-child partial mapping policy is not yet offered. Parallel reactions targeting the same revision can conflict by design.

The coordinator owns backup/restore and archive exports; these were explicitly handed back. The transport agent owns journal-head resynchronization and public auth/projection; the authority agent owns durable identity-provider mapping. Fresh independent review and integrated main verification are still required. No OpenSpec task is marked complete by this report. Engineering estimates for replacing serialized metadata with indexed queues should remain uncertain until contention and near-capacity benchmarks exist; correctness review and format migration dominate that follow-up.

## Field and identity integration check

The initial merge reproduced a field leak: a service denied the source title field still reached the mapping callback under row-level checks. A focused regression failed with one callback invocation instead of zero. Both mapping checkpoints now call the core complete-view authorization seam for the historical snapshot and authoritative current Resource. The regression covers a hidden historical field later revealed, and a previously visible field hidden in the current version, on both databases. All four cases stop with Denied before callback execution. The second checkpoint also prevents materializing work after authorization changes during mapping. Existing fixtures now declare their field grants explicitly; default deny is preserved.

The projected invocation split, native IdentityGate/establish_actor path, shape registration checks and explicit journal-head methods remain intact. Typed execute still requires a complete authorized return view; transport uses projected outcomes. The integrated source-create path performs one additional complete-view load, increasing measured loads from eight to nine; other call counts stay unchanged. No query semantics are changed by this integration.
