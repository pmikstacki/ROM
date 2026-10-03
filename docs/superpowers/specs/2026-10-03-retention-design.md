# Dependency-aware retention

## Contract

Retain the Resource mutation contract when maintenance removes historical data.
An expired request must fail with `IdentityExpired`. It must not become a new
mutation because its receipt is absent. A cache cannot provide this guarantee.

Use explicit `RetryEpochs { current, admission_floor, replay_floor }`.
All values start at zero. Require `replay_floor <= admission_floor <= current`.
The host selects monotonic values. ROM does not select a production TTL.
Commands, invocations, receipts and causal work carry an immutable retry epoch.
An omitted epoch means zero, including after the host advances its policy.
Never assign the current epoch automatically to a retry.

Reject epochs below the replay floor, even if a retained receipt proves the current
row. Reject epochs above current. Between the floors, permit existing receipt
replay and completion of an exact persisted work claim. Reject fresh external
commands. Native commit checks enforce these rules in the same transaction as
identity arbitration. Authorization and current disclosure rules still apply.

## Maintenance boundary

Use bounded offline snapshot maintenance and publish a fresh destination. Preserve
the source. Reuse native migration/restore readers and atomic publication. Store
the new metadata in native format 6 and archive 4/storage 6. Older readers must
reject the format rather than discard retry boundaries.

`RetentionPolicy` specifies epochs, an optional journal prefix cutoff, explicitly
settled effect receipt identities and optional tombstone keys to purge. Omission
keeps the existing journal floor and all effects and tombstones. A settlement is
a host assertion about external effects; ROM cannot infer provider acceptance.

Remove only whole all-Done causal roots below the replay floor. Keep root budgets
with their records. Any unfinished or uncertain root in an expiring epoch rejects
the operation. Do not treat Stopped as completed. Keep newer completed roots.

Remove a contiguous journal prefix. Preserve its floor/head and HistoryGap rules.
Keep receipts needed by retained events, effects and every current row. Keep all
unexpired receipts. From expired duplicates, retain one deterministic current-row
proof rather than every no-op receipt. Never remove a current live row.

Explicit tombstone purge requires an expired receipt history, no retained event,
effect or reference dependency and no retained work. The last restriction is
deliberately conservative: an opaque callback payload can reference other rows.
Reject an ineligible explicit purge instead of silently skipping it.

## Restore and policy trust

Backups preserve epochs. A backup cannot contain policy changes made after its
creation. The host must retain the latest trusted retry fence outside rollback
backups. `Builder::retry_fence` rejects activation of older persisted boundaries.
Do not claim that restoring a self-contained old backup prevents policy rollback.

## Proof

Test sealed replay, rejected fresh input, expired retained anchors, native commit
bypass, whole-root budgets, unresolved delivery, explicit effects, journal gaps,
tombstone purge, capacity reclamation, restart, backup/restore and trusted fences.
Run actual process interruption before destination publication on both adapters.
Keep implementation in cohesive modules; `lib.rs` and `mod.rs` remain facades.
