# Public diagnostics consumer: independent review

Date: 2026-10-08.

## Outcome

No new blocking defect was found in the reviewed diagnostics boundary or public consumer tests.
This is a source review and recorded-evidence audit. The reviewer did not run Cargo or change implementation files.
It does not establish final package admission or a current full verifier result.

## Reviewed behavior

`examples/consumer/tests/diagnostics.rs` uses public imports from `rom`, `rom-consumer`, and `rom-sqlite`.
Its host drains records without an exporter callback in the core.
The exporter fixture owns a reader, not a Runtime or database handle.

The create, replay, and action case checks two commits, matching logical correlation, advancing local sequence, and stored effect counts.
It serializes all drained records and rejects known private title, note, principal, Resource ID, and request keys.
These assertions inspect diagnostic records. They do not inspect an arbitrary external transport or claim zero secret copies in process memory.

The capacity-one case observes dropped records and completes commands after reader closure.
Commands and shutdown have two-second ceilings in that case.
The stalled exporter case completes another command and shutdown before releasing its exporter.
The fixture then returns `BrokenPipe` and closes its reader.
It proves independence from that host-owned exporter. It does not simulate a production network exporter.

## Source boundary checks

The fixed `DiagnosticEvent` contains categories, opaque tokens, counters, and checked timing.
It has no Resource payload, principal, raw identifier, or application error text field.
`DiagnosticKey` has no Debug or serialization implementation.
Correlation uses HMAC-SHA256 with versioned, length-delimited domains.
Stable operation and root inputs preserve replay links. Claim generation changes the claim token while retaining the work token.

The queue uses `try_send`. Full or closed queues update loss counters without waiting for a reader.
The collector holds at most 32 records. Queue capacity is restricted to 1 through 4096.
Dropping a collector does not invoke an exporter or publish records.

Command, reaction, and notification wrappers publish after their guarded inner functions return.
The record collector does not write a receipt, event, bundle, or durable diagnostic payload.
Shutdown releases its lifecycle guard before publication.
Cancellation may leave only a Started shutdown observation; the public documentation explicitly preserves this limit.
Unknown commit outcomes remain Unknown rather than diagnostic confirmation of failure or success.

The earlier corrected Resolve, StorageRead, Receipt, and Validation stages remain present.
The public timing description remains a supervised stage interval. It does not promise callback-only CPU duration.

## Independently checked evidence

All hashes below matched the current files or preserved native artifacts.
Native logs were read through `/var/lib/nixos-containers/rom-dev/var/tmp/`.
The container's bind-mounted executable was read at the host path `target/debug/deps/diagnostics-f0d665d3483f4932`.

| Evidence | SHA-256 |
| --- | --- |
| Consumer test source | `d8a595a180e93240f541a100f0c8d21906541912ff505d7e2db214d839096c75` |
| Root lock | `a58a4ccca73fee85836e6f6a8052d59432f92f10dbd03fabfe09d6111b149ae4` |
| Native test executable | `1a8b99ed85fc7abbb9201e5e863cbe869f793b60874cd89c5d6d9afeb283cb46` |
| Initial compile log | `43ccc9bc9e6df5f0036c8d02bc584b6ce557a08cd75ad4f5990d9393440c2c40` |
| Corrected test log | `e7e48d2c585b0652280ddd5cdfcc5a4cc42aa8d75d48782bfef9dc9c46b869c0` |
| Scoped Clippy log | `96985f61f579ddbd2d58e2768b104fbb1127d7f587b72f45cbc861365a83bac2` |

The corrected log contains three passing tests, zero failures, and zero ignored tests.
The scoped Clippy log ends with successful completion.
The initial failure was a compile error from comparing snapshots without `PartialEq`.
It is preserved compile evidence, not a behavioral regression RED.

## Remaining release checks

Repeat these public consumer tests against the matching extracted final release packages.
The recorded tests use workspace dependencies and SQLite.
The separate conformance suite covers redb, but this consumer run does not independently repeat that adapter scope.

Measure overhead on final source. Earlier measurements do not establish current overhead, allocation cost, or production exporter performance.
Run the current full local verifier and obtain independent release acceptance.
Production hosts must supply protected key material, unique stream sessions, exporter deadlines, redaction, and disposal limits.
The consumer uses an explicit synthetic key and session. It does not provide production key management.

## Fresh workspace evidence audit

The subsequent capsule `.superpowers/rom-010-final-diagnostics-6Ffo1E` repeats the three public tests and scoped Clippy successfully.
An independent audit verified the raw log hashes and all 591 current input hashes against the capsule manifest.
The corrected 18-row overhead record also matches the raw measurement objects exactly.
The [diagnostics review](rom-0.1.0-diagnostics-independent-review-2026-10-08.md#fresh-measurement-evidence-audit) records the matrix, sums, hashes, and measurement limits.

This closes the previously missing fresh workspace overhead subset for that unchanged source set.
It does not establish installed final-package coverage, full local verification, production exporter performance, or production key management.
Future compiled source changes require matching verification. Release admission remains false.
