# Persistence latency: independent strategy review

Date: 2026-10-08.

## Recommendation

Optimize the retained work metadata path, not the durability guarantee or acceptance deadline.
The smallest sustainable target is native per-work and per-root records with a shared incremental transition model.
Keep Resource rows, receipts, events, and changed work records in the same native transaction.
A decoded-state cache can be a bounded intermediate experiment. It is not a complete correction for whole-state write amplification.

This review changes no implementation and runs no native workload.
No proposed optimization has a measured speedup in this report.

## Independently checked failures

The optimized SQLite seed reached a completed drain of 1000 rows at 113846 ms.
It created 1100 rows at 118207 ms, then failed the finite drain deadline.
The optimized redb seed drained 1400 rows at 116847 ms.
It created 1450 rows at 118615 ms, then failed the finite create deadline.

Both result records retain the 10000-row target, batch size 100, and 120000 ms seed deadline.
Both processes exited one and drained. Both source fences are true.
These measurements establish seed failure. They do not provide successful steady-state latency or final 10000-row throughput.

| Capsule under `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/` | Evidence | SHA-256 |
| --- | --- | --- |
| `application-load-seed-9e68c5ce25a3fe3004731a37` | `result.json` | `d22b9d24e4287f1fe138574db090742061b3d42b0e40c2432e3fe48ce65f8be6` |
| Same SQLite capsule | `process-output.json` | `c6903286c74626a9900e753a639a41df14786a52329803fe01dfd53fe2767511` |
| `application-load-seed-29638f0187f7166e5f8e8205` | `result.json` | `656f8d320f4cc647fa74a8e0ac0cbd4d028f699f11cb0bb948a297c32096a219` |
| Same redb capsule | `process-output.json` | `68c7f65818d628153b75cef2ad8454eb348814f74b386f74ba4ad4a24b405368` |

Older `71c08634a53d9aa53a580c39` and `042888583135886b53054802` capsules stopped at 500 drained rows.
They are earlier evidence, not the optimized runs above.

## Source-supported amplification

SQLite `persistence::state` decodes the complete `rom_state` JSON value.
`save_state` serializes and replaces that complete value.
`reaction_update` performs both operations for each work transition inside an immediate transaction.
Resource `commit` also decodes and replaces complete metadata alongside its row, receipt, event, and effects.

redb `state::read` decodes the complete `state` value. `state::write` serializes and replaces it.
`Storage::reaction_update` performs both operations with Immediate durability.
`commit_bundle` repeats that metadata path inside the Resource bundle transaction.

The shared model adds further retained-history costs:

- `StorageState::update_work` clones the full state.
- `WorkLedger::apply` clones the full ledger.
- `prepare_candidate` compares retained records to preserve version semantics.
- Retry-epoch validation scans retained work.
- `reaction_records` clones all records.
- `StorageState::bundle` clones full metadata and computes retained journal sizes.

Therefore, a one-record change can decode, copy, scan, encode, and rewrite unrelated retained work.
This mechanism is confirmed by source inspection.
Its share of measured elapsed time remains a hypothesis until stage timing or profiling separates these costs from sync and scheduling.
The increasing batch cost is consistent with amplification, but is not a causal profile by itself.

## Primary-source constraints

SQLite permits one write transaction at a time. BEGIN IMMEDIATE acquires the write transaction before its first mutation.
Several native table changes can share that transaction. [SQLite transactions](https://www.sqlite.org/lang_transaction.html).

SQLite WAL still has one writer. It writes changed pages and introduces checkpoint work.
Changing synchronization can weaken power-loss durability; this is not an acceptable latency correction here.
Large WAL growth and long readers can also affect latency. [SQLite WAL](https://www.sqlite.org/wal.html).

redb's transaction API permits multiple table operations before one commit and documents one active write transaction.
Its documented default durability is Immediate. The adapter explicitly selects that level.
[redb WriteTransaction](https://docs.rs/redb/latest/redb/struct.WriteTransaction.html).
The fetched reference is current documentation, not a pinned-version implementation proof.
The workspace pins redb 4.3.0; source inspection confirms the adapter's actual transaction calls.

These APIs support atomic partitioned metadata updates. They do not require one large serialized state value.

## Alternatives

| Strategy | Smallest useful change | Limit or risk | Decision |
| --- | --- | --- | --- |
| Bounded decoded-state cache | Reuse one adapter-owned committed snapshot under its native writer gate | Whole-state clones, validation scans, encoding, and writes remain. Unknown commits require invalidation. | Optional measured intermediate step; not the sustainable endpoint. |
| Native incremental metadata | Store work by ID, root budgets by root, and small global metadata separately | Requires shared delta validation, indexes, migration, and archive reconstruction tests | Recommended correction, beginning with the hot work path. |
| Bounded transaction batch | Combine compatible native updates before a commit | Changes acknowledgement, cancellation, crash, and external-attempt boundaries unless carefully integrated | Defer until incremental writes are measured. |
| Relax synchronization or discard retained work | Reduce native persistence cost | Weakens the requested durability or retained-history contract | Reject. |

## Minimal safe implementation sequence

First measure decode, candidate validation/copy, serialization, native write, and commit intervals on the existing frozen seed workload.
Keep diagnostic output bounded and outside adapter guards.
Do not convert an incomplete seed into a passing load result.

Next isolate a shared work transition that returns validated changed records and changed root counters.
Keep claim generation, retry epochs, work revision increments, root limits, and operator receipts in that shared model.
Avoid copying those rules into two adapters.
A cache or index may accelerate selection. It must not become authority for a claim or commit outcome.

Persist the affected work records and root counters in native tables within the existing transaction.
Keep Resource commit and any completed claim update atomic.
Keep DeliveryStarted durable before an external callback, and DeliveryFinished durable after its observed outcome.
Maintain an eligibility index for claims rather than repeatedly exporting all retained records.
Index ordering must retain the existing scheduling and generation rules.

On failure before commit, discard the candidate and retain the original logical state.
On unknown commit, retain Unknown and invalidate any decoded cache.
Reopen from durable receipts and records; do not trust a cached success or failure.

Add an explicit native format conversion if the layout changes.
Preserve canonical archive output by reconstructing the shared StorageState at backup and restore boundaries.
Do not mutate accepted predecessor archives or old proof databases.
Use copied predecessors to prove forward conversion and exact receipt replay.
An old binary's ability to open a new layout is a separate rollback contract; do not assume it.

## Required tests and measurements

1. Compare every logical state transition against the existing shared model, including limits, epoch mismatches, root budgets, and generation fences.
2. Inject failure after each affected native row write, before commit, and after committed acknowledgement loss.
3. Reopen both adapters and verify exact Resource, receipt, event, effect, work, and operator state.
4. Prove DeliveryStarted precedes external attempt and unresolved delivery remains recoverable after restart.
5. Preserve restore generation changes, cursor fencing, retention accounting, references, and current authorization.
6. Repeat authentic R11 predecessor replay on copies. Preserve all historical inputs and their byte hashes.
7. Repeat the unchanged 10000-row seed and 120-second ceiling on both adapters.
8. Measure total bytes encoded, records touched, file/WAL growth, checkpoint duration, RSS, and per-stage latency at increasing retained counts.
9. Repeat current conformance, public consumer, backup/restore, full verifier, and extracted final-package gates after correction.

Success requires retained-history growth without rewriting all prior work for each local transition.
Finite storage limits still apply. No speedup or release acceptance is claimed before these measurements pass.
