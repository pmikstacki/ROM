# ROM 0.1.0 operations gap audit

Date: 2026-10-08. Status: source audit and proposed executable cases.

This audit covers R8, R9, R10, and R11 in the [consumer research plan](rom-0.1.0-consumer-research-plan.md).
It does not change their scope or mark a release gate complete.
No build, server, database operation, process change, or disk scan ran during this audit.
Only this report was written.

## Evidence identity

Inspected HEAD: `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`.
The worktree contains candidate changes. HEAD alone does not identify their complete source set.
The following hashes identify selected files read in this audit.

| File | SHA-256 |
| --- | --- |
| `Cargo.lock` | `f6ac31a111d67e8c4d1cc8bc0438ac0cf02bd87e851c78c86d6e2756b1241bf3` |
| `crates/rom/src/execution/lifecycle.rs` | `f5134a908cbb8800cf0732c540d9ee9dd48ddc69fe0d78b29d1d17d0c5930465` |
| `crates/rom-ai/src/telemetry.rs` | `e59d49f730798ddaa599d1fb0a084bd606dd07b168e633525fd3da7cf31d45e5` |
| `tests/persistence/tests/delayed_upgrade.rs` | `76e465877068338eec2f344379eb2bd0d70f0810c2a52882f4ab90744469c861` |
| `demo/src/upgrade.rs` | `32606d93c2670a55a8e2a9ce89086c4c0256f8f527c0233da2502d28b57d434f` |

Historical result documents describe executed checks at their stated source identities.
This audit read those reports and relevant source. It did not reproduce those checks or inspect every historical log.
Current source conformance still needs current execution evidence.

## Requirement status

| Requirement | Existing source or retained evidence | Missing release proof |
| --- | --- | --- |
| R8: complete application recovery | Both adapters expose `backup_to` and fresh-destination `restore_from`. Shared tests cover receipts, tombstones, journal fences, work, and unknown delivery. | A fresh application host must recover database, actual blob bytes, configuration references, identity configuration, and unfinished work together. Measured RPO/RTO are absent from the inspected records. |
| R9: diagnosis and correlation | `Runtime::status` exposes intake, owned work, permits, and registration counts. Operator contracts expose authorized work. AI has an optional `FlowObserver`. | No inspected core source provides the complete action → receipt → event → reaction → external-attempt correlation with timing. Bounded-label, redaction, exporter-failure, and overhead proof remain separate requirements. |
| R10: supported workload | Admission, subscriptions, snapshot sizes, command sizes, blob transfer, and work already have finite limits. Maintained suites inject specific interruptions and authority changes. | No inspected record establishes a current mixed whole-host workload with latency distributions, RSS, queue age, disk growth, uploads, streams, and work. Fault cases must run against that deployment profile. |
| R11: safe populated upgrade | Native/archive conversion and schema migration exist. Delayed upgrade tests preserve receipts, operator history, pending work, unknown outcomes, and fences. | Actual accepted 0.0.3 source population, custom-codec historical replay, matching Rust/Studio startup, and separate rollback/restore acceptance still need proof. |

## R8: whole-application restore is the first operational gap

The database archive contains private logical data and work. It explicitly excludes external blob bytes and external delivery effects.
The archive cannot supply application functions, provider credentials, or a deployment image.
These exclusions are correct boundaries, not a reason to narrow R8.

`demo/src/upgrade.rs::recover` reuses the original `objects` directory.
It proves that retained attachment bytes work with a migrated and restored database.
It does not prove attachment restoration when the original host or directory is unavailable.
The reference upgrade report states this limit explicitly.

Use the existing database and blob APIs through a host-owned application manifest.
Keep this manifest outside the driver-free core.
Record matching source/assets, adapter format, archive digest, object keys/digests/sizes, configuration references, and secret-reference names.
Do not put secret values or private Resource contents in the public report.
A trusted backup directory still contains private archive and object bytes.

### Next bounded executable case

1. Use an exclusive source directory and a different exclusive recovery directory for each adapter.
2. Populate synthetic Resources, retained receipts, tombstones, attachments, and pending/reconciling/stopped work.
3. Quiesce the original host and workers before the coordinated snapshot.
4. Create the database archive through the public adapter method.
5. Copy every declared attachment into the private backup directory.
6. Verify each copied object against its recorded digest and size.
7. Record the last acknowledged mutation identity and snapshot time.
8. Make ten later synthetic mutations after the backup boundary.
9. Stop the original owner before recovery.
10. Restore only from the backup into fresh database and object destinations.
11. Start matching application code with restored configuration references and separately provisioned isolated identity.
12. Verify login, current authority, exact receipt replay, attachments, tombstones, journal fences, and resumed work.
13. Count acknowledged post-backup changes absent from the recovered host.
14. Measure time from recovery start to the required healthy application checks.

Repeat on SQLite and redb. Preserve the source, manifest, archive, restored files, and failure evidence.
Reject an existing destination, an incomplete object inventory, a changed object digest, and an unresolved required configuration reference.
The recovery process must not read the original object directory.
No generation fence permits the old host to remain active against its original database or receiver.

The [core investigation](rom-0.1.0-core-investigation.md) proposes 1,000 Resources, 100 receipts, 20 tombstones, and 20 attachments.
Those values are experiment envelopes, not accepted service targets.
Declare RPO/RTO targets separately from measured results. A successful drill does not certify power-loss recovery.

