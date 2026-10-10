# Durable scheduling floor

Status: selected implementation design; verification pending.

## Public seam and frozen data

`Channel::intent_at(payload, not_before_unix_seconds: u64)` returns an ordinary versioned `Intent`.
`intent(payload)` and `Intent::new` use `not_before: None`.
Both `Intent` and `PendingWork` store `Option<u64>` with serde default and omission for `None`.
The timestamp uses trusted runtime Unix seconds. Conversion from provider milliseconds belongs outside this generic seam.

Notification materialization copies the frozen field. PendingWork equality includes it.
The identity hash remains stable for the same root, path, channel, and ordinal.
A duplicate work identity with changed timing returns `IdentityMismatch`; it never replaces the prior floor.

## Ledger invariants

Initialize due to `max(cause.started_at, not_before.unwrap_or(0))`.
Retry and operator scheduling cannot lower due below this immutable floor.
Restore fences active claims and retains the original floor, attempts, root usage, and reconciliation requirements.
Archive validation rejects a stored due below the floor.
A backward clock cannot authorize a delivery-start or delivery-finish transition before the floor.

Pending work that reaches its original age limit before eligibility stops with `Age` without claiming or delivering it.
No delay extends original age, attempt, root, record, byte, or fanout budgets.
The existing worker observes eligible work. No new timer, polling loop, or scheduler is introduced.
The existing worker's idle interval does not establish a real-time execution guarantee.

## Format admission and explicit upgrade

Use native format 9 and archive format 7. Existing native format-8 readers admit only marker 8.
Existing archive-format-6 readers admit only archive 6/storage 8. New markers therefore fence old readers.
The new ordinary reader admits only current markers. Upgrades publish into fresh destinations and never rewrite source files.

Formats 3–7 retain their existing explicit recovery-metadata conversion.
Format 8 already contains operator receipts, delivery profiles, and Work revisions. Its upgrade must preserve them.
Missing scheduling metadata in format 8 means `None`. A scheduling field present in an older format is rejected.
Reject contradictory scheduling data rather than silently discarding it.

Archive versions 1–5 retain their current conversions. Archive 6 must retain its operator metadata and accept no new scheduling fields.
Expose `upgrade_v6_archive` for archive 6/storage 8 into archive 7/storage 9.
Native `upgrade_from` admits populated format 8 in addition to its historical formats.
Migration admission supports format 8 through the same checked decoder.
Restore continues to fence leases without resetting the frozen scheduling floor.

## Acceptance and limits

Core tests establish eligibility, equality, restore, and control transitions with a deterministic clock.
Shared actual SQLite/redb tests establish atomic mutation+intent, replay, restart, current authority, and version fencing.
Populated native-8 and archive-6 upgrades retain row values, receipts, events, effects, tombstones, operator receipts, and Work budgets.
Negative cases retain source bytes and leave fresh destinations absent.
A finite worker-driven test starts the existing worker only after registration, commits, restarts, advances the clock, and observes one delivery.
No paid provider, live deployment, or production data is involved.
