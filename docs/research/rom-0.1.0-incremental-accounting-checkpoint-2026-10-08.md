# Incremental Work accounting checkpoint

Date: 2026-10-08. Status: tested helper and preserved reference; production integration remains open.

The helper computes canonical current and lifecycle-reserved JSON contributions for affected work and root entries.
It accounts for escaped map keys, separators, empty maps and the serialized limits envelope.
Replacement arithmetic uses checked totals and publishes only after validation.
Prior contributions must come from a coherent trusted transaction read. Aggregate bytes cannot authenticate a supplied prior record.

The original lifecycle reservation transform remains unchanged: maximal numeric widths, reserved Leased state, Retryable delivery and maximal root use.
Six differential tests compare helper totals against independent whole-ledger serialization.
They cover insertion, replacement, removal, states, delivery outcomes, optional floors, payloads, capacity limits and rejected arithmetic.

Three reference tests compare the unchanged production ledger with byte-preserved old implementation copies.
They cover all five update variants, duplicate enqueue, stale claims, atomic ID-ordered prefix effects, late overflow and orphan-root rejection.
These baseline tests do not establish equivalence of the forthcoming transition engine.

## Executed checks

| Evidence | Observed result |
| --- | --- |
| `/var/tmp/rom-010-work-accounting-api-red.log` | Initial missing-contract compile failure retained |
| `/var/tmp/rom-010-work-accounting-work-clippy.log` | 41 Work tests passed; scoped Clippy then rejected a test initializer |
| `/var/tmp/rom-010-work-accounting-final-focused.log` | Six accounting tests, two reference tests and scoped all-target Clippy passed after the initializer correction |
| `/var/tmp/rom-010-work-accounting-reference-three.log` | Three reference tests passed after adding the orphan-root control |
| `/var/tmp/rom-010-work-accounting-frozen-source-hashes.txt` | Final accounting/reference source identity; recorded before the later shared predicate visibility change |

No complete 42-test rerun or full local verifier is claimed by this checkpoint.
The helper is currently compiled only in tests, until a production transition caller uses it.
The preserved reference modules remain byte-identical to [the captured manifest](../../tests/persistence/reference-work-20261008/SHA256SUMS).
The test-only oracle permits unused frozen methods in its own module. No production warning allowance was introduced.

## Next integration

Implement transaction-local validated deltas using the [reviewed contract](rom-0.1.0-incremental-work-contract-2026-10-08.md).
Compose the reference ledger with its enclosing retry-epoch validation when comparing storage-level transitions.
Ledger-only Action payload examples are not valid storage-level epoch fixtures.
Keep the completion predicate shared with retention; only its module visibility changed, not its behavior.

Then integrate both native adapters, canonical archive reconstruction and explicit version conversion.
Repeat the unchanged workload from [the attribution report](rom-0.1.0-retained-work-attribution-2026-10-08.md).
No latency improvement or production release readiness is established by the helper tests.

## Bundle module checkpoint

The existing StorageState::bundle method moved byte for byte into `storage_state/bundle.rs`.
Its captured method SHA-256 is `bffc71b49718a62a020c04c8228a94552fea2bc0087e1060d55fafa8b4388adb`.
The public method path and operation order did not change.
Thirty-eight storage_state tests passed after extraction. Production-only core Clippy and targeted formatting checks passed.
Four temporary test-only incremental-engine unused warnings remain in the test log; all-target Clippy is not claimed here.
Evidence: `/var/tmp/rom-010-bundle-extraction-tests.log`, `/var/tmp/rom-010-bundle-extraction-clippy.log` and `/var/tmp/rom-010-bundle-extraction-identity.json`.
The full verifier and actual incremental bundle integration remain pending.