## R9: reuse status, but add lifecycle correlation

`RuntimeStatus` intentionally contains no actor, Resource identity, or payload.
It describes current capacity. It cannot explain the history of one stuck operation by itself.

AI `FlowObservation` contains an opaque run ID, step, attempt, elapsed milliseconds, and a category.
Its constructor bounds identifier length, step, and attempt.
`observe_safely` catches an observer panic. It cannot preempt a blocking callback.
This AI-only interface does not establish tracing across ordinary Resource actions and reactions.

The exact source seams already exist:

| Stage | Named source module |
| --- | --- |
| Admission and drain | `crates/rom/src/execution/lifecycle.rs` |
| Mutation, commit, and receipt replay | `crates/rom/src/execution/command.rs` |
| Reaction claim and result | `crates/rom/src/reactions/worker.rs` |
| Delivery execution and supervision | `crates/rom/src/channels/execution.rs`, `supervision.rs` |
| AI attempts | `crates/rom-ai/src/flow/worker.rs`, `telemetry.rs` |

Use a host-installed observer or subscriber. Do not install a process-global subscriber inside ROM.
Keep correlation IDs in access-controlled events, not metric labels.
Do not emit credentials, Resource values, tool arguments, evidence references, or full request fingerprints.

Add synthetic secret sentinels to inputs and provider responses. Assert that exported bytes contain none of them.
Compare disabled instrumentation with a bounded recording observer under the same workload.
Test a panic, exporter error, and a full observer queue. The mutation result and durable state must retain their normal semantics.
A permanently blocking callback needs an explicit host isolation policy; panic handling alone does not solve it.

## R10: measure the composed host

Do not derive a workload capacity claim from unit-test duration or query microbenchmarks.
The current admission limits and lifecycle tests are useful building blocks.
They do not measure simultaneous HTTP, streams, blob transfer, database growth, and durable work.

Start with the investigation's explicit profile: 10,000 Resources, 32 clients, 32 subscriptions, and queries bounded to 50 rows.
Use two minutes warm-up, ten minutes measured traffic, and two minutes drain/recovery for each adapter.
Record request counts, operation mix, attachment sizes, work rate, deadlines, CPU/RSS limits, and disk limits before each run.
Use one deployment owner and one adapter run at a time.

Report p50/p95/p99 for reads, commits, receipt replay, caller waiting, stream lag, and delivery separately.
Record rejected/overloaded counts, maximum RSS, queue age, native/WAL growth, and obligations retained at drain.
Use finite request and output budgets in addition to an initial free-space check.
Do not treat shared-host free space as the tested process's measured allocation.

Run separate bounded fault cases for a slow subscriber, unavailable receiver, exhausted private disk image, and process interruption.
Inject disk exhaustion only inside a dedicated bounded image. Do not fill the shared root filesystem.
Verify recovery identities, receipt/event counts, bounded memory/admission, and retained unknown outcomes after each fault.
A slow subscriber must not retain unbounded server queues or block shutdown indefinitely.
Restart must not reset work attempts, age, or provider reservations.

These proposed counts remain provisional until the deployment profile is stated in the support matrix.
Measured failure limits can inform that profile. They must not silently remove the required fault cases.

## R11: separate four compatibility claims

1. Current semantic format fixtures establish converter behavior.
2. An accepted old writer establishes real historical population.
3. An old reader refusal establishes explicit downgrade rejection.
4. Application rollback or retained-backup recovery establishes the operator's rollback procedure.

`delayed_upgrade.rs` currently populates the current adapter and then writes the predecessor marker.
It deliberately omits new scheduling fields and exercises old-layout conversion.
This fixture does not replace an actual accepted 0.0.3 writer.
The coordinator owns actual old-reader rejection against copied current fixtures. Do not duplicate that work.

Next, build a bounded old-source population helper from accepted `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
Use only APIs and codecs available in that source set.
Populate custom fields, original receipts, tombstones, pending work, and an unresolved external outcome.
Close it, preserve its exact bytes and image identity, and convert into a fresh candidate destination.
Start matching Rust/Studio code. Replay the old operation through its retained codec under current authority.
Verify work payloads, attempts, budgets, generation fences, and no duplicate publication.

Attempt old application startup only where its data contract permits it.
Otherwise, test recovery into a fresh destination from the retained old backup.
Never present executable rollback over an incompatible database as a successful rollback.

The current `rom-backup` model declares archive 7/native 9.
Its README still describes archive 6/native 8 and conversion only through native 7.
Update this documentation after current compatibility acceptance. Do not use the stale README as candidate format evidence.

## Recommended execution order

R8's fresh-directory restore is the highest-priority composed-host proof.
Prepare the actual old-writer fixture in parallel when native build ownership permits it.
Implement R9 correlation before R10 if load diagnosis depends on those observations.
Run R10 after the host, restore procedure, and source identity are frozen.
Finish with matching extracted packages, full verification, independent review, deployment acceptance, and release artifacts.

These operations cases supplement current UI, AI, identity, and consumer work.
They do not reduce R1–R14 or declare 0.1.0 ready.

## Writing review

Technical identifiers and stated requirement boundaries are preserved.
Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary.
Full compliance requires verification against the official standard.
