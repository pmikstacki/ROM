# Independent archive/native marker compatibility review

Date: 2026-10-08. Scope: current archive dispatch, native-layout compatibility tests, and supplied backup logs.
No product edits or native execution occurred during the review.

## Result

The current reader explicitly accepts only archive/native pairs `(7, 9)` and `(7, 10)` through the canonical version-7 decode path.
It verifies one bounded private envelope before dispatch. There is no error-based retry under another format interpretation.
The decoded manifest keeps the actual input marker; validation compares counts and exclusions against that exact supported pair.
The complete Snapshot validation remains unchanged.
No source blocker was found in this compatibility branch.

The shared STORAGE_FORMAT constant still equals 9 at this checkpoint.
Therefore current ordinary backup writers still emit `(7, 9)`.
The tests construct valid `(7, 10)` envelopes to verify the new read branch.
They do not prove an implemented native-10 layout, a native-10 writer, or actual accepted predecessor conversion.

## Maintained test scope

The positive test uses a populated canonical fixture with work, operator receipt, and delivery-related data.
It tests both backend tags and both supported markers.
It compares the complete returned manifest and body, then checks the source archive bytes remain unchanged.
The negative test rejects unsupported pairs, an unknown manifest field, incorrect operator receipt count, and an orphan root.
It also rejects wrong backend and an envelope one byte over the configured maximum.

These are fixture archives created by the unchecked test encoder with valid envelope hashes.
They are not historic accepted release artifacts or an actual old Runtime-produced database.
The tests do not demonstrate current authorization, custom-codec replay, native dirty-file recovery, or old-writer refusal.
Those remain native upgrade and packaged-consumer requirements.

The existing full backup suite covers other envelope/publication and historical upgrade cases.
Passing that suite does not automatically extend every old negative case to both new marker branches.
Add branch-specific corrupt-hash/count/record-limit coverage if integration changes shared decode or accounting behavior.

## Actual evidence

The genuine RED rejects marker 10 with Unsupported in the positive pair-preservation case.
The corrected full backup suite reports 32 passing tests, zero failures, and zero ignored cases in 0.02 seconds.
The separate scoped Clippy log reaches successful completion.
The audit independently read the raw logs and calculated their hashes.

| Host evidence | SHA-256 |
| --- | --- |
| `/var/tmp/rom-010-backup-native-marker-red-20261008.log` | `5a78ed2082ee955fb9c79cb5f8ecce98c8c4ccb255309fbd05fba698f155c6ac` |
| `/var/tmp/rom-010-backup-native-marker-green-20261008.log` | `9d1e6df46894c2e03356de7992f5474d58ce5cefcdd695d38ec06489e8cef65d` |
| `/var/tmp/rom-010-backup-native-marker-clippy-20261008.log` | `c404d41613bff6407850781f2b724fe10ae9967428c91b27362f5f32fbfd615b` |

The [format decision review](rom-0.1.0-incremental-format-decision-review-2026-10-08.md) remains the compatibility plan.
Physical-summary/index charges, bounded canonical reconstruction, native-9 conversion, actual historical archives, and native-10 publication remain open integration gates.
No speedup, full verifier, final source freeze, or release admission follows from this checkpoint.
