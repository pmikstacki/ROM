# Public native-11 validation and load attribution

Date: 2026-10-08.

## Result

The full local verifier passed on the current source. Both public database adapters passed the selected large-data correctness tests.
The unchanged application load admission failed on both adapters. ROM 0.1.0 is not released.

| Gate | Actual result | Scope |
| --- | --- | --- |
| `./scripts/check` | Exit 0; 360 Node tests passed | OpenSpec, maintained Node checks, and `scripts/check-rust`. Studio browser checks remain separate. |
| Public SQLite large-data test | One selected test passed | Current physical format 11; bounded read, migration, retention, and restore regression. |
| Public redb large-data test | One selected test passed | Current physical format 11; bounded read, migration, retention, and restore regression. |
| SQLite load preparation | Failed at the drain deadline | 2,900 Resources fully drained; 3,000 created. |
| redb load preparation | Failed at the create deadline | 4,600 Resources fully drained; last progress reported 4,650 created. |
| SQLite stage diagnostic | Failed at the drain deadline | 3,200 Resources fully drained; instrumentation is not throughput admission. |

The load profile still requires 10,000 Resources and 10,000 completed Work records within 120,000 milliseconds.
Each batch contains 100 Resources. The tests retain SQLite FULL durability and the existing redb durability contract.
No deadline, workload, or acknowledgement guarantee was reduced.

## Source and execution evidence

The verifier's before and after witnesses contain 3,568 files and 34,694,702 bytes. Their file records are identical.
Its log SHA-256 is `32ca417ac844a3fd3a9ef86c38dc0613f925d567f4ff90685e32c20a2a1c9e67`.
The private result is `.superpowers/rom-010-public11-conformance/verifier-green-result.json`.
These witnesses establish the executed checkout. They do not establish clean release artifacts or a published tag.

The large-data runner checked 260 source inputs and 133 harness inputs before and after execution.
Each sequential test used its own 1-GiB image and a 1-GiB memory limit. Both tests passed without OOM.
The runner verified process drain, unmount, and detachment of its owned loop device. The images remain preserved.
The result is `.superpowers/rom-010-native-large-run/evidence/run-83271016e1e39f9b1daf6b0f/result.json`.
The root recheck is `.superpowers/rom-010-public11-load/public11-large-root-review.json`.
This is correctness evidence. It does not establish application throughput.

The optimized load binary SHA-256 is `1034bc2d09b6df1e0883835e1a8479faa7cfd083d650bae49924dc5f9ee74de9`.
Its build identity captures 465 inputs. Both load probes reported unchanged source witnesses and drained owned processes.
Their cgroups reported no OOM. Other-project CPU and disk contention were not measured.

The private evidence parent is `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/`.

- Build: `application-load-build-1eb8847fdbfb6e60e9abb7ad/build-identity.json`.
- SQLite: `application-load-seed-5ad7193b86073d9212ad7cea/result.json`.
- redb: `application-load-seed-e43fb674b663536a39c54194/result.json`.
- Diagnostic build: `application-load-build-c298cac092745a0aab2007e9/build-identity.json`.
- SQLite diagnostic: `application-load-seed-1a816e6f0dcba4ba0707c5c1/result.json`.

## Measured journal improvement

The diagnostic measured 443,354 metadata bytes for the first 1,500 Resources, after subtraction of the identity baseline.
The historical diagnostic measured 328,670,214 metadata bytes for the same Resource prefix.
The historical-to-current ratio is approximately 741.33. This comparison concerns encoded metadata bytes, not end-to-end speed.

The current first 100-Resource batch published 29,077 metadata bytes. Its fifteenth batch published 29,800 bytes.
The historical corresponding batches published 1,641,631 and 42,213,300 metadata bytes.
The journal change removed the measured growth from repeated publication of retained event vectors.
Different execution times, compiler artifacts, and instrumentation remain separate evidence.

## Remaining cost

After subtraction of the setup baseline, the diagnostic recorded these aggregate wall-clock intervals:

| Observation | Nanoseconds |
| --- | ---: |
| Native transaction completion | 118,104,297,592 |
| All named adapter stages | 119,551,805,187 |

