# Transactional restrict relations implementation plan

**Goal:** Enforce declared Resource references in each native persistence transaction.

**Architecture:** The core supplies validated descriptors and recursive reference extraction. Each adapter stores descriptors and an indexed reference graph. Native transactions enforce existence and restrict deletion with the Resource commit.

**Spec:** `openspec/changes/prepare-framework-release/design.md`, stage 2; `docs/research/restrict-reference-integration-plan.md`.

**Execution:** Continue the owner's authorized release goal with parallel tests, adapter implementation and review. Use TDD for each behavior.

## Fixed contracts

- `Storage::register(&self, descriptors: &[Descriptor]) -> Result<()>` binds descriptors before Runtime intake starts. The default implementation returns `Unsupported`.
- Native stores reject a new commit for an unregistered kind. Receipt replay precedes schema and reference validation.
- Registration adds empty kinds or accepts an identical stored descriptor. Changed descriptors and existing unbound rows require explicit migration.
- Registration retains descriptors for kinds omitted from a later Runtime. Incoming references from those kinds still restrict deletion.
- `Descriptor::reference_targets(&self, value: Option<&Value>) -> Result<Vec<Key>>` validates the layout and canonical value. It returns sorted, unique targets.
- Optional, nullable, list and map fields retain their declared semantics. Tombstones have no outgoing references.
- Target checks, incoming restrict checks, edge replacement and the existing commit bundle share one native transaction.
- A self-reference checks the candidate live row. Its own edge does not block deletion. Other live sources do block deletion.
- Integrity failures return `Conflict` without source identities. Current authorization still controls receipt and row disclosure.
- Native format 4 stores per-kind descriptors and reference indexes. Ordinary open rejects format 3; migration remains an explicit maintenance operation.
- Archive version 2 carries descriptors and canonical edges. Export validates these against current rows; restore rebuilds native indexes before publication.
- Historical receipts, events and tombstone authorization values do not contribute live edges.
- Do not infer supported multiwriter Runtime operation from native transaction tests.

## Review focus

1. A source kind omitted on restart must not disable restrict.
2. A matching receipt must remain replayable after its old target is deleted.
3. Partial edge writes must roll back with all other writes.
4. An archive with absent or extra edges must fail validation.
5. Schema changes must fail closed, including a reference silently changed into a string.

## Tasks

- [x] Add failing real SQLite/redb tests for dangling references, restrict, unlink and self-reference.
- [x] Add core descriptor serialization, strict recursive extraction and canonical schema validation tests.
- [x] Add native format-4 schema binding and atomic indexes to SQLite and redb. Test create/delete races, replay and rollback.
- [x] Forward registration through transparent test adapters; register explicit descriptors in raw persistence conformance fixtures.
- [x] Extend archive collection, semantic validation and restore to preserve schema and edges. Test tampered and incomplete graphs.
- [ ] Add explicit offline format upgrade and schema migration with interruption tests. Preserve receipts and pending obligations.
- [ ] Exercise the reference application through upgrade, restart and backup/restore.
- [ ] Run focused tests, full local checks and an independent review. Record limits and evidence before merging.

The reference-integrity slice passed these checks on 3 October 2026. See
`docs/research/restrict-reference-results.md`. The final gate remains open for the
pending native migration and application-upgrade tasks. The archive-only upgrade
does not complete those tasks.

## Later stage-two work

Dependency-aware retention and identity expiry remain required. Their policies must distinguish live relations from journal, receipt and pending-work dependencies.
