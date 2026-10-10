# Work terminal diagnostic, 2026-10-09

The current SQLite mixed-load diagnostic failed its original acceptance criteria. The diagnostic files and process cleanup passed independent review.
This result does not qualify production load, selected SQLite 3.53.4, release artifacts, or publication.
No UI changes were part of this experiment.

## Executed source and controls

The diagnostic Host build completed with physical exit zero in 28.901712 seconds.
Its detached binary SHA-256 is `b4dde98c5a3d59d8e94b42e883dd9596043c772db22e30167ed10a98c9bcebf8`.
The build pins 532 load sources and 4,505 maintained source entries. It uses bundled SQLite and the `storage-stage-timings` feature.
It does not qualify another native engine or a final consumer package.

The historical private collector remains unchanged. A fresh collector uses nonblocking open before the existing descriptor and identity checks.
Three real FIFO cases failed at the intended assertion with the historical reader. All 25 controls passed with the corrected reader.
A full parent reader passed 28 controls. It hashes the same bounded bytes that it parses and retains the complete coordinator record.
These tests do not establish hard wall-time bounds for regular filesystem reads. Outer deadlines remain necessary.

The first combined run passed 68 controls and failed two namespace-sensitive path checks.
Its `/workspace/ROM` import paths differed from the build's `/root/ROM` source identity.
The host repetition used the same 70 assertions and the actual mixed harness paths. All 70 controls passed in 438.255945 milliseconds.
Both attempts remain preserved. Their processes closed, and their owned cgroups were empty.

Independent source review also found a stale `current_harness517_exact` label. The fresh derivative now records `current_harness532_exact`.
The actual source count gate was already 532. Neither workload criteria nor source validation were relaxed.

## Mixed workload result

The workload retained 12,000 scheduled groups, 13,280 successful HTTP responses, and 12,000 completed Work records as acceptance criteria.
It retained active 32, pending 64, original phase deadlines, stream recovery, current authorization, and FULL durability.

| Observation | Actual result |
| --- | --- |
| Scheduled groups | 12,000 |
| Scheduling refusals | 5,213; all slots occupied, zero idle refusals |
| Traffic HTTP responses | 7,505; 7,495 success, nine denied, one unknown outcome |
| Operator HTTP requests | Four; all successful |
| Final durable Work | 11,476 records, all Done; 524 below the criterion |
| Normal stream recovery | 16 expected expiries and 16 fresh recoveries |
| Core overload counters | Zero |
| Native Work failures and dropped samples | Zero |
| Host, proxy, login physical exit | Zero, without signals |
| Client and outer mixed physical exit | One, without signals |

The first retained execution failure identifies `mixed-client` and route `other`.
It does not identify the chronological first HTTP failure. The unknown outcome remains unclassified.
Zero idle refusals rejects the earlier idle-dispatch hypothesis for this run.
The 524-record difference does not establish lost acknowledged Work. The client refused scheduled offers; per-command completeness needs its separate oracle.

## Native commit distribution

These are wall-time intervals around native Work commits. They are not exclusive CPU time or direct fsync measurements.

| Origin | Semantic change | Width | Native successes | Total interval |
| --- | --- | --- | --- | --- |
| Claim prefix | Unchanged | 0 | 2,845 | 10.670365 ms |
| Claim prefix | Changed | 1 | 1,464 | 27.943447018 s |
| Claim prefix | Changed | 2 | 6 | 0.110221445 s |
| Materialize | Changed | 1 | 1,060 | 18.298763912 s |
| Materialize | Changed | 2 | 6 | 0.125083117 s |
| Delivery started | Changed | 1 | 404 | 5.740166874 s |
| Delivery finished | Changed | 1 | 404 | 6.152675777 s |

Host counters record 1,072 Source claims and 404 Notification claims.
Six grouped materialization calls account for 12 Sources. The remaining 1,060 Sources used singleton materialization.
The counters establish that batching occurred. They do not distinguish sparse arrivals, same-root barriers, and Notification barriers for every singleton.
Idle native intervals represent approximately 0.0183% of the total Work commit interval.
Removing idle commits is not a credible primary optimization for this workload.

## Cleanup and evidence

Before and after provider GETs completed with physical exit zero. Their complete JSON values were equal in the same provider window.
ROOT recorded equality before requesting the exact owned stop file. Provider cleanup completed with physical exit zero.
Independent review found 47 recorded process births absent, seven cgroups empty, and five ports vacant.
Namespace cleanup passed. Original seed data and failed experiment evidence remain preserved.

Evidence paths below are relative to the repository unless they start with `/var/tmp/`.

| Evidence | Path or SHA-256 |
| --- | --- |
| Qualified diagnostic build | `.superpowers/rom-010-work-terminal-diagnostic-build-prep-20261009/build-current/` |
| Qualified controls | `.superpowers/rom-010-work-terminal-mixed-controls-host-execution-20261009/` |
| Full mixed coordinator | `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-213b800c63d7a350ac22609e/result.json` |
| Native Work receipt | `d2dbb48ce04193ce0f76710f103f3ac179c742fa2fedf99db03db26417bd975e` |
| Host Work receipt | `f5aa81b0f6e4713eac81ea1f9368aa546acb46d56de42baaef7536e7e7fd21e4` |
| Independent mixed review | `.superpowers/rom-010-work-terminal-mixed-preflight-review-20261009/mixed-execution-review.json` |
| Independent review SHA-256 | `2687467c8f385607873fb50d7ba30578a229ec2a94600fecdf34215d9ada2d2a` |

## Next investigation

A dense distinct-root control can distinguish batch capacity from legitimate same-root and Notification barriers.
The same ordering, idempotency, unknown-outcome, and durability assertions must remain in each variant.
No batching delay, larger admission limit, weaker durability, or reduced acceptance criterion is justified by this diagnostic.

The run also recorded 573,671 Storage load calls, with a total interval of 37.249465274 seconds.
Source inspection found repeated authority reads during query projection. Observation already checks authority before and after its commit-gated operation.
An operation-scoped authority proof is a candidate for investigation. It is not an implemented optimization or a measured cause.
Per-row policy, field authorization, expiry, local revocation, and final current-authority checks must remain effective.
Cross-request caching of authorization is not proposed.

Vocabulary follows the project's STE guidance and technical terms. This report does not certify official dictionary compliance.
