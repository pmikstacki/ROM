# Accepted-writer compatibility: independent review

Date: 2026-10-08. Scope: the R11 historical writer and reader subset.

The actual predecessor writer resolves the earlier marker-rewrite limitation. Two proof gaps remain before this subset becomes release acceptance evidence.
No product defect was reproduced in this review. The full 0.1.0 release remains open.

## Review method

I read the maintained writer, its copied build inputs, the current upgrade test, and the coordinator's report.
I inspected the raw build, upgrade, Clippy, metadata, lock, binary, and archive-control evidence.
I ran bounded Node SHA-256 and lock comparisons. I did not compile or rerun native tests because another worker owns that lease.
I applied the module and evidence rules in `docs/quality.md`, and the prose rules in `docs/writing.md`.

Evidence root: `/root/ROM/.superpowers/rom-010-predecessor-writer-sCBFeN`.
Historical source: `/root/ROM/.superpowers/rom-010-old-reader-rMxXNO/rom-0.0.3-584c01b61412`.

## Verified findings

| Item | Independent result |
| --- | --- |
| Accepted revision | `584c01b614127b3f62799f1a26a1cdf3f734c3dc` matches the accepted manifest and preceding reader witness. |
| Accepted tree | `7c35772c89aa5e78ca70b040adb58293236215a5` matches both witnesses. |
| Source archive | SHA-256 `f18922f9c2a6059570c0bff541deaf379f23c99a02c6185c33c8f3f965a45abe` matches the actual archive. |
| Selected historical source | All 442 files match the accepted manifest. |
| Writer package resolution | Metadata resolves ROM packages to the separate historical extraction, not the current workspace. |
| Dedicated lock | All 91 packages match accepted dependency names, versions, sources, and checksums, except the new fixture package. |
| Historical executable | Actual SHA-256 is `41347a1d2bf13787f90649427a6ae0ce71377761dccc37ccaf5cc711127e4b6b`, matching `binary-identity.json`. |
| Actual old population | Both writer logs record native 8 and archive 6. Source uses accepted public adapter and storage operations. It does not rewrite a marker. |
| Native and archive upgrade | Raw current log records one explicit test passing on both adapters, with zero failures or ignored tests. |
| Archive controls | Two positive archive-6 controls and two negative archive-7 controls exit zero. Original inputs and private copies match recorded hashes. |
| Exact rejection | The old reader requires `Error::Unsupported`; missing inputs and arbitrary failures cannot satisfy the negative control. |
| Honest ignore annotation | The maintained trial requires witnessed historical input. Its explicit invocation is distinguished from ordinary workspace success. |

The historical data contains two rows, including a tombstone, two mutation receipts, events, an effect, and two work records.
One work record has an active lease, delivery `Unknown`, and two attempts. The operator retry receipt and work root budgets are nonempty.
The current test checks predecessor cursor rejection and old delivery-claim rejection. It also checks retained values, fingerprints, and source database bytes.
The archive controls preserve archive bytes. They do not reopen or mutate a native database.

## Actionable proof gaps

### P2: assert the complete active-work recovery state

`compare()` checks active work payload, attempts, a newer generation, and absence of a lease.
It does not check the retained `delivery`, the exact replacement state, due time, or lifecycle revision.
A destination that loses `Unknown`, or incorrectly marks active work `Done`, could satisfy those assertions.

Extend the explicit trial to assert the documented recovery behavior for this `AtLeastOnce` input.
Assert retained `Unknown`, pending retry state, fenced generation, lifecycle revision, and the expected eligibility boundary.
Use a separate unresolved-delivery profile case if support claims include `AwaitingReconciliation`.
Keep these assertions separate from archive conversion, whose current assertions compare only rows and receipts.

### P2: capture the current-reader execution identity

The predecessor executable has source, package graph, and binary witnesses. The current-reader log identifies the test filename and binary basename.
It does not record the current executable hash, executed test source hash, or corresponding current dependency/source witness.
The preceding full verifier ran before this fixture. It cannot establish this new trial's source identity or final integration.

Before final acceptance, preserve a current-reader witness with exact command, environment, compiler, lock, source hashes, and executable hash.
Run the explicit historical-input gate again against that witnessed candidate. Run the combined verifier after integration.

## Provenance corrections

The preparatory witness captures 2,662 paths, including the fixture lock before dependency resolution.
My current hash comparison finds one changed captured path: `probe/Cargo.lock`.
Its initial hash is the accepted workspace lock, `a080b892d06dd30e65917a9a13715918cbd2b02fe64827f0334d7bde7005b441`.
Its final dedicated lock hash is `7e56bebe27436b3516feae1a2d5649694f383011f41dde0f0a524c14d2ff31e5`.
The final hash matches `lock-conformance.json`. Historical source and copied fixture source hashes remain unchanged.

