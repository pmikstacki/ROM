# Bounded whole-application load and failure admission

Date: 2026-10-08. Status: source investigation and proposed execution design.
This report covers R10. No mixed-load result is established by this document.

## Investigated boundaries

`StudioHost::router` exposes generic Resource operations through its existing `rom_http::Http` binding.
The same Host exposes blob reserve, upload, detach, and download routes.
Both groups resolve current sessions and use existing authorization and CSRF checks.
The fixture must use these actual routes rather than a separate benchmark-only implementation.

`rom_http::observation::observe` exposes SSE for observations.
Its response stream polls current authority and the shutdown signal.
A load case must stop reading an actual network response to exercise a slow transport consumer.
A delayed call to a Rust callback alone does not establish this behavior.

`WorkView` exposes state, attempts, generation, and `due` through the operator protocol.
`due` is not an enqueue timestamp. The fixture must not label it as queue age.
Queue age requires an observed enqueue time and the matching work-start or delivery time.
The pending R9 observer contract must be inspected before that metric is implemented.

## Measurement strategy

A closed-loop client can hide delays when a slow server also reduces request generation.
wrk2 measures latency from the intended send time to the completed response, addressing coordinated omission.
Its documentation also identifies timing resolution and calibration limits.
Use scheduled-arrival latency and actual dispatch latency as separate measurements. [wrk2 design](https://github.com/giltene/wrk2).

For this finite fixture, retain a bounded sample for every scheduled operation.
Compute p50, p95, and p99 by operation class. Do not average independent percentiles.
Prometheus distinguishes mergeable histogram counts from precomputed summary quantiles.
A later exporter can use histograms; the finite test does not require a metrics service. [Prometheus histogram guidance](https://prometheus.io/docs/practices/histograms/).

The load generator must have a finite pending queue and active-request limit.
Tokio semaphore fairness does not replace an application request budget.
Admission rejections and generator backlog must remain visible in the result. [Tokio semaphore contract](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html).

## Proposed deployment profile

The operations audit proposes 10,000 Resources, 32 clients, and 32 subscriptions.
Queries are bounded to 50 rows. Each upload is bounded to 64 KiB.
The proposed full run has two minutes of warm-up, ten measured minutes, and two drain minutes per adapter.
These are proposed test counts, not an accepted production support profile.

Use a scheduled mix of 50% reads, 25% mutations, 10% receipt replays, 10% queries, and 5% blob operation groups.
Each successful mutation can also schedule a declared reaction through the ordinary Resource contract.
Track each blob group's reserve, upload, and download separately.
Track successful commits, known denials, overload, unknown outcomes, and client timeout separately.
A replay must retain its original operation identity and input.

Begin with a shorter exploratory run before admitting the full profile.
Proposed exploratory limits: 30 seconds warm-up, 90 measured seconds, and 30 seconds drain per adapter.
Use at most 100 scheduled operations per second, 32 active requests, and 64 pending requests.
The generator must reject additional scheduling when its finite queue is full.
The maximum is 12,000 scheduled operations before drain, excluding the separately bounded seed population.
Blob groups can make three requests. Bound actual dispatched requests to 13,280 before drain.
A passing exploratory run does not establish the ten-minute profile.

## Proposed finite allocation and ownership

New fixture ownership is `tests/application-load/**` and this report only.
Existing R8, R7, core, Host, and production files remain unchanged.
Run SQLite and redb sequentially. Use one exclusive owned deployment at a time.
Reuse the granted private ports 44389–44393 only after fresh ownership and listener checks.
Use fresh source/binary/lockfile witnesses after R9 freezes.

Proposed Host and generator limits retain four CPUs, 4 GiB RAM, and 512 PIDs per owned group.
Bound each HTTP response to 1 MiB and each request deadline to five seconds.
Bound the combined samples and diagnostics to 64 MiB per adapter.
Bound attachments to 640 uploads per exploratory run: at most 40 MiB of payload bytes.
Bound each fresh application directory to a monitored 256 MiB planning envelope.
Monitoring is not a hard quota. An enforced private filesystem is required for the disk-full case.
All failed cases, snapshots, and source directories must remain available as evidence.

The existing private provider fixture has a 300-second ready window and 600-second container timeout.
It can support a short exploratory case when launched immediately before traffic.
It cannot establish the proposed 14-minute complete profile without an explicitly extended finite fixture lifetime.
Do not silently reuse an expired ready file or restart it during a baseline measurement.
Synthetic token validity must cover the chosen run or be renewed through an actual supported login flow.
Neither choice changes the earlier actual-expiry regression evidence.

## Separate fault cases

| Fault                   | Actual injection                                                                            | Required correctness checks                                                                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Slow subscriber         | Stop reading one owned SSE socket while mutations continue.                                 | Finite server queues, visible stream closure or lag, responsive unrelated calls, bounded shutdown.                                                    |
| Provider outage         | Stop only the exact owned synthetic Authentik server through its admitted engine object.    | Temporary failure differs from denied identity; no protected disclosure; fresh and retained sessions are checked separately.                          |
| Process interruption    | Interrupt only the fixture's freshly admitted Host process after a known accepted mutation. | Recover through the same database; replay receipts; preserve work identity and generation fences.                                                     |
| Private disk exhaustion | Exhaust a dedicated fresh bounded image used only by this fixture's database and objects.   | No acknowledged partial commit; retain unknown outcomes; reject incomplete attachment publication; recover after the fixture releases its own filler. |

The disk-full image proposal is 64 MiB for a separate reduced fault profile, inside the existing private envelope.
Never fill the shared root filesystem. Never delete real journal or WAL files to simulate failure.
SQLite documents corruption hazards around modifying database-related files outside its protocol. [SQLite corruption guidance](https://www.sqlite.org/howtocorrupt.html).

Record each adapter's actual durability configuration before interruption.
redb exposes durability options; the test must preserve the product adapter's current setting. [redb durability contract](https://docs.rs/redb/latest/redb/enum.Durability.html).
A successful graceful shutdown does not substitute for process-interruption evidence.

## Required output

Record intended arrivals, dispatched requests, backlog, completed requests, and rejected scheduling.
Report separate latency distributions for reads, commits, replay, queries, uploads, and stream delivery.
Record peak RSS, cgroup memory events, CPU time, application directory growth, and SQLite WAL growth.
Use per-application measurements. Shared free-space changes do not identify this fixture's allocation.
Record work queue age only after its timestamps and definition are established.
At drain, list accepted obligations that remain pending and their stable identities.

The result must identify the exact source, binary, locks, configuration, workload, faults, and enforced limits.
Original Astral Plane private-view clearing remains a separate application acceptance requirement.
A load fixture cannot establish AP-UX-006/013/021 adoption or AP-UX-033 guest usability.

## Implemented measurement primitives

Four maintained Node tests now pass for finite admission and scheduled-arrival measurements.
Their initial missing-module failures and subsequent passes are retained under `/var/tmp/rom-010-application-load-*-20261008.log`.
The sample limit is 13,280. Operation and outcome labels come from fixed lists.
Unknown labels, extra payload fields, invalid times, repeated identities, and exceeded budgets are rejected.
Scheduled latency includes client dispatch delay. Dispatch latency and generator delay are also reported independently.
These tests do not establish Host performance or fault recovery.

The current R9 fixed diagnostic record starts its local work timer after the claim.
It therefore does not provide durable per-work queue age.
The approved fixture will measure enqueue-to-claim locally through controlled workload observations.
Across restart, that local measurement is unavailable unless a durable timestamp is established separately.
The approved longer provider proposal has a finite 1,200-second ready window and 1,500-second container timeout.
It still requires a maintained fixture-only implementation and cleanup admission before execution.

## Maintained provider-window admission

The shared fixture now accepts only the default window or the explicit `--load-window` profile.
Default settings remain 300 ready seconds, 600 container seconds, and a 540,000 ms relay lifetime.
The load profile selects 1,200 ready seconds, 1,500 container seconds, and a 1,440,000 ms relay lifetime.
Unknown arguments and container lifetimes outside 600 or 1,500 seconds are rejected.
No actual provider was launched for this change.

The ready deadline is limited by the remaining container and relay lifetimes, with a 45-second cleanup reserve.
The fixture records launch times, health timing, selected limits, and the actual remaining admission budget.
The health loop has a bounded recorded deadline; 150 attempts do not mean 150 elapsed seconds.
Before a workload starts, `requireWorkloadWindow` must confirm that its finite duration fits the remaining window.
A nominal 1,200-second profile does not guarantee 1,200 seconds after variable startup.

The maintained tests reproduce a late-startup case with only 855 seconds available.
An 840-second workload is admitted immediately and rejected after another 20-second delay.
Expired lifetimes and malformed evidence are rejected.
All 118 provider and load mechanism tests passed in 11,450.929893 ms.
Logs: `/var/tmp/rom-010-provider-window-admission-red-20261008.log` and `/var/tmp/rom-010-provider-window-admission-green-20261008.log`.
This result verifies admission logic, not a long-running actual provider or a mixed-load deployment.

## Profile configuration required by source inspection

The current Runtime defaults admit 1,024 snapshot rows and 1 MiB of snapshot data.
A 10,000-Resource query profile must explicitly configure sufficient whole-kind candidate bounds.
A 50-row result limit does not remove authorization or complete-candidate requirements.
The fixture must record these Runtime limits independently of the returned page size.

The default durable reaction ledger has 1,024 records and a 1 MiB byte limit.
Seeding or mutating the profile can create additional reaction work before delivery.
Record the configured ledger bounds and retained work volume before starting traffic.
Do not disable reactions or silently drop work to make the proposed load profile pass.

## Maintained finite exploratory profile

The fixture profile now specifies 12,000 complete query candidates and 32 MiB of candidate bytes.
The returned page remains 50 rows. Actual encoded seed size must pass a separate byte proof.
The retained work ledger admits at most 12,000 records and 16 MiB.
Completed work still occupies this budget; a worker drain does not remove completed records.
The code retires complete causal roots only through explicit retry-epoch maintenance.
The load fixture will not use that maintenance to hide its retained work cost.

The proposed isolated LoadRecord registration has one admitted reaction per semantic mutation.
Seed 10,000 Resources in batches of 100, then drain all 10,000 corresponding work records.
Require measured completion counts, ledger bytes, candidate bytes, and a bounded drain duration before measured traffic.
The exploratory traffic admits at most 1,500 further semantic mutations and 500 additional notification intentions.
This gives a maximum of 12,000 retained records when each mutation creates exactly one reaction.
The native registration must verify that exact work fanout before this profile is executed.
A rejected admission, unexpected fanout, or excess encoded size must stop the workload rather than truncate its evidence.

Three maintained profile tests passed after the absent-module RED.
They distinguish page and candidate limits, reject incomplete seed drain, and count completed records against the ledger.
Logs: `/var/tmp/rom-010-load-profile-red-20261008.log` and `/var/tmp/rom-010-load-profile-green-20261008.log`.
This is fixture admission evidence, not an executed native load result.

## Concurrent queue timing design

The proposed observer does not hold its bookkeeping lock during adapter commit or claim operations.
It records successful commit confirmation and actual claim confirmation using a local monotonic clock.
A claim can race the post-commit bookkeeping. Such a case must report an unavailable sample.
It must not fabricate a zero interval or reconstruct a time from `due` or chain-root start.
Duplicate confirmation must retain the first observed start rather than reset it.
The observer retains at most 12,000 identities and samples. It exports timings and missing counts, not raw work identities.
The measured interval includes native claim I/O and excludes unconfirmed commit time.
It is fixture commit-confirmation-to-claim-confirmation delay, not durable queue age.
All samples become unavailable after a process restart.
Bookkeeping and missing-sample overhead must accompany any eventual performance result.
The new native mechanism tests are staged; they have not run while the shared native lease belongs to AI.

## Executed native observation mechanism

The first native run failed the duplicate-confirmation regression: three tests passed, and one failed.
A repeated confirmation moved the stored start time forward and reduced the measured interval.
The correction retains the first confirmation. All four timing mechanism tests then passed.
The subsequent native gate passed five tests, including the reused closed configuration test.
The load library and extracted recovery fixture both passed Clippy with warnings denied.
Two initial style warnings were corrected without suppression; their failure log remains preserved.

Logs: `/var/tmp/rom-010-load-observer-native-red-20261008.log`, `/var/tmp/rom-010-load-observer-native-green-20261008.log`, and `/var/tmp/rom-010-load-observer-final-check-20261008.log`.
The separate native target used 777,912,320 allocated bytes during the check.
This is an observed allocation, not a hard quota.
The real two-adapter reaction-registration tests are now staged for the next coordinated native slot.
The actual Host workload, transparent Storage forwarding, and failure matrix remain unexecuted.

## Actual native registration and transparent observation controls

The initial two-adapter registration run failed both seed-drain assertions.
The synthetic worker could claim the work, but the load Resource policy did not admit its source access.
The fixture policy now admits the exact `demo-host` Service worker, the bootstrap actor, and the linked synthetic Human.
It does not admit arbitrary external service identities.
Both unwrapped and observed SQLite/redb controls then completed all ten admitted reactions.
The four controls retained exactly ten work records, verified their final `Done` states, and checked actual serialized candidate and ledger bytes.
The observed controls each produced ten confirmation-delay samples, zero missing samples, and zero rejected observation entries.
The transparent wrapper forwards adapter ownership, query strategy, journal, receipts, and operator methods.
It does not hold its bookkeeping lock during native commit or claim operations.
Observation overflow does not change the adapter's commit decision; it is reported separately.

The final native gate passed nine tests and Clippy with warnings denied.
Logs: `/var/tmp/rom-010-load-registration-red-20261008.log` and `/var/tmp/rom-010-load-registration-final-20261008.log`.
An intervening test-format lint failure remains preserved in `/var/tmp/rom-010-load-registration-green-20261008.log`.

The source now stages a finite Host runner using the shared fixture identity and Host configuration.
Preparation has a 120-second deadline and preserves completed reaction records after each 100-row batch.
The exploratory Host lifetime is 240 seconds. The explicit long-window candidate lifetime is 960 seconds.
Neither lifecycle has launched. Runner compilation and provider admission review remain required.
Actual HTTP scheduling, fault fixtures, RSS/disk sampling, and the complete mixed-load matrix remain open.

## Maintained mixed traffic plan

The exploratory generator now emits exactly 12,000 scheduled groups across the 30-second warm-up and 90-second measurement phases.
It distributes every traffic category across both phases.
The plan contains 9,040 reads or queries, 1,500 mutations, 820 fingerprint-identical replays, and 640 attachment groups.
Each attachment group reserves, uploads, and downloads one 64 KiB object.
The plan has 13,280 primitive HTTP requests and at most 40 MiB of upload input.
Exactly 500 mutations include a notification intention.
Replays preserve the original expected revision, action input, and idempotency key even when arrival order races the original request.
The returned query page is 50 rows. Whole-kind candidate admission remains a separate 12,000-row and 32 MiB bound.

Three maintained plan tests passed after the absent-module RED.
The complete current load-helper gate passed 12 tests.
Logs: `/var/tmp/rom-010-load-plan-red-20261008.log` and `/var/tmp/rom-010-load-plan-green-20261008.log`.
These tests establish finite workload construction and replay fingerprints. They do not establish completed HTTP traffic or production throughput.
Stream setup, operator inspection, and deliberate fault controls have separate admission and request budgets before they can run.

## Compiled finite Host candidate

The staged library and Host runner subsequently compiled under the coordinated 60-second native window.
Ten tests passed, including the fixed 256 KiB bound for 12,000 maximum-width timing values.
Clippy passed with warnings denied. Log: `/var/tmp/rom-010-load-host-compile-20261008.log`.
This establishes compilation and focused controls only. It is not a preserved original compiler-artifact witness or a launched load case.
The next admission needs a frozen source inventory, detached actual Host binary, bounded seed preparation, and provider lifetime proof.
The real load and failure matrix remain open.

## Provider validity and scheduled HTTP controls

The load fixture now has a scoped synthetic-provider validity helper.
Exploratory admission requires the preserved `minutes=5` setting.
Full-profile admission temporarily sets `minutes=20` through the closed provider API.
It verifies the active setting, then restores and verifies the original setting on every scoped exit.
An uncertain PATCH response also triggers restoration. A restoration failure rejects the result.
These controls did not change any live provider setting.

Before traffic, the caller must supply timing from the actual provider token.
The remaining lifetime must cover traffic and at least 45 seconds of cleanup.
Exploratory timing permits at most 300 seconds of original validity; full timing permits at most 1,200 seconds.
This is fixture admission, not signature validation or a replacement for Host expiry checks.
Natural expiry remains a separate failure test.

The maintained scheduler retains planned arrival times, dispatch delay, and completion time separately.
It bounds active requests and pending client work. Client scheduling refusal is separate from server overload.
Its deadline abort reaches admitted executors, and the maintained deadline control verifies their completion.
The closed HTTP executor bounds requests to 13,280 and response data to 64 MiB.
Each response has a separate 64 KiB limit.
Read and mutation responses must match the requested Resource identity.
Query responses must contain at most 50 matching Resource rows.
Downloads must contain the original 64 KiB attachment bytes.
Credentials and Resource values do not enter the sample summary.
HTTP `outcome_unknown` remains distinct from overload, denial, and confirmed success.

The current source-only gate passed 24 maintained tests.
Log: `/var/tmp/rom-010-load-controls-green-20261008.log`.
Absent-module RED logs and the explicit unknown-outcome regression are preserved:
`/var/tmp/rom-010-load-scheduler-red-20261008.log`,
`/var/tmp/rom-010-load-http-red-20261008.log`, and
`/var/tmp/rom-010-load-http-unknown-red-20261008.log`.
An intermediate duplicate-declaration setup failure was corrected before this final gate.
These tests use controlled acquisition functions. They do not establish completed mixed HTTP traffic against the actual Host.
Source/binary capture, actual login, stream controls, process sampling, and scoped failure execution remain required before the R10 matrix is accepted.

The next source-only gate passed 26 tests after the login-session binding and dual-failure control were added.
Log: `/var/tmp/rom-010-load-latest-controls-green-20261008.log`.
The login helper reuses the actual Authentik form stages used by the earlier recovery fixture.
Its session binding requires the linked synthetic User and one secure cookie for the exact private origin.
This source preparation does not establish a new actual login.
The dual-failure test preserves both a failed traffic scope and failed provider restoration through an `AggregateError`.
Its maintained RED is `/var/tmp/rom-010-load-validity-dual-red-20261008.log`.
A staged build wrapper requires the actual compiler-artifact event, unchanged source inventory, and bounded detached binary preservation.
It has not yet executed a build or admitted a live provider/load phase.

## Actual seed admission: both adapters rejected the proposed bound

The actual standalone Host compiled from the recorded 343-source inventory.
The build completed in 1.37 seconds under the approved 120-second ceiling.
Detached retention preserved a 52,632,448-byte binary:
SHA-256 `d8a664d7662f76d41c42e0d5c7f1f6c748482f0f6d16558b6053d4f4ab56b19e`.
Identity: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-d798f67a60d97acccf54205e/build-identity.json`.
The native target occupied 880,078,848 allocated bytes. This was an observation, not a hard quota.

Both sequential, isolated 10,000-row seed probes reached their 130-second wrapper deadline.
Both owned process groups then drained. Both source and binary fences remained unchanged.
Neither adapter admitted the proposed workload. No provider, network port, or HTTP load started.
The configured Resource, candidate, retained-work, RAM, CPU, and process limits were not widened.
Neither cgroup reported an OOM or process-limit hit.

The SQLite failed nonce preserves 400 Resource rows and 400 durable work records.
A read-only postmortem found 316 `Done`, one `Leased`, and 83 `Pending` records.
Encoded work-ledger state was 447,907 bytes; encoded whole durable state was 565,547 bytes.
The source directory occupied 6,594,560 allocated bytes.
Database, WAL, and SHM content hashes were unchanged before and after the read-only inspection.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-dc8c492388294695e70aefd3/readonly-postmortem.json`.
The first controller discarded child output; no per-batch latency proof exists for this attempt.
This is an evidence limitation, not proof that setup or writes caused the timeout.

The redb failed nonce occupied 4,202,496 allocated bytes.
Its bounded output was preserved and was empty. Exact partial Resource/work counts remain to be inspected through a fresh offline copy.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-39e0c98811ec0d6d397afc3e/result.json`.
Both nonces preserve `resource-postmortem.json` with actual cgroup CPU and device I/O counters.
SQLite recorded 49,382,778 microseconds of CPU use; redb recorded 108,611,881 microseconds.
Neither group recorded CPU throttling.
The host-device write counters were 417,923,072 bytes for SQLite and 1,075,339,264 bytes for redb.
Loop-device counters describe another layer of the same writes and must not be added to those values.
These observations show substantial work. They do not isolate the cause or establish production throughput.

The next fixture must retain bounded batch create/drain timings and partial progress.
The original failed data stays unchanged. The final 10,000-row workload must not be reduced to make its admission pass.
The actual mixed-load and failure matrix remains open because seed admission failed.

## Timed unoptimized reproduction and production-profile comparison

The instrumented fixture compiled from a fresh 344-source inventory.
Ten native tests and Clippy with warnings denied passed.
Its detached binary was 52,697,712 bytes, SHA-256 `025bc6b933ccd796c395a42a4519c3c02d6e23706eba6669913db595d5e9050a`.
Both new seed probes exited with the fixture's `finite seed create deadline` error.
They retained fixed per-batch create/drain output. Neither admitted the full workload.

| Adapter | Batch | Create 100 rows | Drain admitted work |
| --- | --- | ---: | ---: |
| SQLite | 1 | 2.926 s | 7.264 s |
| SQLite | 2 | 5.403 s | 14.263 s |
| SQLite | 3 | 9.283 s | 15.279 s |
| SQLite | 4 | 9.460 s | 19.018 s |
| SQLite | 5 | 11.901 s | 23.302 s |
| redb | 1 | 1.834 s | 6.023 s |
| redb | 2 | 4.803 s | 11.859 s |
| redb | 3 | 8.614 s | 15.713 s |
| redb | 4 | 10.240 s | 19.457 s |
| redb | 5 | 12.627 s | 22.746 s |

These are consecutive retained-ledger batches in one run per adapter, not independent repeated benchmarks.
The costs increased in both creation and draining. The table does not isolate a specific function.
SQLite completed its 500-row drain at 118.103 seconds; redb completed it at 113.920 seconds.
SQLite's subsequent failed database contained 516 rows/work records: 500 `Done` and 16 `Pending`.
Its main database hash stayed unchanged during inspection.
However, Node's `readOnly: true` connection created an empty WAL and 32 KiB SHM when these sidecars were initially absent.
The postmortem explicitly records `content_unchanged: false`. This inspection does not prove filesystem immutability.
Future SQLite inspections must use fresh detached copies, as the redb inspection already does.
The original main database and all sidecars remain preserved.

The fresh redb offline copy reported 565 Resource/work records: 500 `Done` and 65 `Pending`.
Candidate JSON was 74,863 bytes; the work snapshot was 528,573 bytes.
The original failed redb file hash was unchanged.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-inspection-d8f21027cc1afb8e8db507fe/result.json`.
The copied database was reopened through public Storage APIs. Its ownership generation is therefore separate from the preserved original.

Source inspection identifies a hypothesis for further measurement.
Both adapters decode the complete `StorageState`, update one work record, encode the complete state, and commit it for each claim/finish.
Relevant implementation: `crates/rom-sqlite/src/persistence.rs`, `crates/rom-redb/src/storage.rs`, and `crates/rom-redb/src/state.rs`.
No adapter or core implementation changed during these probes.
All timed results above used Cargo's unoptimized development profile with debug information disabled.
They do not establish a production capacity limit.

The approved next comparison keeps the exact 10,000-row workload and all retained-work bounds.
It builds an optimized binary in the separate `target/application-load-release` directory.
Its compilation has a 900-second ceiling, two jobs, no incremental compilation, and locked offline resolution.
The target's 8 GiB allowance is planned and monitored; it is not a hard quota.
Detached binary preservation enforces the existing 256 MiB streaming limit.
The runtime and wrapper ceilings stay 120 and 130 seconds.
Preparation: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-2e835b42c3b6ff5f7b8cad80/preparation.json`.
The recorded comparison verifies identical hashes for all 342 native compiler input files.
Fixture JavaScript identities changed and are not represented as identical historical fixtures.
Compilation completed with exit zero in 2 minutes 26 seconds. Optimized seed results remain pending.
The detached binary is 23,547,984 bytes; its SHA-256 is `7d58b77fd8f87b5e8c2b080c131a09b4d3af3d2d433a7c1c18f15a63dcf990f4`.
The release target allocated 622,882,816 bytes at the recorded observation.
No mixed HTTP load or provider phase was launched by this worker.

## Actual provider relay admission correction

The original explicit load profile selected a 1,440,000 ms relay deadline.
An actual provider launch rejected this value before readiness: the supervisor's maximum is 1,200,000 ms.
The failed launch and stopped containers remain preserved in `/var/tmp/rom-010-astral-provider-chromium-long-window.log`.
Existing profile tests checked numeric configuration but did not exercise actual supervisor admission.

A new regression invokes `ProcessRegistry.admit` with each maintained profile's actual relay deadline.
It failed for the long profile before the correction. The supervisor's strict maximum remains unchanged.
The corrected long profile selects 1,200 nominal ready seconds, 1,500 container seconds, and a 1,200,000 ms relay deadline.
Default lifetimes remain 300, 600, and 540,000 ms respectively.
The ready interval uses the earliest actual lifetime deadline, less startup time and the 45-second cleanup reserve.
It is an upper limit, not a guarantee of 20 minutes after startup.
Admission still refuses a workload whose complete duration cannot fit in the measured remaining interval.

RED evidence: `/var/tmp/rom-010-provider-relay-admission-red-20261008.log` (one intended failure, five passes).
The first broad rerun passed 143 of 144 tests.
Its sole failure was a duplicate load-profile assertion that still assumed the old relay reserve.
That assertion was corrected without changing provider source after the root worker's independent 13-test check.
The final provider and load helper rerun passed all 144 tests.
GREEN evidence: `/var/tmp/rom-010-provider-relay-admission-green-final-20261008.log`.
Optimized seed timing is deferred until the root worker's actual provider/browser phase stops, to avoid concurrent CPU contamination.

The corrected actual provider reached readiness in run `db9b07bddfe3ae27bd10bfb2b71d0146`.
Its worker and server stop commands exceeded their 15-second command deadlines.
These failed acknowledgements were not treated as proof of stopped containers.
A separate, bounded, read-only reconciliation inspected exactly the three recorded immutable container IDs.
All three matched the preserved images, nonces, network references, cgroups, and resource limits.
Each actual container had PID zero, `Running: false`, and `Paused: false`.
The recorded init and conmon process births were absent. Both original and reconciliation cgroups were empty.
Private ports 44389–44393 were vacant. No stop, restart, or kill command was issued by reconciliation.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/provider-reconciliation-db9b07bddfe3ae27bd10bfb2b71d0146-1791458787378.json`.
The original stop deadlines remain preserved as historical failures; the new observation establishes the later stopped state.

## Optimized seed admission results

Both optimized comparisons failed the unchanged 10,000-Resource admission profile.
SQLite reached its 120-second drain deadline; redb reached its 120-second create deadline.
Both processes exited with code one and drained normally. Neither reached the wrapper's 130-second timeout.
Their cgroups were empty, with no OOM or PID-limit events. Both source fences remained unchanged.
These probes followed both browser engines' completed cleanup; no provider or root benchmark ran concurrently.

| Adapter | Development time to 500 completed reactions | Optimized time to 500 completed reactions | Optimized time to 1,000 completed reactions |
| --- | ---: | ---: | ---: |
| SQLite | 118.103 s | 36.773 s | 113.846 s |
| redb | 113.920 s | 16.402 s | 64.114 s |

Each entry is a milestone in one retained-ledger run, not a repeated production benchmark.
Compilation optimization improved these milestones but did not make the complete profile pass.
The profile, candidate bounds, retained ledger, durability, and deadlines were not reduced or widened.
Mixed HTTP traffic remains unadmitted.

Fresh detached-copy inspections established the exact partial state:

| Adapter | Resource/work records | Done | Pending | Candidate JSON bytes | Work JSON bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| SQLite | 1,100 | 1,032 | 68 | 145,751 | 1,028,540 |
| redb | 1,487 | 1,400 | 87 | 197,028 | 1,390,248 |

The inspector opened only new detached copies through public Storage APIs.
SQLite's original WAL, SHM, and rollback journal were absent before and after inspection.
The original database file hashes remained unchanged for both adapters.
SQLite's main file was 4,984,832 bytes. redb had 8,417,280 logical bytes and 7,892,992 allocated bytes.
Copy reopening uses a fresh ownership generation; it does not adopt original claims.
The controller now rejects sidecars, including dangling symlinks, and separately checks original inode identity after inspection.
The sidecar and inode guards each have preserved behavioral RED evidence; all 32 load helper tests passed afterward.

Evidence paths:

- SQLite seed: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-9e68c5ce25a3fe3004731a37/result.json`.
- redb seed: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-29638f0187f7166e5f8e8205/result.json`.
- SQLite copy inspection: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-inspection-8ddfb4063c893c69392e05f4/result.json`.
- redb copy inspection: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-inspection-2cb800c2132d8f769817168d/result.json`.
- Sidecar RED/GREEN: `/var/tmp/rom-010-load-sqlite-copy-sidecars-behavior-red-20261008.log`, `/var/tmp/rom-010-load-sqlite-copy-sidecars-green-20261008.log`.
- Inode RED/GREEN: `/var/tmp/rom-010-load-sqlite-copy-inode-red-20261008.log`, `/var/tmp/rom-010-load-sqlite-copy-inode-green-20261008.log`.

The next approved fixture increment measures fixed storage-operation counts, total durations, and maximum durations.
It wraps actual inner calls and preserves their results. It does not hold observation locks during adapter I/O.
Work snapshot sizes will not be reported as actual cumulative encoded-state writes.
There is no public per-write codec observer in the current adapters.

## Actual storage boundary attribution

Three maintained tests first failed for missing call observations, overflow evidence, and reentrant-call observations.
The implemented observer preserves success and error results. Its lock is acquired only after each inner operation returns.
Fifteen native fixture tests and Clippy with warnings denied then passed.
The complete worst-case numeric batch evidence fits the separate 64 KiB file budget.
Logs: `/var/tmp/rom-010-load-storage-call-red-20261008.log`, `/var/tmp/rom-010-load-storage-call-full-green-20261008.log`, `/var/tmp/rom-010-load-storage-call-clippy-20261008.log`.

The new optimized build completed in 7.20 seconds with a matching 349-file source witness.
Its detached binary is 23,592,064 bytes, with SHA-256 `1b8858ae9f0a7469e73809efac997f6f32f3ea4bf2a44b94ba990fe8d5e87751`.
Build identity: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-6e609e6e75218f045a22b9dc/build-identity.json`.
Both attributed runs still failed the unchanged 10,000-Resource profile within their existing deadlines.
SQLite reached the drain deadline; redb reached the create deadline.
Both exited normally with code one, preserved their source fences, and drained without OOM or PID-limit events.

| Synchronous Storage operation | SQLite calls / total wall seconds | redb calls / total wall seconds |
| --- | ---: | ---: |
| commit | 1,205 / 37.830 | 1,490 / 36.997 |
| Claim | 1,186 / 35.357 | 1,428 / 34.015 |
| Materialize | 1,164 / 40.059 | 1,400 / 38.638 |
| retry_epochs | 3,616 / 7.761 | 4,471 / 10.121 |
| load | 7,148 / 0.147 | 8,760 / 0.040 |
| receipt | 3,615 / 0.036 | 4,470 / 0.009 |

These are observed whole-call durations, including adapter locking and I/O. They are not CPU times or isolated codec durations.
They include the separately recorded identity-seeding baseline. Summed durations must not be assumed to exclude concurrent waits.
No failed Storage calls or observer overflows occurred.
Finish had zero calls: these empty-child reactions complete through Materialize.
The root worker deferred its heavy builds and benchmarks; other projects' whole-host activity was not measured.
No claim of exclusive machine use or production throughput follows from these runs.

Detached-copy inspection found 1,200 SQLite rows/work records: 1,164 Done and 36 Pending.
redb had 1,485 rows/work records: 1,400 Done and 85 Pending.
Original file hashes and inodes remained unchanged. SQLite sidecars remained absent; only the new copies were opened as databases.
The source freeze was released only after these inspections and source checks completed.

Evidence:

- SQLite: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-63551767c2659fee265ce662/result.json`.
- redb: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-a57f4a54daf51195f73d9fca/result.json`.
- Each seed directory contains `storage-call-boundary.json`: 7,728 bytes for SQLite and 8,208 bytes for redb.
- Each seed directory contains `resource-postmortem.json`, with exact owned-cgroup counters. Its memory peak is not process RSS.
- SQLite copy: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-inspection-5e91bc8bb57d0623e406af66/result.json`.
- redb copy: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-inspection-fd9a0a8ae28727fe18f413cf/result.json`.

The whole-call measurements narrow the next investigation to commits and work-state updates.
They do not establish the share caused by JSON decoding, accounting, locking, serialization, or durable writes.
The approved next product work requires independent old-model equivalence checks and the same 10,000-Resource admission profile.