Native transaction completion accounts for approximately 98.789% of the recorded stage intervals.
These are instrumented inclusive wall intervals. They are not exclusive CPU time or traced synchronization syscall time.
The measurement does not establish that fsync alone accounts for this share.

A subsequent control performed 100 overwrites and fsyncs of one 4-KiB file on the private mounted ext4 filesystem.
Its median was 7.018011 ms, and its 95th percentile was 15.286037 ms.
A later host-filesystem control used the same operation count and page size.
Its median was 3.423361 ms, and its 95th percentile was 3.865238 ms.
Neither control reproduces a database transaction. The controls ran at different times with unmeasured external contention.
They support a matched filesystem investigation. They do not prove a database speedup.

## Next experiment

First compare the unchanged workload on the private loop filesystem and a bounded host-filesystem destination.
Keep the same binary, profile, authorization, transaction mode, and acknowledgement behavior.
Record the destination's enforced allocation limit and owned-process lifecycle before admission.
Do not bypass private path validation or reuse a failed database as a fresh destination.

If transaction completion remains dominant, investigate bounded persistence batching with delayed acknowledgement.
A claim batch must commit before callbacks start. Completion still requires its own durable barrier.
Do not combine a callback with its claim transaction or remove crash recovery boundaries.
Any batching design needs cancellation, unknown-outcome, ordering, attempt-accounting, and restart conformance tests.

Mixed HTTP traffic, its fault matrix, final packaged consumers, and release artifact gates remain incomplete.
The successful correctness gates do not substitute for those requirements.

## Writing scope

This report follows the project's STE guidance. Technical identifiers and measured values retain their original meaning.
Vocabulary was checked against known rulings and high-risk patterns, not the complete official ASD-STE100 Part 2 dictionary.
This report does not certify STE compliance.

## Sequential host-filesystem comparison

The subsequent comparison used the same optimized binary and unchanged 465 captured inputs.
Each new source directory was bound to its own fresh host directory inside a private mount namespace.
The launch used `unshare --mount --propagation private`. No existing source directory was reused.

The owned target retained the four-core, 4-GiB, 512-PID cgroup limits and the 130-second outer process deadline.
The workload retained its 120-second deadline, batches of 100, and database durability.
The controller observed the actual host executable, process identity, and 64-MiB soft and hard file-size limits.
It checked at least 1 GiB of host headroom before launch.
The file-size limit is kernel-enforced. The directory has no aggregate hard quota.
The planned allocation was 256 MiB per adapter; the root audited actual allocated blocks after each run.

| Adapter | Last complete drain | Last create progress | Actual retained allocation | Result |
| --- | ---: | ---: | ---: | --- |
| SQLite | 6,100 at 119,004 ms | 6,200 at 119,453 ms | 16,826,368 bytes | Failed at the drain deadline |
| redb | 9,300 at 119,412 ms | 9,400 at 119,883 ms | 23,928,832 bytes | Failed at the drain deadline |

Both completed owned-process drain and unmount. Both passed the current 465-input source and preserved-binary checks.
Their fixed seed file inventories remained below the planned allocation and the individual file-size limit.
The larger completed prefixes support further filesystem and transaction investigation. They do not establish successful load admission.
Each comparison is one sequential observation. External contention and run-to-run variation remain unmeasured.

The root audits are:

- `.superpowers/rom-010-public11-load/host-audit-b5504011b1b76b042d4f15bb.json`.
- `.superpowers/rom-010-public11-load/host-audit-9749413d603e0db646b09295.json`.

Two earlier host probes had an invalid result collector.
Its limit parser rejected trailing whitespace in `/proc/<pid>/limits`.
The processes had the configured kernel limit, but the controller did not collect valid admission results.
Those probes and their files remain preserved. They are not the two results in the table.
A regression evaluated the actual old and corrected parser literals against the kernel's whitespace format.
The corrected controller also preserves output on failure and stops only its registered child when observation fails.