Describe this as a planned lock-resolution transition, not as an unchanged capture of every build input.
Keep both witnesses. Do not rewrite the original preparation file.

The maintained writer source differs from the compiled copy through formatting and import order only.
The current Rust test source SHA-256 at review time is `f43b15a7fcde0831a98c8dea20a09d07d0ea3e2a05cf545241e51fb4b0f1601b`.
Do not describe maintained and compiled writer files as byte-identical. Record the formatting distinction or produce a fresh witness.

## Acceptance boundary

This evidence supports actual accepted-writer population, the asserted native upgrade behavior, and byte-preserving archive acceptance and rejection controls.
It does not establish full work-state preservation during archive conversion, whole-application cutover, historical custom-codec replay, rollback, or packaged release acceptance.
The planned 8 GiB target allowance was not an enforced quota. The report correctly distinguishes allowance from allocation.
No source, product, target, database, archive, failed trial, or historical evidence was deleted or changed during this review.

## Corrective review: review-corrected-v2

The coordinator supplied a fresh trial after the findings above. I independently inspected its source, witness, hashes, compiler output, and raw logs.
I did not compile or execute native code during this corrective review.

Evidence: `/root/ROM/.superpowers/rom-010-predecessor-writer-sCBFeN/review-corrected-v2`.
The earlier findings remain as historical findings. Both P2 proof gaps are resolved for this compatibility subset.

### Active work and archive equivalence

The current test reconstructs the entire predecessor active record. It permits only four documented changes.
Generation and lifecycle revision each increase by one. State becomes `Pending`. Due time becomes the original chain start time, `10`.
Full record equality retains `delivery: Some(Unknown)`, payload, attempts, and all other fields.
Nonleased work retains full record equality. The predecessor cursor and active delivery claim still fail against the upgraded destination.

The archive conversion now compares the complete serialized `Snapshot` with the predecessor snapshot.
This assertion includes work state, operator receipts, events, effects, descriptors, references, budgets, and fields beyond rows and receipts.
It establishes conversion equivalence for these populated inputs. It does not establish all possible work profiles or historical application codecs.

The first corrective attempt failed because its expected due time was `0`.
The preserved raw failure shows actual due time `10`, with the other reconstructed fields equal.
The current `eligibility_floor()` includes `cause.started_at`; this predecessor fixture has no additional delay.
The correction changes the test oracle to the original start time. It does not change storage or recovery implementation.
The first failure and its destinations remain under `review-corrected`.

### Current execution identity

I independently matched all 255 captured current source and manifest files to `current-source-before.json`.
I matched the current lock, both raw logs, witness document, and retained test executable to their recorded SHA-256 values.
All six input hashes still match their pre-execution capture. Each input also matches the original predecessor output.
These checks cover both native database files, both predecessor archives, and both logical summaries.

The captured test source SHA-256 is `1e84a367d981cdb08df7f70ba8df305f9f64ae32d38f4516b39b38d02116419f`.
The retained executable SHA-256 is `41082094010891141f2428208e57d331c417805cb71a2f25db1058cdd6b71f37`.
Its size is 80,710,328 bytes. The current lock hash is `f6ac31a111d67e8c4d1cc8bc0438ac0cf02bd87e851c78c86d6e2756b1241bf3`.
The compiler record identifies Rust 1.99.0, commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, on `x86_64-unknown-linux-gnu`.

The witness records the explicit historical-input command, locked graph, two build jobs, disabled incremental compilation, and 120-second deadline.
The raw test log records one passed test covering SQLite and redb, with no failures or ignored tests.
The matching Clippy log completes without warnings. The witness retains `terminal: 0` and `admitted: false`.

### Documentation and remaining limits

The coordinator's report now distinguishes the preparatory lock from its final resolved lock.
It also distinguishes the formatted maintained fixture from the frozen compiled copy.
One passage still describes full conversion work-state equivalence as outside the assertions. That passage refers to the earlier trial and needs updating.
The case table should point to the corrective logs for the stronger current result. Preserve the earlier logs as historical evidence.

This corrective subset has no remaining blocking finding from my review.
Whole-application upgrade and cutover, custom-codec replay, rollback, full combined verification, and final packaged release acceptance remain open.
An ordinary workspace run still cannot replace the explicitly executed historical-input gate.
