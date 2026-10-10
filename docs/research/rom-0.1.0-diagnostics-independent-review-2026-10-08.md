# ROM 0.1.0: independent diagnostics review

Date: 2026-10-08. Scope: R9 diagnostics, command execution, durable work, notification delivery, and shutdown.

This review inspected source and existing native logs. It ran no native builds and changed no product source.
The review follows `AGENTS.md`, `docs/quality.md`, and `docs/writing.md`.
The report uses the ASD-STE100 writing guide. It does not certify dictionary compliance.

## Result

The reviewed implementation has two diagnostic accuracy defects. These defects must be resolved before R9 admission.
The review found no raw-payload disclosure or blocking exporter call in the inspected production paths.
This result does not establish completion of R9 or release 0.1.0.

| Priority | Location | Finding | Correction and proof |
| --- | --- | --- | --- |
| P2 | `crates/rom/src/execution/command.rs:108`, `:148`, `:165` | A non-authorization failure can inherit the last Authorization stage. | Add explicit definition-resolution and receipt/storage-read observations. Test their failures without inventing authorization failures. |
| P2 | `crates/rom/src/reactions/claims.rs:143`, `:157`; `crates/rom/src/diagnostics/types.rs:66` | The public callback-duration description excludes work that the measured interval includes. | Describe the interval as queue, mapper execution, and return transit; or measure callback execution separately. Test the selected contract. |

### Failure stage attribution

`run_inner` records Authorization/Succeeded before registry lookup. An unregistered kind then returns `Error::Unregistered` without another stage.
`run_observed` uses the last stage for every error other than Denied. This path produces Authorization/Invalid despite successful authorization.

The first receipt lookup and its Resource load have the same problem. A storage failure can produce Authorization/Unknown.
Later receipt and state loads can inherit Validation or Execution. These categories describe the previous observation, not necessarily the failed operation.

Use explicit boundaries for definition resolution, receipt resolution, and storage reads. Keep categories fixed and payload-free.
Do not report every storage error as a failed commit. Commit/Unknown must continue to mean the actual commit response.

Add maintained cases for an authorized unregistered kind, receipt lookup failure, and current-row load failure.
Each case must retain the original business error. It must emit no false Authorization failure or Commit result.
Run adapter-independent taxonomy cases and applicable fault cases on SQLite and redb.

### Mapper interval

The Reaction timer starts before `pool.spawn`. It stops after `receive.recv` returns.
The interval therefore includes Rayon queue wait, mapper execution, and return transit.
`stage_elapsed_ns` currently describes a completed mapper or external callback duration. That description can mislead operators when the pool is busy.

A truthful interval description is sufficient if the contract requires the whole supervised stage interval.
If callback-only timing is needed, capture its start and end inside the Rayon callback.
Keep queue wait separate. Do not subtract cumulative observations and claim exact callback time.

Add a held-pool case if separate timings are implemented. Otherwise, update the API description and design report consistently.
The existing tests prove that timing is present. They do not prove callback-only timing.

## Source boundaries inspected

- `crates/rom/src/diagnostics/**`
- `crates/rom/src/execution/{diagnostic_record,command,lifecycle,builder,state}.rs`
- `crates/rom/src/reactions/claims.rs`
- `crates/rom/src/channels/execution.rs`
- `tests/persistence/tests/diagnostics.rs`
- `crates/rom/Cargo.toml` and the hmac entry in `Cargo.lock`
- `docs/research/rom-0.1.0-core-observability-design-2026-10-08.md`

The core files were shared working-tree sources. This review did not freeze them or prove that their bytes match compiled artifacts.
Line references identify the inspected candidate. Subsequent corrections need another source comparison and affected checks.

## Properties supported by source inspection

The event schema contains fixed tokens, enums, counters, and optional durations. It has no application string or Resource value field.
`DiagnosticKey` has no Debug or Serialize implementation. The key type makes no allocator-erasure guarantee.
HMAC inputs have a version prefix, explicit component lengths, and separate Operation, Root, Work, Claim, and Lifecycle domains.
Parent tokens use the Work namespace. Claim tokens include generation; work tokens do not.