The isolated controller is `.superpowers/rom-010-public11-load/host-seed-probe-v3.mjs`.
The root source-regression record is `.superpowers/rom-010-public11-load/host-probe-v3-review-input.json`.
This controller is an experiment, not a production deployment path or a replacement for release acceptance.

The comparison does not justify relaxed durability or an increased workload deadline.
Before batching, check child-selection order, callback visibility, root attempt accounting, lease lifetime, and partial failure.
A completed claim transaction remains a required barrier before callback execution.

## Current native batch seed validation

The coordinator built the current grouped-claim runtime and corrected the load observer's forwarding gap.
The new binary matches 476 source identities. Both adapter runs used that same preserved binary.
The workload, mapper, durability settings, and 120-second deadline remained unchanged.

| Adapter | Resources and completed Work | Total elapsed | Drain elapsed | Actual retained allocation | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| SQLite | 10,000 each | 85,748 ms | 8,208 ms | 26,300,416 bytes | Passed the seed gate |
| redb | 10,000 each | 74,729 ms | 7,272 ms | 34,332,672 bytes | Passed the seed gate |

Each run retained the actual 64-MiB file-size limit and completed owned-process drain and unmount.
The coordinator verified current source identities, preserved binary identity, and allocated blocks after both runs.
Independent read-only review also verified both results and their retained file hashes.
These are single sequential observations, not latency distributions or sustained production throughput measurements.
The earlier incomplete runs do not provide a valid complete-workload speedup ratio.

Each adapter recorded 600 native claim-prefix calls, 400 atomic Work update calls, and 40,000 keyed claim checks.
The legacy singleton claim and materialization counters remained zero.
All 21 counter categories had no recorded failures or overflow.
The 600 prefix calls include two Idle results per batch of 100 Resources.
The historical six-category stage window omits the three new categories and cannot describe total grouped execution cost.
Boundary timings are synchronous wall time; they are not exclusive CPU time or fsync counts.

Root's summary is `.superpowers/rom-010-runtime-r10-prep/root-two-adapter-seed-summary.json`.
The SQLite audit is `.superpowers/rom-010-public11-load/host-audit-87c768e823bbdcf797cc671d.json`.
The redb audit is `.superpowers/rom-010-public11-load/host-audit-f1bca3dace5855213d48d290.json`.

The seed gate is now satisfied for this current-source experiment.
Mixed traffic, genuine disk-full behavior, provider loss, interrupted recovery, and final packaged validation remain incomplete.
The successful seed results do not complete R10 or admit ROM `0.1.0` for production.

## Mixed coordinator source review

The private mixed coordinator now has root and independent source review.
Both reviewers executed its 37 protocol and proxy-budget tests; all passed.
The 11 reviewed files match the retained source freeze.
No coordinator listener, mount, login, native process or mixed workload ran during this review.

The coordinator requires 12,000 admitted groups and 13,280 successful normal HTTP requests.
It also requires four operator requests, four consumed live streams and one deliberately unread stream.
Seed completion alone cannot satisfy its final proof: the native ledger must contain exactly 12,000 completed Work records.
Owned processes, sockets, upstream connections and the mount must drain before acceptance.
The retained seed database and source identities must remain unchanged.

The proxy separately bounds request count, concurrent requests, uploads, headers and aggregate ingress and egress.
The unchanged client retains its own consumed-stream byte bound.
Host RSS sampling and final allocated-block measurement are observations, not hard memory or directory quotas.
Queue timing remains local commit-confirmation-to-claim-confirmation; it is not a restart-durable clock.

Root evidence is `.superpowers/rom-010-runtime-r10-prep/mixed-root-source-review.json`.
Independent review is `.superpowers/rom-010-mixed-independent-review/review.md`.
The freeze is `.superpowers/rom-010-runtime-r10-prep/mixed-private-freeze.json`.
Its SHA-256 is `34cdbb96cab6e1af99188652f1f264259c9341ec4e027cf5f14828c41bfd5b18`.

Execution still requires fresh trusted browser inputs, a sufficient provider window and exclusive ports.
This source review does not establish mixed-load performance, fault recovery or final release readiness.
