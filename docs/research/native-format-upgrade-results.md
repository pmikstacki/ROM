# Native storage format upgrade

Status: implemented and verified for native format upgrades. The release goal remains open.

## Contract

Both native adapters expose `upgrade_from(source, destination, descriptors, limits)`.
The operation converts exact format-3 stores to format 4 in a fresh destination.
It requires an explicit descriptor catalog and preserves Resource values.
See the [operator procedure](../native-upgrade.md).

The adapters collect a coherent snapshot. Shared backup code binds descriptors,
checks values and derives live reference edges. Existing restore code rebuilds and
validates the native destination. Shared publication code syncs the staged file,
runs the interruption hook and publishes without overwrite. Native close and
checkpoint operations stay inside each adapter.

This is a storage-format upgrade. General Resource value transformations, retention
and the reference application's complete upgrade journey remain release requirements.
Release checklist item 2.2 is still open.

## Defects found during review

A test showed that normal redb open could alter an unsupported legacy file before
it rejected the old marker. A read-only preflight now checks the format first.

An unclean redb source can require physical recovery. A second failing test showed
that a read-only open alone could not upgrade that source. ROM now recovers a
private temporary copy and reads its snapshot. The original file stays unchanged.
This path needs temporary disk space approximately equal to the physical file size.
Clean sources do not need the copy. The logical data budget does not bound this
physical disk requirement.

Review also found duplicated file sync and publication operations. The shared
`Stage::publish_with` method now owns that sequence. Adapter code still owns native
reconstruction and closing. All 14 maintained `lib.rs` and `mod.rs` files passed
independent inspection against the facade-only rule. Rust-required procedural macro
entry functions delegate to the expansion module.

## Test evidence

The native upgrade suite passed eight tests. Two ignored child helpers are invoked
by parent tests. The cases cover:

- Exact rows, receipts, events, effect intentions and nonempty protected provenance.
- Reopen, persisted references and restrict deletion on SQLite and redb.
- Missing descriptors, dangling references, byte limits and record limits.
- Existing destinations, symlinks and missing sources.
- Process exit before publication, unchanged source and retry into a new destination.
- Committed SQLite WAL contents and private recovery of an unclean redb source.
- Active claims, preserved attempts and uncertain delivery outcomes.
- Rejection of old claims and old journal cursors after restore fencing.

The independent persistence suite passed 112 tests across 13 binaries. Three
ignored process helpers are invoked by their parent tests. Shared backup binding
passed eight tests. The fixtures construct exact older native layouts; these tests
do not certify every historical ROM binary or physical power loss.

Evidence in `rom-dev`:

- `/var/tmp/rom-native-upgrade-conformance-final.log`
- `/var/tmp/rom-native-upgrade-intentions.log`
- `/var/tmp/rom-dirty-upgrade-red.log`
- `/var/tmp/rom-native-upgrade-full-check.log`

These runs used Rust/Cargo 1.99.0, the unchanged lockfile and the native-upgrade
changes on top of `e5ca29a`. Sources were in the `release-relations` worktree.

The coordinator's final `./scripts/check` exited successfully after the publication
refactor. It ran strict OpenSpec validation, formatting, Clippy, workspace tests,
doctests, Rustdoc, dependency isolation, consumer execution, compile fixtures and
isolated auth/identity checks. The existing real-MinIO cases remain opt-in and were
not run in this pass.