The collector holds at most 32 records. Queue capacity is validated at 1 through 4096 records.
Publication uses `try_send`. Full, closed, collector overflow, and sequence exhaustion have separate saturating counters.
Sequence identifies allocation order. Source documentation does not promise receive order or business journal order.

Collectors publish explicitly after guarded helpers return. Their Drop implementation does not publish or call host code.
Command and work collectors retain no Runtime or database handle. The host reader does not participate in Runtime drainage.
Disabled diagnostics skips collector construction. A closed reader skips later token construction and local timer starts.

Commit/Unknown is distinct from success. A saved DeliveryFinished status does not convert an external Unknown into delivery success.
Caller cancellation does not release accepted execution ownership. Shutdown observations describe the current waiter, not a durable lifecycle journal.
Cancellation can leave Shutdown/Started without a terminal observation. `RuntimeStatus` remains the sampled state authority.

The hmac lock entry is version 0.12.1 with digest 0.10.7.
Its checksum matches the selected archive: `6c49c37c09c17a53d937dfbb742eb3a961d65a994e6bcdcf37e7399d0cc8ab5e`.
This match does not replace the full dependency advisory, license, feature, and MSRV gate.

## Existing native evidence inspected

The paths below are host-visible native-container logs. This reviewer read them directly.

| Log under `/var/lib/nixos-containers/rom-dev/var/tmp/` | Observed result | SHA-256 |
| --- | --- | --- |
| `rom-010-diagnostics-shutdown-green.log` | 10 maintained cases passed; one overhead case ignored. Applicable cases loop over both adapters. | `3aa1dc9609f96f0eb63b3bc49c8f3a292d5d8fcb0109084314bb620aa338b76a` |
| `rom-010-diagnostics-shutdown-clippy.log` | Clippy reached successful completion without listed warnings. | `b13304a094aaad9d480de4bc16f2fd8671fcf313c7356495f17a73dfe3335bb8` |
| `rom-010-diagnostics-shutdown-core-regressions.log` | 115 unit cases passed, six integration suites passed, and two doctests passed. | `79b92e62ee5920a0a9331e563579617e09f67f67001506d4ecb83b15aed28c2e` |
| `rom-010-diagnostics-overhead.log` | 18 measurement rows and one passing explicit ignored-case invocation. This run preceded shutdown integration. | `b20bc4c47987081ec31edc88cd42777fedfbf231c2835523c82e13f701e868b4` |

The maintained suite covers commit acknowledgement loss, reopen, correlation, redaction, reader loss, caller cancellation, host failure, and external Unknown.
The inspected unit log also includes HMAC vectors, domain boundaries, claim generations, sequence exhaustion, collector limits, and concurrent allocation uniqueness.
The sentinel checks inspect serialized diagnostic events. They do not establish redaction in a production network exporter or its logs.

Each overhead round has 64 creates and 512 receipt replays. There are three rounds for each adapter and mode.
Each drained round exports 3,328 records without loss. Each capacity-one undrained round enqueues one and drops 3,327 records.
Disabled, drained, and undrained modes preserve the business results in this fixture.
The run does not measure allocations, RSS, network exporter cost, production concurrency, or a reaction-heavy workload.
The fixed mode order and redb tail prevent a portable overhead claim. These measurements do not describe the post-shutdown source candidate.

## Remaining R9 admission evidence

Resolve the two findings and run their regression cases. Retain the failing cases and their corrected results.
Run the focused diagnostics suite, affected core suites, and Clippy after the correction.
Run the full local verifier on the final combined source. The prior full verifier predates these changes.

Run an installed public consumer that drains diagnostics through the host boundary. No such example was found in the inspected example paths.
Verify exporter output redaction and finite disposal there. Record the compiled source, lockfile, compiler, commands, and results.

