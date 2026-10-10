# Incremental native format decision review

Date: 2026-10-08. Scope: current backup, archive, Collector, and native adapter source.
No product edits, native builds, source-hash acceptance, or runtime compatibility tests occurred.

## Recommendation

Choose B: native STORAGE_FORMAT 10, canonical archive version 7, and explicit archive-7/storage-9 import compatibility.
This retains the documented meaning of the public STORAGE_FORMAT constant and current adapter/manifest coupling.
It requires a small explicit archive compatibility branch, plus the native conversion work required by either option.
Do not increment the archive version merely because the physical native tables change.
Do not silently reject accepted archive-7/storage-9 backups.

| Option | Benefit | Required correction and cost |
| --- | --- | --- |
| A: native adapter constant 10, archive storage marker 9 | Current archive reader and manifest expectations remain unchanged. | Public STORAGE_FORMAT currently means native format; keeping it 9 makes that contract false. A truthful split requires new names, documentation, adapter dependencies, and manifest semantics. |
| B: shared STORAGE_FORMAT 10, archive version 7, explicit storage-9 compatibility | Native format and new manifests retain their current documented relation. | Archive reader must explicitly accept the two supported version-7 storage markers. Expected current-marker tests and both adapter conversion paths need updates. |

A is technically possible with an explicitly revised format model.
It is not the lower correct scope under the currently documented public constant contract.
B preserves that contract and changes one compatibility dimension deliberately.
Neither option proves old archives restored correctly until actual maintained and predecessor tests run.

## Authoritative current behavior

`rom-backup/src/model.rs` declares archive version 7 and documents STORAGE_FORMAT 9 as the current native format.
Snapshot::manifest uses those constants directly. Manifests carry backend, exact record counts, and external-data exclusions.
`archive.rs::read` currently calls read_version with exactly the current archive/storage pair.
It validates the private envelope, hash, bounded body, backend, counts, complete Snapshot, and exact reconstructed manifest.
It currently rejects another storage marker even if the canonical body shape is identical.

The older archive upgrade functions handle explicit pairs through archive-6/storage-8.
They retain legacy decoding and scheduling-field rejection for their historical formats.
Archive version 7 currently uses the direct canonical decoder and admits scheduling fields.
Archive-7/storage-9 must continue through that same canonical decoder after native 10 is introduced.
It must not be routed through a pre-scheduling legacy upgrade branch.

redb FORMAT derives from the shared constant. SQLite open checks and initializes user_version from that same constant.
Both native upgrade paths currently describe format 3 through 8 predecessors.
Current redb maintenance expects format-specific table counts and a single complete state record.
Both choices therefore require explicit native-9 support and new native-10 layout validation.

## Minimal B changes

1. Set the shared native constant to 10 after both adapter layout contracts are ready.
2. Retain archive version 7 and canonical Snapshot serialization.
3. Add an exact supported pair set to the ordinary archive reader: `(7, 9)` and `(7, 10)`.
4. Keep unsupported versions, marker combinations, unknown fields, wrong backend, and corrupted envelopes fail-closed.
5. Validate the decoded Snapshot identically for both pairs. Compare expected counts against the actual supplied pair.
6. Return the actual imported Manifest without rewriting its storage marker in memory as if the original archive were new.
7. Write new backups with `(7, 10)`. If converting an old backup, publish a fresh archive and preserve its source bytes.
8. Add native-9 predecessor decoding and explicit fresh-destination conversion to both adapters.
9. Update version-specific native table/schema/index validation, current-format tests, and migration notes.

Dispatch from one bounded, verified envelope where practical.
Do not catch an arbitrary parse or validation error and retry it under an older interpretation.
Compatibility is an explicit supported pair, not permissive fallback on malformed input.
Older archive upgrade outputs may become `(7, 10)` while their original source files remain unchanged.
Retain historical tests that prove the accepted source semantics; update only expectations for newly written destinations.

## Collector and physical accounting

Collector::new currently decodes complete canonical StorageState and initializes work/operator record counts from it.
It charges the canonical state's complete encoded byte length before collecting rows, receipts, events, effects, descriptors, and references.
Its scheduling flag uses `format >= 9`, so the current native-10 canonical body does not need a scheduling downgrade.
Collector::physical separately charges adapter-owned records and encoded key/value byte totals without adding canonical snapshot data.
SQLite query-index validation already uses that physical charge path.

Split-layout reconstruction must retain both dimensions: complete canonical state bounds and additional physical metadata/index bounds.
Do not initialize Collector from a slim header with no WorkLedger and thereby omit retained work records or bytes.
Do not decode or accumulate all physical work rows unbounded before reconstructing the canonical state.
Introduce bounded collection/reconstruction support if needed before complete canonical encoding.
Charge derived root summaries, accounting records, member/active indexes, and any other adapter metadata under explicit budgets.
Document deliberate duplicate physical/canonical charges rather than silently dropping one to fit an old limit.

The archive file byte limit and native inspection budget have different evidence surfaces.
Changing their implementation must not quietly weaken either limit or add synthetic logical Work records for physical summaries.

## Compatibility controls

- Restore a genuine accepted archive-7/storage-9 backup through the current public restore path after the change.
- Preserve its exact file hash and semantic state, receipts, custom codec values, events, effects, and current authorization behavior.
- Write and restore archive-7/storage-10 under both backends with unchanged canonical validation.
- Reject unsupported pairs, wrong backend, unknown manifest fields, wrong counts, corruption, and exceeded bounds.
- Convert offline native-9 sources to fresh native-10 destinations with original source hash/inode evidence preserved.
- Prove actual old native-9 binaries refuse native-10 before writable open; test private recovery-copy behavior for dirty redb sources.
- Retain older supported native and archive conversion tests without rebinding receipt origins or scheduling semantics.
- Reconstruct and validate all new summaries/indexes, including corruption and undercharged-metadata cases.

Old readers rejecting new `(7, 10)` archives is an explicit forward-compatibility limit, not a historical-backup regression.
The current new reader must continue accepting the owner-required `(7, 9)` backups.
This recommendation does not admit a release or replace actual compatibility evidence.
