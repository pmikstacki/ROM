# Accepted-release writer and archive-reader trial

Date: 2026-10-08. Status: executed compatibility subset; release admission remains open.

The earlier populated predecessor test rewrote a current writer's storage marker.
That test did not establish compatibility with data written by the accepted 0.0.3 executable.
This trial uses a separate executable compiled against the verified accepted release source.

## Source and execution identity

Accepted revision: `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
The verified extraction and archive identity come from the preceding [reader trial](rom-0.1.0-previous-reader-rejection-2026-10-08.md).
The new evidence directory is `/root/ROM/.superpowers/rom-010-predecessor-writer-sCBFeN`.
Its `preparation.json` records source and fixture hashes before compilation.
The coordinator compared source and fixture hashes again after compilation. The accepted source and compiled fixture source remained unchanged.
The preparation records the lock before dependency resolution. Its final resolved lock has a separate hash in `lock-conformance.json`.
These lock identities are different. The final graph retains the accepted dependency versions.

The dedicated dependency graph has 91 packages. Every dependency version matches the accepted release lock.
Its exact comparison and lock hash are in `lock-conformance.json`.
Executable size and SHA-256 are in `binary-identity.json`.
The build used two jobs, no incremental compilation, offline dependencies, a locked graph, and a 180-second command deadline.
The build exited zero. The separate target occupied 254,068 KiB after execution.
The planned allowance was 8 GiB. This allowance was not an enforced filesystem quota.

## Executed cases

| Case | Result | Evidence |
| --- | --- | --- |
| Actual 0.0.3 SQLite writer | Native 8 and archive 6 created | `sqlite-writer.log` |
| Actual 0.0.3 redb writer | Native 8 and archive 6 created | `redb-writer.log` |
| Current native upgrade on both adapters | Passed; source bytes retained | `review-corrected-v2/current-upgrade.log` |
| Current archive conversion on both adapters | Archive 6 converted into fresh archive 7; source retained | `review-corrected-v2/current-upgrade.log` |
| Previous archive reader, positive controls | Both archive-6 inputs accepted | `archive-reader-results.json` |
| Previous archive reader, negative controls | Both archive-7 inputs rejected with `Error::Unsupported` | `archive-reader-results.json` |
| Current maintained trial Clippy | Passed with warnings denied | `review-corrected-v2/current-clippy.log` |

The current native trial compares rows, receipts, events, effects, descriptors, references, operator state, and work root budgets.
The live Resource and tombstone retain their values. Both original mutation fingerprints remain available.
Active delivery claims receive a newer generation and cannot control the upgraded destination.
Predecessor journal cursors return `HistoryGap`. Original pending work and attempt counts remain intact.
The archive conversion compares the complete logical snapshot, including work and operator state.
Native restore compares the complete recovered work record. It retains `Unknown`, increments claim generation and lifecycle revision, and restores the original eligibility floor.

Independent review identified weaker work assertions and absent current-reader execution identity in the first trial.
The corrected trial records 255 current source files, the final lock, the exact detached test executable, compiler information, logs, and six input files.
Before and after hashes match. See `review-corrected-v2/current-execution-witness.json` and the [independent review](rom-0.1.0-accepted-writer-independent-review-2026-10-08.md).
The first corrective run expected a zero eligibility floor. It failed because the original chain started at time 10.
That failed run remains in `review-corrected`. The fresh corrected trial uses the original chain start and passed on both adapters.

The four previous-reader archive controls compare SHA-256 before and after each invocation.
All four archives remained byte-identical. Missing paths cannot satisfy an archive control.
The negative control requires the actual predecessor reader's unsupported-format result, rather than any failure.

## Maintained execution and limits

The writer fixture is in `tests/persistence/support/predecessor-writer`.
The maintained fixture received formatting changes after its compiled copy was frozen. The compiled copy remains preserved with its exact source hashes.
The current trial is in `tests/persistence/tests/predecessor_writer.rs`.
The explicit test ran with `ROM_PREDECESSOR_EVIDENCE` and `--ignored`; one test covered both adapters and exited zero.
Its ignore annotation prevents ordinary workspace tests from pretending to possess a witnessed historical writer.
Default workspace success does not establish this historical-input gate.

No storage or upgrade implementation changed in this increment.
The preceding combined verifier ran before these new fixtures; it does not verify their addition.
Whole-application cutover, rollback, historical application codec replay, and final packaged release verification remain open.
The native build lease returned to the application recovery worker after the matching Clippy check completed.
