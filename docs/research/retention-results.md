# Retention and retry epoch results

Date: 2026-10-03. Scope: release stage 2.3. This report records maintained code and
tests, not a claim that the whole framework release is ready.

## Implemented contract

Commands, receipts and causal work carry an explicit retry epoch. Epoch zero
preserves the original durable identity and Invocation JSON shape. The host sets
monotonic admission and replay floors. ROM never renews an expired request by
assigning the current epoch.

The shared maintenance engine removes a journal prefix and whole completed roots.
It retains current-row proofs and receipts required by events or effects. Explicit
effect settlement and tombstone purge have separate dependency checks. Both native
adapters use this engine and their existing bounded readers and fresh publication.

Native format 6 and archive 4/storage 6 preserve the policy. Explicit upgrades
support native 3/4/5 and archive 1/2/3. Legacy-marked data with nonzero epoch metadata
is rejected. Current-format Resource migration preserves nonzero epochs.

See the [usage contract](../retention.md) and
[design](../superpowers/specs/2026-10-03-retention-design.md).

## Executed checks

The coordinator ran `./scripts/check` in the native `rom-dev` container with Rust
1.99.0. It passed strict OpenSpec validation, formatting, warning-free Clippy,
workspace tests, doctests, rustdoc, compile fixtures, consumer checks and the auth
and identity verifiers. Source: base `48fcc97` plus this retention change. The
lockfile adds only the existing `rom-backup` package as a config test dependency.

Full log: `rom-dev:/var/tmp/rom-retention-full-check.log`.
Target: `/var/tmp/rom-release-relations-tests-target`.
Build profiles set `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`.

| Check | Evidence |
| --- | --- |
| Core work/state retention | 12 new regressions cover whole roots, budgets, exact claims, atomic rejection and notification acceptance. |
| Runtime epoch handling | 6 tests cover legacy identity, sealed replay, expiry, authority, codec/proposal races and startup fences. |
| Shared snapshot selection | 6 tests cover dependency pins, no-op proofs, limits, explicit effects, tombstone purge and deterministic selection. |
| Native retention | 7 parent tests exercise both adapters, including bounded-capacity reuse and tombstone recreation. |
| Actual interruption | A parent invokes the ignored child fixture on both adapters. The child exits with code 86 before publication. The source remains valid and a retry succeeds. |
| Reactive execution | Two-backend tests drain an epoch-1 Source through Action and Notification while fresh epoch-1 commands are sealed. |
| Restore fence | Both adapters restore an older backup; Runtime activation rejects the newer trusted fence. |
| CLI and HTTP | An explicit epoch remains unchanged. Expiry maps to HTTP 410 and does not cause an automatic retry under another epoch. |
| Config reload | Explicit request/resume epoch APIs preserve the caller's original epoch. Default wrappers retain zero. |
| Current-format migration | Nonzero boundaries, receipt/work epochs and exact identities survive a typed field migration and backup/restore on both adapters. |

Focused logs include `/var/tmp/rom-retention-shared-red.log`,
`/var/tmp/rom-retention-shared-final.log`, `/var/tmp/rom-retention-native-expanded.log`
and `/var/tmp/rom-retention-archive-final.log` in the container.
The current-format migration regression was added after the full verifier. Only
its test file changed. The coordinator reran the complete native retention test
target and workspace format check; both passed. Scoped Clippy also passed.
Final test log: `/var/tmp/rom-retention-final-acceptance.log`.

## Review corrections

Independent reviews checked core, adapter and maintenance boundaries. They found
three issues that now have regression coverage:

1. A Done notification without confirmed Accepted delivery could be retired.
   The predicate now requires Accepted. Unknown, timeout and other outcomes do not qualify.
2. A bundle or work materialization could persist inconsistent epochs and fail
   only during later archive validation. Shared atomic checks now reject it before persistence.
3. A legacy format marker could carry nonzero new epoch fields. Explicit legacy
   validation now rejects that input instead of treating it as valid old data.

Reviews found no remaining blocker in this slice. Constructors and snapshot
budget checks share helpers. New behavior uses named modules; facade roots only
declare or export them. Historical experiment artifacts were not rewritten.

## Limits and remaining work

The policy is explicit and offline. This does not establish a production TTL,
online compaction, concurrent-writer cutover or an unlimited history capacity.
Pending and Stopped obligations can prevent expiry. The host must resolve them.
Settling a raw effect is a host assertion, not proof of external acceptance.

An independent durable retry fence is required to detect rollback to an old
backup. The backup alone cannot know later policy. Logical purge does not erase
source files, other backups or external deliveries. The byte limit bounds
serialized logical data, not exact heap usage or physical database file size.

Stage 2.4 still requires the reference application's combined upgrade, backup and
restore acceptance journey. Planner/index integration and release operations also
remain on the full release checklist.