Either complete the broader overhead plan or explicitly record its remaining limits at admission.
The current measurements cannot support claims about allocations, memory, reaction-heavy load, or a network exporter.
Preserve all existing logs, failure evidence, and historical source candidates.

## Corrective review: definition and read failures

This follow-up inspected the corrected source and three new native logs. It ran no native commands and changed no product source.
The original unregistered-kind path now records Resolve/Invalid after Authorization/Succeeded.
The maintained RED failed because the old path attributed this error to Authorization. The failure remains preserved.

The new `diagnostic_record::load` helper records StorageRead before each observed current-row load.
Receipt/Started precedes the exact retry lookup. The injected failure cases now report StorageRead/Unknown or Receipt/Unknown, as appropriate.
The wrapper forwards `acquire_owner` to the actual SQLite or redb adapter. Its normal operations use that database.
The cases retain `Error::Storage`, assert an empty Resource snapshot, and inspect serialized records for the existing secret sentinels.

The public `stage_elapsed_ns` description now states a supervised stage interval. It explicitly includes Reaction queue and return transit.
This fixes the callback-only implication in the public API. The design report's earlier callback-interval wording still needs the same clarification.

| Log under `/var/lib/nixos-containers/rom-dev/var/tmp/` | Observed result | SHA-256 |
| --- | --- | --- |
| `rom-010-diagnostics-resolution-red.log` | Intended unregistered-kind attribution failure; one failed case. | `9c7d9e13557b0906d860542e682942e9b17fc1bf75200ec431225028528a0bfc` |
| `rom-010-diagnostics-read-failure-green.log` | 12 maintained cases passed; one overhead case ignored. | `ea476a46915cd6aeb28480f245903ced76e7b830cfe6d4b3a030992347355f18` |
| `rom-010-diagnostics-read-failure-clippy.log` | Clippy reached successful completion. | `a9366078bfcbd2f708332971d29bdb1dced88238466eee1f2ebdf196efad58c3` |

These results close the original unregistered-kind and failed-read examples. They do not establish complete phase attribution.
One related P2 issue remains in `execution/command.rs`: a successful missing-receipt lookup leaves Receipt/Started as the active stage.
The following expected-revision check can then emit Receipt/Conflict for an ordinary stale revision.
The second and third lookup paths both have this condition. The receipt lookup did not fail.

End the successful lookup observation, including its None result. Start the revision-validation boundary before the expected-revision comparison.
Add a maintained stale-revision case that reports a revision conflict without a failed Receipt stage.
Retain authorization categories for actual Denied responses. Preserve the original Conflict business result and Resource revision.

The focused logs do not replace final-source full verification, installed-consumer drainage, or the remaining overhead evidence.

## Corrective review: successful lookup and revision conflict

The subsequent source inspection confirms that all three successful missing-receipt lookups now emit Receipt/Succeeded.
Both expected-revision comparisons now follow Validation/Started. The first replay path returns to Receipt after its current-row load.
The corrected flow no longer attributes the reviewed stale-revision failure to receipt resolution.

The maintained stale-revision case uses SQLite and redb. It retains `Error::Conflict`, requires Validation/Conflict, and rejects Receipt/Conflict.
The intended RED failed at the missing Validation/Conflict assertion. The corrected complete diagnostic suite passed 13 cases.
One overhead case remains ignored in that normal invocation. The following Clippy log reached successful completion.

| Log under `/var/lib/nixos-containers/rom-dev/var/tmp/` | Observed result | SHA-256 |
| --- | --- | --- |
| `rom-010-diagnostics-revision-phase-red.log` | Intended stale-revision attribution failure; one failed case. | `b2152a7b2c9af17aa1a4f9c1e94b5343351b89c3fbbc30967d8715772e9b33d0` |
| `rom-010-diagnostics-revision-phase-green.log` | 13 maintained cases passed; one overhead case ignored. | `e7f59c301588ff15a08d80e8fbf1335014ba485265980b88b84f21059e28a084` |
| `rom-010-diagnostics-revision-phase-clippy.log` | Clippy reached successful completion. | `98fc0db4bf185def94dc3b02c6c3ae2daea69c7f13c24db53f92c19c73253b99` |

