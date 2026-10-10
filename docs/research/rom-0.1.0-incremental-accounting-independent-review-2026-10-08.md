# Independent incremental accounting and reference review

Date: 2026-10-08. Scope: accounting helper, tests, frozen reference modules, and recorded focused native logs.
No native command or product edit occurred during this review.

## Result

No source blocker was found in the reviewed arithmetic helper or frozen reference capture.
The helper calculates exact canonical entry contributions and preserves its prior state when an update fails.
It is currently compiled only under cfg(test). It does not change production WorkLedger behavior or provide a measured speedup.
The reference tests establish a baseline for future differential tests, not equivalence of an unimplemented incremental engine.

## Accounting behavior

`entry_length` serializes the map key and complete value through serde_json, then adds the colon with checked arithmetic.
Map totals separately add one comma between entries. The empty WorkLedger serialization supplies the exact envelope and frozen limits encoding.
Root values reserve u32::MAX. Work entries reserve the same fields and shapes as the current whole-ledger reservation implementation.
Those include maximum attempts, generation, revision and due, the specific Leased state, and Retryable delivery.
Frozen PendingWork, payload, ID, and optional not_before remain unchanged.

The helper maintains separate current and reserved totals. Bounds include work-record count and both byte totals.
Root count contributes bytes but does not consume the existing max_records limit.
The zero-entry map check detects residual entry bytes after removing the last item.
Each public replacement mutates a private copy, checks complete totals, and publishes only on success.
Underflow returns Storage; arithmetic overflow returns TooLarge. Rejected changes leave the original helper unchanged.
The comma calculation uses saturating subtraction only to express zero commas for zero entries; byte and record arithmetic does not saturate.

This helper is not an identity or semantic validator.
It receives prior contribution sizes rather than exact authenticated keys and records.
A wrong prior contribution can evade rejection in a multi-entry map if aggregate arithmetic remains plausible.
The eventual transition reader must derive contributions from coherent authoritative records, not caller-supplied untrusted totals.
Frozen limits must match the header used to construct the envelope. Passing unrelated limits to check_bounds does not rebuild that envelope.
Import must independently recompute canonical totals and validate the complete graph before local arithmetic becomes authoritative.

## Test coverage and limits

Six accounting tests compare totals with whole canonical serialization after insert, replace, and remove operations.
They cover escaped and Unicode keys, all declared WorkState/StopReason/DeliveryOutcome values, optional floors, and Source/Action/Notification payloads.
The reservation oracle clones the whole ledger and applies the original reservation transformation independently of the helper.
Error tests cover invalid zero contributions, missing removal, residual bytes, arithmetic underflow, overflow, and atomic rollback.
Boundary tests compare helper bounds with current WorkLedger bounds and test the exact accepted byte threshold and one byte below it.

Some accounting fixtures are deliberately invalid domain ledgers, including orphan roots and incompatible delivery states.
They test encoding arithmetic, not admission or archive acceptance.
The separate reference test covers valid zero-attempt member roots and rejects orphan-root metadata.
The maintained tests are deterministic examples, not exhaustive arbitrary-payload property testing.

## Reference independence

All five preserved files match `tests/persistence/reference-work-20261008/SHA256SUMS` byte for byte.
All five compiled files under `reaction_work/reference/` match the same preserved hashes.
They contain the old transitions, control, reservation, restore and retention implementation rather than wrappers around the replacement helper.
This provides an independent transition implementation for the next engine.

The reference imports current shared model types and serde definitions through its facade.
A future shared-type or serialization change can affect both implementations despite frozen transition bytes.
Preserve the captured files and compare canonical serialization against historical fixtures when those shared definitions change.
The current baseline tests cover all five updates, duplicate enqueue, stale claims, ID-ordered prefix effects, late overflow rollback, and orphan roots.
They do not yet cover the complete planned bundle, operator, epoch, retention, uncertainty, and native failure matrix.

## Actual evidence

The earlier `work-clippy` log reports 41 passing reaction-work tests, then a Clippy failure in a test initializer.
That failure is preserved. It is not a successful Clippy gate or an intentional behavioral RED.
The subsequent focused log reports six accounting tests and two reference tests passing, then successful scoped Clippy completion.
The later reference log reports three passing reference tests, including the added orphan-root case.
No current complete 42-test rerun or full verifier result is established by these logs.

| Log under `/var/tmp/` | SHA-256 |
| --- | --- |
| `rom-010-work-accounting-work-clippy.log` | `4bd45ada8079f6328f84ac9c13c47294d5841da1506109a4a6be12eca9244a9b` |
| `rom-010-work-accounting-final-focused.log` | `7b3b527b2f628fd23be6703102d7fc9991f033c1e05d17570036d990d8bfb9c2` |
| `rom-010-work-accounting-reference-three.log` | `9ba226d1d8b69cf5dbd4456545957e95171f15d379f7387497bf6766019faecb` |

The audit independently checked `/var/tmp/rom-010-work-accounting-frozen-source-hashes.txt` against current files.
The helper, its tests, reference facade/tests, all five reference implementations, original ledger, and Cargo.lock still match.
The reaction_work module facade changed during the authorized engine work and no longer matches that earlier inventory.
Do not treat the earlier inventory as a complete current-source execution fence.

| Reviewed source | SHA-256 |
| --- | --- |
| `reaction_work/accounting.rs` | `608d5c4724b3e63b6d42f54362443343c9fa84d7cd8030f41e0a1c00b157262b` |
| `reaction_work/accounting_tests.rs` | `d406e276b97d10febfc53df60aa8517a4163fc95b3e06105d6efbba5bea11877` |
| `reaction_work/reference_tests.rs` | `6e6363813cb3184652cd241a1ddf7d2dbf80e8d304c4bba7a21267bab0bd2a2f` |

These source paths start with `crates/rom/src/`.
Production integration, complete differential sequences, native persistence, final-source evidence, and performance acceptance remain open.
