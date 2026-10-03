# Native format upgrade plan

**Goal:** Upgrade native format-3 stores into fresh format-4 destinations without changing the source or Resource values.

**Architecture:** Each adapter reads its older format in one coherent read-only transaction. Shared maintenance code binds explicit descriptors and validates the complete graph. Existing restore publication creates the new destination.

**Spec:** `openspec/changes/prepare-framework-release/specs/framework-readiness/spec.md`, data lifecycle and interrupted upgrade.

## Contracts

- Both adapters expose `upgrade_from(source, destination, descriptors, limits) -> Result<Self>`.
- The source must have the exact format-3 inventory and marker. Ordinary open continues to reject older formats.
- The complete descriptor catalog is supplied explicitly. ROM does not infer reference fields from JSON values.
- `rom_backup::bind_legacy_schema` validates and binds the catalog, derives current live edges and validates the resulting snapshot within output limits.
- Rows, revisions, receipts, fingerprints, events, effects and work payloads retain their values. New relation metadata enforces restrict.
- Destination restore rotates the journal generation and fences active work claims. Unknown outcomes and remaining attempt budgets retain their meaning.
- Native readers use read-only APIs. SQLite can use shared-memory locks while reading WAL; no migration SQL writes to the source are allowed.
- A new destination is published only after native reconstruction and validation. Existing paths and symlinks are not overwritten.
- Under `test-support`, `upgrade_from_observed` accepts a callback immediately before publication. A child exits there without destructors to test interruption.
- Keep `lib.rs` as a facade. Share collection and publication mechanics with existing backup and restore; isolate version-specific inventory checks.

## Tests and implementation

- [x] Observe failing tests for the missing native upgrade and shared binding APIs.
- [x] Implement shared legacy binding, record preservation and output-budget tests.
- [x] Implement read-only SQLite and redb collection plus shared restore publication.
- [x] Test both native formats, active/pending work, receipts, reference enforcement, invalid schemas, missing targets, limits and destination refusal.
- [x] Terminate child processes before publication; verify source preservation and successful recovery into a fresh destination.
- [x] Independently review the code and run local checks before integration.

See [results and verification](../../research/native-format-upgrade-results.md).

This completes a storage-format upgrade slice. General Resource value transformations,
dependency-aware retention and the reference application upgrade journey remain required.