The reviewed implementation taxonomy findings are resolved. The public timing description is accurate for the supervised stage interval.
No implementation blocker remains from these corrective reviews. This permits another integration build; it is not final R9 or release acceptance.
The design report's earlier callback-interval sentence still needs alignment with the corrected public description.
Final-source full verification, installed-consumer evidence, and the broader overhead limits remain as stated above.

## Fresh measurement evidence audit

An independent read-only audit checked `.superpowers/rom-010-final-diagnostics-6Ffo1E` on 2026-10-08.
All 591 recorded inputs still match their recorded byte counts and SHA-256 values.
This audit did not run native code or modify the compiled input set.

The corrected acceptance record matches all 18 raw measurement objects exactly.
The matrix contains each SQLite/redb, disabled/drained/undrained, and round 0/1/2 combination once.
The first object follows Rust's test-name prefix on the same line.
The original parser missed that object. Its preserved `acceptance.json` has 17 rows and is superseded.
Use `acceptance-corrected.json` for these results.

Each round performs 64 creates and 512 exact receipt replays on a fresh database.
The test checks 64 journal events after those operations.
Drained measurements include host drainage after each command. Each drained round exports 7296 records with zero recorded drops.
Undrained measurements use capacity one. Each round enqueues one record and records 7295 full-queue drops.
These are deliberate loss conditions, not complete exported traces.

The audit independently recalculated the sums and percentages below from the raw objects.

| Adapter | Disabled total, ns | Drained total, ns | Undrained total, ns | Drained increase | Undrained increase |
| --- | ---: | ---: | ---: | ---: | ---: |
| SQLite | 1289761262 | 1371888868 | 1342125274 | 6.3676595366685795% | 4.059977109158974% |
| redb | 1412191554 | 1510484487 | 1475757177 | 6.960311632057814% | 4.501204020088623% |

These percentages compare three summed wall intervals with the corresponding disabled sum.
They do not represent an average of per-round percentages or a statistical confidence bound.
The fixed-order workload uses an unoptimized debug profile. It does not establish production latency, allocation cost, RSS, or network exporter performance.
Subsequent changes to the compiled input set require fresh matching evidence.

The raw overhead log reports one passing characterization test, zero failures, and 13 filtered tests in 8.48 seconds.
The fresh public consumer log reports three passing tests, zero failures, and zero ignored tests.
The scoped Clippy log reaches successful completion. Its 0.11-second duration reflects a warm development build.
The capsule records Cargo 1.99.0 and rustc 1.99.0.

| Independently hashed evidence | SHA-256 |
| --- | --- |
| Capsule `inputs-before.json` | `8895f57a85c2c88cc148cb11289edc7d2f001516f355dbfa337953b7895413d7` |
| Capsule `acceptance-corrected.json` | `225feac919231cd9f4bbc5d7e3bf57f9d5b4ec3b28c206226f55c048cdc21caf` |
| Native `rom-010-diagnostics-final-source-overhead.log` | `1dfda02b0ec90713cd01d956b5d7ab6fbfd73f8a35fa42956ab64ef795aafa7f` |
| Native `rom-010-diagnostics-final-public-consumer.log` | `e163237c263b999d316023c40a80a1c1d679e7523aecb79f40b62623a1b28f62` |
| Native `rom-010-diagnostics-final-public-clippy.log` | `4cf86278c1e6d659943a46a3d7e3da429ac15be90f0f17742e24ef85228e7d3e` |

Native log paths start with `/var/lib/nixos-containers/rom-dev/var/tmp/` on this host.
The audit found no blocker in the corrected measurement record.
It closes the fresh workspace measurement subset, not installed final-package coverage or full local verification.
The capsule correctly retains `release_admitted: false`.
