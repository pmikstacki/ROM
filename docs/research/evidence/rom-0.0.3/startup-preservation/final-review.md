# Final independent startup preservation assessment

Date: 2026-10-05.
Clean reviewed source: `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
Result: no material blocker found in the correction or its recorded startup proof.
The corrected source preserves the library disclosure and mutation contracts.
The final complete producer, independent artifact acceptance, and corrected live deployment remain pending.

This is read-only source and evidence assessment.
No tests, builds, browser actions, database writes, service changes, or repository edits were performed by this reviewer.
Private identifiers, configuration, profiles, paths to stored attachments, and secrets are excluded.

## Exact source identity

The worktree was clean at review.
All four corrected files match the hashes from the earlier frozen source review and the copied-upgrade receipt:

| File | SHA-256 |
| --- | --- |
| `demo/src/studio_startup.rs` | `9772fcaed8212e85e15f717f3caf306f0c02fa179af1381dd1634e102b6c9878` |
| `demo/src/studio.rs` | `e722aafb574668b2752585db07607fa41dcaff25e672532d05ed3f827c336aaa` |
| `demo/src/studio_startup_tests.rs` | `c2f567ffbd358f1a2c5bfc47df62dee3f12c905416ac997e66442032236b4860` |
| `demo/src/lib.rs` | `c5ad89be63c3728fa1c6f4424070ae0c1b7885fe7460b16cf6065292e6ed6ae9` |

There is no library source diff from accepted base `c66c470` to this clean candidate.
The host uses its exact selected Storage instance for startup inspection.
An existing Row, including a tombstone, is preserved without historical create-receipt replay.
An absent row still uses the ordinary Runtime create, current authority, revision arbitration, reference validation, and atomic commit.
Runtime.establish_actor runs before existence-only provisioning, retaining lifecycle and admission failure.
Storage errors propagate rather than becoming absence or success.

The optional absent showcase selects only a stored live target from the finite declared Task seed candidates.
Existing showcases remain unchanged. If all prerequisite Tasks are deleted, the optional fixture remains absent.
This avoids resurrection and does not weaken ResourceRef integrity or core tombstone disclosure.
No cached state, synthetic receipt, new authorization exception, or library contract change was introduced.
Physically purged rows remain outside the once-ever provisioning guarantee; ordinary create and retention/retry rules remain authoritative.

## Development verification and actual copied restart

The public startup evidence retains three distinct failures: the initial reference-restrict fixture failure, the intended tombstone-replay Denied failure on both adapters, and the missing-fixture prerequisite failures.
The final GREEN log records eight passing focused tests on SQLite/redb.
They preserve exact rows, tombstone revision/protected metadata, journal heads, and state/event/receipt/effect counts after reopen and repeated bootstrap.
Closed-runtime and failed-lookup cases retain their exact errors and unchanged counts.
The full demo tests and all-target/all-feature Clippy passed in the preserved record.
The public full local verifier record and owner result report a pass after the production correction.
These records were inspected; they were not independently executed by this reviewer.

The private copied-upgrade receipt was inspected without reproducing private fields.
Its aggregate results match the public `copied-upgrade.json` exactly.
The recorded optimized binary exists and its bytes independently match SHA-256:
`ddeda5f8674333da679703674d679173553fad6294b0a559b7c22b8525109c2b`.

The first copied-store startup returned HTTP 200.
The receipt reports all 13 existing rows preserved, including the deleted Task tombstone.
Provisioning added one absent synthetic showcase, one event, and one receipt, with effects unchanged.
The second process startup returned HTTP 200 and exited zero, with ledger counts unchanged.
The receipt reports that original state was not changed.
This actual process proof supplements the native regression tests and supports the intended upgrade/restart behavior.
It does not constitute a fresh clean-source release binary, public authenticated browser journey, or live deployment acceptance.

Private receipt SHA-256, without copied private content:
`31d84831e771a0103a11e5bea772cbcaa3a16c7041148acd1ea9b2160e289f3c`.
Public aggregate receipt: `docs/research/evidence/rom-0.0.3/startup-preservation/copied-upgrade.json`.

## Goal and final gate status

OpenSpec tasks 5.6 and 5.7 remain unchecked in this clean source.
The release plan retains complete production, independent artifact verification, and final requirement reconciliation.
The README explicitly describes the accepted older candidate and the pending restart correction.
The older completion record preserves its accepted artifacts and healthy preview, then records the later deletion/restart finding and repeated acceptance requirement.
The startup note calls the optimized copied-data binary a private development proof, not the final release binary.

These status distinctions are accurate.
The source and focused evidence establish the correction, while its final delivery and corrected deployment remain pending.
The previous runtime identity bridge cannot accept this changed production binary.
The active producer must complete all required gates on the corrected clean source.
The final artifact must pass independent verification, and current live authentication/restart acceptance must identify the corrected runtime and assets.
No completed-goal or completed-release claim is justified by this report alone.
