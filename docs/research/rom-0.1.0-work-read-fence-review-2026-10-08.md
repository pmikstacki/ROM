# Independent Work read-fence checkpoint

Date: 2026-10-08. Scope: corrected shared read facts, ReadFence, bridge publication, and nine maintained bridge cases.
No product edit or native run occurred during this review. Broader affected-suite and Clippy evidence remain separate.

## Result

The reviewed source fixes the previously identified lost no-op read precondition and foreign Idle context problems.
Both maintained cases failed for their intended behavioral assertions before correction and pass in the current nine-case bridge log.
No source blocker remains from this narrow corrective review.
This is a coherent-context association mechanism, not a native transaction, phantom detector, or general MVCC protocol.

## Exact read facts and filtered writes

The engine records keyed record reads in `record_facts`, including None for confirmed absence.
Repeated keyed reads use those captured facts or the transaction-local overlay.
Candidate reads capture the complete returned record and reject disagreement with an already captured fact.
Finalization moves record facts into separate record_preconditions before filtering unchanged record writes.
It likewise captures every root's original optional value before filtering unchanged root writes.

Consequently, an exact duplicate enqueue keeps its record precondition even when its write set is empty.
A repeated unchanged DeliveryStarted also retains its record and root read facts.
Attempted insertions retain absence preconditions for records and newly created roots.
The bridge checks every retained fact before any record, index, root, or header mutation.
Failed checks return Conflict without publishing candidate changes or rotating the existing context.

These are trusted coherent-reader facts. The native implementation must not accept a serialized caller's assertions as authority.
The actual native application must use the original writer transaction and enforce all captured facts before publication.

## ReadFence behavior

ReadFence owns an Arc token with no serialization interface. Equality uses pointer identity; Debug prints only `ReadFence`.
The token remains allocated while a delta references it, which prevents address reuse during that delta's lifetime.
WorkRead must return the same token for its coherent context.
The engine checks context identity before and after preparation.

Each imported WorkImage receives a fresh token. Foreign images therefore differ even when their headers and byte accounting match.
Bridge application checks token identity before applying a delta.
Every successful application receives a new token, including an Idle or other no-op application.
Earlier prepared deltas cannot be reapplied merely because the canonical state stayed equal.
The maintained Idle rotation case verifies that behavior.

The token does not authenticate a reader's candidate order, missing candidates, or range predicate contents.
It does not prevent a malicious trusted reader from sharing a token with inconsistent data.
Same-context altered-reader tests exercise exact record and root checks, but they do not prove arbitrary predicates.
Adapters must bind one token to one original coherent transaction and preserve its lifecycle during preparation/publication.
Do not reuse the token across reopened transactions or invalidate it independently of the actual transaction boundary.

## Actual checks and remaining coverage

The preserved readset RED reports Ok(Changed) instead of Conflict for a foreign exact-duplicate enqueue.
The preserved context RED reports Ok(Idle) instead of Conflict for a foreign equal-accounting candidate image.
The corrected log reports nine passing bridge tests, zero failures, and zero ignored tests in 0.05 seconds.

The nine cases cover import validation, ordinary publication, stale application, retained Done history, and foreign equal-accounting records.
They also cover no-op read facts, foreign Idle predicates, successful no-op token rotation, and same-context record/root corruption.
The root-corruption test exercises a changed root, not an unchanged root filtered from the write set.
The source preserves that unchanged root fact correctly, but no explicit readonly-root regression appears in this nine-case gate.
Confirmed absent record/root facts likewise have source coverage but no explicit same-context absence-corruption case in this gate.
Add those negative cases when integrating native insert/precondition handling; do not label them already executed.

| Host log under `/var/tmp/` | SHA-256 |
| --- | --- |
| `rom-010-work-bridge-readset-red.log` | `c5a83238afb7cb36bf1d798a283a4cc4bf32c067caacdd4d276b6d8be9b29291` |
| `rom-010-work-bridge-context-red.log` | `a96cc6bc1564791001de50a91cafda3d4c7315ba202a9341788d7f0c07417364` |
| `rom-010-work-bridge-fence-green.log` | `1dea1555df75623551338bb3bf8e5dfa674eb5b8f214e1400cc6dd0efd12a7d2` |

| Inspected source under `crates/rom/src/reaction_work/` | SHA-256 |
| --- | --- |
| `incremental.rs` | `04d8935fa4b6b6625893605f74caa5ed40810c2f57ef2b4cc5fb9dc2fdd21198` |
| `incremental/engine.rs` | `42b7f7fb582fce62f0ddeebed46c5a74501c257b8554ebeae8e8843b904dfb09` |
| `incremental_bridge.rs` | `346463560f501fd0df37efba6ba3e58cb5c1351f5061bcf2056db0cefaea8a3e` |

The audit independently read the raw logs and calculated the listed hashes.
The broader owner run was still active when this checkpoint was written.
No scoped Clippy, full verifier, native adapter integration, query-index speedup, or production completion is claimed here.

## Fresh affected-suite checkpoint

Root subsequently reran the affected suite and scoped all-target Clippy using distinct native log paths.
The independent audit read 57 passing tests, zero failures, and zero ignored tests in the affected log.
The separate Clippy log reaches successful completion.
Their host-exported SHA-256 values are `fb6944f2970264aea3f7cf5d85f77534b74be0832d733bae1cdf43f0259d8c13` and `8c233a54790c4800d3dd585436af679aa838da259bfdb129412e2d51cdb75cdc`.
Paths: `/var/tmp/rom-010-root-work-fence-affected-20261008.log` and `/var/tmp/rom-010-root-work-fence-clippy-20261008.log`.

An earlier owner's complete raw log and inventory were accidentally truncated during export.
The surviving tool-captured excerpt is not a replacement for that lost full log.
The recreated current inventory matches the previously captured digest `1d1aa33771ccd794543713966c15cff1a76372cd617a705340b2d22806fe3bda`.
It is recreated evidence, not the recovered original file.
The fresh root rerun supplies the complete affected-suite evidence used here.
Subsequent production-support changes require their own current-source checks; this checkpoint is not final release verification.
