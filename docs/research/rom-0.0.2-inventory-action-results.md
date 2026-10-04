# Inventory action through the shared Studio contract

## Acceptance gap

The demo declared a Task action, but no Inventory action. The browser plan requires actions on both Resource types.
The demo now declares `Action<InventoryItem, u64>` named `restock`. Its scalar input uses the shared descriptor and action form.
The callback uses checked addition before assignment. An overflow returns an invalid-input error.
No endpoint, controller, persistence route or core action rule changed.

## Failure-first test

The native test first failed because Inventory discovery did not contain `restock`.
After declaration and registration, the test passed through the generic invocation path.
It creates stock at `u64::MAX - 1`, applies one unit and reads revision 2 at `u64::MAX`.
Another unit fails without changing native state, receipt, event or effect counts.
Replaying the original successful request returns the same row and does not change counts.

The first green run had an unused test import warning. Independent review found it before the strict gate.
Only the unused import was removed. The final focused test and demo Clippy gate passed with warnings denied.
The warning log remains separate from the final strict result.

## Evidence and scope

- [Missing action RED](evidence/rom-0.0.2/inventory-action/red.log).
- [Initial test and Studio-feature build](evidence/rom-0.0.2/inventory-action/green-build.log).
- [Final test and strict Clippy](evidence/rom-0.0.2/inventory-action/final-tests-clippy.log).
- [Source identity](evidence/rom-0.0.2/inventory-action/source.sha256).
- [Immutable binary identity](evidence/rom-0.0.2/inventory-action/native-binary.sha256).
- [Both-store/both-browser workflow](rom-0.0.2-full-browser-workflows-results.md).

The immutable acceptance binary is `/var/tmp/rom-studio-inventory-action-acceptance/rom-demo`.
Its SHA-256 is `c93a20cfc1d92cc6e3e6f613105bbbcc61468179008fc118fafabb994d1c88f3`.
It was built before removal of the test-only import. Product code did not change after that build.

The commands used the existing shared native target, one Cargo job, the default profile and no incremental compilation.
The 92 GiB checkpoint ran before each compiler invocation. These focused commands are not the full release producer.
The native arithmetic test uses SQLite. The complete browser flow uses SQLite and redb through the same declared action.
