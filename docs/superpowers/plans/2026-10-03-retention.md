# Retention implementation plan

> **For agentic workers:** Use superpowers:executing-plans for the assigned task.

**Goal:** Reclaim historical data without reopening expired mutation identities.
**Architecture:** Core owns epoch admission and work/journal invariants. A shared
snapshot engine selects removable data. Native adapters collect and publish it.
**Tech Stack:** Rust 1.99.0, existing SQLite/redb adapters and locked dependencies.
**Spec:** `docs/superpowers/specs/2026-10-03-retention-design.md`.

## Global constraints

Preserve source stores, public module paths and historical evidence. Use explicit
host policy. Never relabel an old request with a new epoch. Keep facade roots and
share validation. Publication targets must be absent. No background TTL policy.

## Tasks

- [x] Core epoch API: `crates/rom/src/retry_epoch.rs`, runtime command/receipt/work
  plumbing and CLI option. Test omitted zero, sealed replay, current auth and a
  trusted restore fence in `crates/rom/tests/retry_epochs.rs`.
- [x] Persisted state: `storage_state/retention.rs` and `reaction_work/retention.rs`.
  Test exact claims, monotonic boundaries, whole-root pruning and atomic rejection.
- [x] Shared maintenance: `crates/rom-backup/src/retention{,_policy}.rs`, public
  exports and `tests/retention.rs`. Test actual removal, dependency pins, explicit
  effect settlement, tombstone purge and byte/record limits before implementation.
- [x] Native adapters: `retention.rs` in both stores, epoch access/commit checks,
  explicit format upgrades and shared persistence tests. Verify sources unchanged,
  restart, backup/restore and process exit before publication.
- [x] Independent review, full `./scripts/check`, usage guide and results report.
  Update the release checklist only after evidence covers the full requirement.

## Review focus

- An expired current-row proof must not grant replay. Core and native tests cover it.
- A stale claim must not admit a fresh old-epoch action. Persisted-state tests cover it.
- Done parents with unfinished children must retain budgets. Ledger tests cover it.
- No-op receipts lack journal timestamps. Snapshot tests cover deterministic pins.
- Restoring old policy must not silently reopen admission. Runtime tests use an
  independent newer fence. Docs describe the host's responsibility.
