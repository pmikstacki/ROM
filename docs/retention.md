# Retain durable history

ROM can remove expired receipts, journal entries and completed work. The host
selects the policy. ROM preserves dependencies and rejects unsafe removal.
Maintenance writes a fresh database. It does not modify the source.

## Select retry boundaries

Each mutation has an explicit retry epoch. The default is zero. Keep the epoch
with the request and its idempotency key for every retry.

```rust
let command = rom::Command::create("one", value)
    .idempotency("create-one")
    .retry_epoch(1);
```

An epoch is a request namespace. It is not a Resource schema version or a journal
cursor. `Runtime::retry_epochs(&actor)` reads the current boundaries after the
normal actor check. It does not assign an epoch to a command.

`RetryEpochs` has three fields:

| Field | Meaning |
| --- | --- |
| `current` | Highest permitted request epoch |
| `admission_floor` | Lowest epoch for fresh external mutations |
| `replay_floor` | Lowest epoch for receipt replay and existing causal work |

Require `replay_floor <= admission_floor <= current`. Boundaries can only increase.
ROM does not choose a TTL. It does not use a journal position as a receipt's age.
No-op requests can have receipts without journal entries.

For example, `{ current: 1, admission_floor: 1, replay_floor: 0 }` closes epoch
zero to new external mutations. Existing receipts can replay. Existing workers
can complete through a matching persisted claim. When that work is complete,
the host can advance `replay_floor` to one.

After expiry, a request in epoch zero returns `IdentityExpired`. This also applies
when its receipt remains as proof of a current row. HTTP reports status 410 with
code `identity_expired`. Resource mutation commands accept `--retry-epoch N`.
An omitted option means zero. Operator controls keep `retry_epoch` in their exact request file.
Never retry an expired request with a new epoch to bypass this result.

For configuration reloads, use `ReloadTicket::request_with_epoch` and
`ReloadTicket::resume_with_epoch` with the original epoch. Keep that epoch in the
caller's durable recovery data. `SourceActivation` does not store it. The existing
`request` and `resume` methods always use zero. A ticket keeps its selected epoch
for every source mutation; it does not renew the request after expiry.

## Run offline retention

1. Stop writers and workers for the source store.
2. Select retry boundaries and explicit maintenance limits.
3. Select a journal prefix to remove, if required.
4. Call the native adapter with a new destination path.
5. Inspect the returned counts and test the destination before activation.
6. Keep the source until the deployment passes its acceptance checks.

```rust
let policy = rom_backup::RetentionPolicy::new(rom::RetryEpochs {
    current: 1,
    admission_floor: 1,
    replay_floor: 1,
}).journal_through(last_confirmed_position);

let (store, report) = rom_sqlite::Sqlite::retain_from(
    source,
    destination,
    &policy,
    rom_backup::BackupLimits::default(),
)?;
```

`Redb::retain_from` uses the same contract. Without `journal_through`, ROM keeps
the existing journal floor. Journal removal is a contiguous prefix. Old cursors
return `HistoryGap`; they do not silently skip missing history. Native publication
also rotates the journal generation and fences old work claims.

The report contains counts and epochs. It contains no Resource values or effect
payloads. The default budget is 128 MiB of logical data and 400,000 records.
Decoded data, indexes and copies require additional memory. The operation is
offline and bounded; it is not an unbounded streaming compactor.

## Understand retained dependencies

- Keep every unexpired receipt. Expired history can still be needed as proof.
- Keep receipts referenced by retained journal entries or effects.
- Keep one full receipt for each current row, including a tombstone.
- Remove only complete causal roots. Preserve attempt budgets with their records.
- Reject expiry across Pending, Leased, Stopped or uncertain delivery work.

Raw effects do not prove provider acceptance. By default, ROM keeps them. The host
can call `.settle_effects(receipt_identity)` to assert that all effects for an
expired receipt are settled. An absent identity or an unexpired receipt fails.
This assertion does not bypass unresolved causal work.

Use `.purge_tombstone(key)` only for deliberate physical removal of a deleted row.
ROM rejects the request if unexpired receipts, retained events, effects or
references depend on it. Any retained work also prevents this operation because
opaque callback payloads can depend on other rows. Live rows cannot be purged.
Purge does not erase copies from the source database, older backups or external systems.

## Prevent policy rollback after restore

Store the latest trusted epoch boundaries outside the backups that can roll back.
Pass them to `Runtime::builder().retry_fence(trusted_boundaries)` before `build`.
A restored store with older boundaries fails activation. Persist this fence before
activating a newer policy. Keep it in trusted deployment configuration or another
independent durable authority.

A backup preserves the policy known when it was created. It cannot record future
retention decisions. A zero default fence cannot detect rollback to older policy.

Accepted 0.0.3 native format 8 and archive format 6 preserve epoch metadata and operator receipts.
The 0.1.0 candidate uses native format 9 and archive format 7 to retain frozen scheduling floors.
Use the explicit
[format upgrade](native-upgrade.md) for older data. Older readers must reject these
formats. Interruption before publication leaves the source usable and destination
absent. An `Unknown` publication result requires destination inspection before retry.
