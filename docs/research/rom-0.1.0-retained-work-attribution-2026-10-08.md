# Retained Work storage-boundary attribution

Date: 2026-10-08. Both optimized preparation probes failed the unchanged 120-second seed deadline.
The workload still requests 10000 Resources and 10000 retained completed Work records.
This is failure attribution, not release acceptance or proof of an optimization.

## Observed results

Root read both seed result files, numeric observations and terminal copy-inspection result files directly.

SQLite completed its last full 1100-record batch at 103945 ms. Its copied terminal state contains 1200 records: 1164 Done and 36 Pending.
redb completed its last full 1400-record batch at 114107 ms. Its copied terminal state contains 1485 records: 1400 Done and 85 Pending.
Both probes retained before/after source fences, empty terminal cgroups and no OOM events.
Observed categories contain zero storage errors and observer overflows. The fixture deadline still failed.

The table subtracts identity-setup baseline counts and durations from terminal observations.
Durations are synchronous inner Storage call wall intervals. They are not exclusive CPU, sync or codec time.
Do not sum overlapping observations into an exclusive percentage of process wall time.

| Adapter | Call | Workload calls | Boundary seconds |
| --- | --- | ---: | ---: |
| sqlite | retry_epochs | 3600 | 7.760514 |
| sqlite | load | 7128 | 0.146543 |
| sqlite | receipt | 3600 | 0.036154 |
| sqlite | commit | 1200 | 37.774630 |
| sqlite | claim | 1186 | 35.356915 |
| sqlite | materialize | 1164 | 40.059146 |
| redb | retry_epochs | 4455 | 10.120985 |
| redb | load | 8740 | 0.040352 |
| redb | receipt | 4455 | 0.008825 |
| redb | commit | 1485 | 36.949555 |
| redb | claim | 1428 | 34.014665 |
| redb | materialize | 1400 | 38.637955 |

Empty-child reactions complete through Materialize. This workload made no Finish calls.
No encoded live-state or write-byte attribution was collected.

## Selected next action

Implement exact current and lifecycle-reserved canonical accounting in the shared core.
Then implement validated affected-record transitions and native per-record persistence in both adapters.
Preserve ID-ordered Claim and all preceding expiry/reconciliation effects in one atomic transition.
Do not weaken durability, discard history, shrink the seed or widen the deadline.

The [contract investigation](rom-0.1.0-incremental-work-contract-2026-10-08.md) and its [independent review](rom-0.1.0-incremental-work-contract-independent-review-2026-10-08.md) define this seam.
Source inspection establishes full-state amplification. These boundary measurements identify expensive operations, but do not isolate amplification's internal share.
Only the corrected implementation and repeated unchanged workload can establish a speedup.

The old transition sources are preserved byte for byte in [the test reference](../../tests/persistence/reference-work-20261008/README.md).
Its manifest must remain unchanged when the new engine evolves. Capture alone does not establish differential equivalence.

## Evidence

- sqlite: result `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-63551767c2659fee265ce662/result.json`; metric SHA-256 `c256cedd764d30b355d695a8cf53c7b91fc421f0be69bf74289404dc504e74f4`.
- redb: result `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-a57f4a54daf51195f73d9fca/result.json`; metric SHA-256 `54414e8d331b7ddfec74f006e56b1469cbea2b8b504302c30969e21c8d6986d5`.

Full numeric summary: `/var/tmp/rom-010-attribution-summary-20261008.json`.
SQLite copy inspection: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-inspection-5e91bc8bb57d0623e406af66/result.json`.
redb copy inspection: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-inspection-fd9a0a8ae28727fe18f413cf/result.json`.
Original databases were not opened for these copy inspections. Original inode/hash and SQLite sidecar checks were retained.
