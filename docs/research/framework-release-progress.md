# Framework release progress

Active owner goal: complete the four sequential stages in
[the release checklist](../../openspec/changes/prepare-framework-release/tasks.md).
This is an implementation ledger, not a release readiness declaration.

## Stage 1: reference application and author ergonomics

The first maintained slice extends the existing application with `./demo/run reference sqlite` and `./demo/run reference redb`. Its public API journey combines inventory filtering/sorting/moving pages, pending checkout compensation, replay, live membership and Task-to-Dashboard reactions. The journey also reads actual folder attachments after a reopen, without another upload.

The initial executable test failed on the absent `reference` command. After
implementation, two parent integration tests passed: the executable journey and
actual child-process exit/recovery, each on SQLite and redb. The ignored child fixture is explicitly invoked twice. It exits with code 86 immediately after the confirmed rejection commit, without shutdown, destructors or work processing.
Unknown outcomes preserve both reservations; recovery removes only the rejected
token and replay adds no stock/checkout event or queued work.

`./demo/verify` passed in native rom-dev, including formatting, warning-free
Clippy, all demo tests, both actual TCP smokes, server restart/SIGINT checks and
rustdoc. Log: `/var/tmp/rom-reference-verify.log` inside the container.
The release OpenSpec change passes strict validation.

After integrating the reference journey and transition validator, the coordinator
ran the full `./scripts/check` on main at `4acbe7d`; exit 0. This includes the
workspace tests, warning-free Clippy/docs, compile fixtures, dependency isolation,
consumer and authentication/identity checks. Log:
`/var/tmp/rom-release-stage1-check.log`. The attachment follow-up also passed
`./demo/verify` (`/var/tmp/rom-reference-final-demo.log`) and an independent rerun
of both reference tests.

An independent reviewer reran the focused test and both direct commands, and
checked invalid backend/extra-argument rejection. No P1/P2 findings remained in
this slice. Review confirmed that public Runtime/Command/query/live APIs perform
the work; driver references only construct storage. Authority coverage here is
denial of an unprovisioned actor after restart, not authority revocation across
restart. Live coverage proves refreshed membership, not exactly-once emissions.
Process-exit evidence does not certify power loss, schema upgrade or migration.

## Authoring audit and next corrections

Code inspection found that terminal Checkout rules and Stock reservation capacity
were enforced only in actions, while generic patch/replace remained writable.
A separate regression reproduced a direct patch changing a confirmed rejection.
The maintained correction adds a typed Resource transition validator through
the shared mutation pipeline, rather than per-kind transport controllers.
See [transition validation evidence](resource-transition-validation-results.md).
Independent review reran four core tests, three demo transition tests and the
existing compensation test without blocking findings. Immutable compensation
context and protected deletion are also covered. The validator is trusted bounded
native code and runs under the commit gate; it is not a cross-Resource constraint.

Structured action inputs now use `#[derive(Input)]`: ReserveInput declares token
and quantity directly. A single named-object generator supplies Resource and Input
codecs, preserving the established Resource raw-identifier wire spelling. Static
`Input::field_names()` permits sanitized action.field diagnostics; arbitrary custom
errors remain masked. The independent review reproduced and verified fixes for
both raw-identifier compatibility and lost field diagnostics. See
[action input contract](../action-inputs.md) and its OpenSpec change.

The adaptation's wire test first failed with an action-only error, then passed
with `reserve.quantity`, no state change on invalid input and normal durable
replay for valid input. Independent final integration ran seven parent tests
(compensation, reference recovery, named reservation and transition invariants),
including actual TCP and both storage adapters, with no blocking findings.
Final `./demo/verify` and `./scripts/check` both passed on the combined tree:
`/var/tmp/rom-release-stage1-final-demo.log` and
`/var/tmp/rom-release-stage1-final-check.log` in rom-dev.

Stage 1's application/API/review acceptance is complete; the whole release goal
is not. This is automated author-contract evidence, not an external human study.
Explicit discovery allowlists remain a deliberate metadata policy; authors can
already grant all metadata via the existing discovery callback when appropriate.
Stage 1 left Checkout references as strings and ResourceRef as typed identity.
The stage-two update below adds enforcement. Nested structured Inputs,
generic payload declarations and input discovery schemas are not implemented.

## Remaining stage boundaries

Stage 2.1 is complete: persisted descriptors, native atomic restrict checks and
indexed edges now pass the shared SQLite/redb suite. Backup/startup validates the
complete live graph. Review reproduced and corrected archive canonicalization and
work-record budget defects. The full local verifier passed. See
[reference integrity results](restrict-reference-results.md).

The owner also required facade-only module roots. All 14 maintained `lib.rs` and
`mod.rs` files follow this rule, except Rust-required procedural macro entry wrappers.
Public APIs remain available through exports. Tests cover the refactored packages.

Stage 2.2 is complete for versioned offline Resource representation migrations.
Typed steps convert current values, history and work sources while preserving
identities and obligations. Receipts retain exact request-codec versions; current
authorization precedes legacy replay. Both native adapters pass migration, recovery
and interruption tests. Unfinished work requires explicit consumer compatibility
validation. The full local verifier passed. See
[Resource migration results](resource-migration-results.md).

Native format 5 and archive version 3 guard receipt metadata from older tools.
Explicit native upgrades support formats 3 and 4; archive upgrades support versions
1 and 2. The earlier [native upgrade results](native-format-upgrade-results.md)
record the initial format-only milestone. Neither milestone completes the whole release.

Stage 2.3 is complete for explicit offline dependency-aware retention. Retry epochs
prevent reclaimed receipts from reopening old mutations. Whole-root work checks,
current-row proofs, journal/effect pins and trusted restore fences pass tests on
both adapters. Native format 6 and archive 4 now protect that metadata. The full
local verifier passed. See [retention results](retention-results.md).

Stage 2.4 is complete. The reference app upgrades Checkout from a text stock ID
to a typed reference, rebuilds restrict edges, backs up/restores and completes
pending compensation. Both adapters pass clean shutdown and actual process-exit
acceptance. Old receipts require the retained codec and current authority.
The tests also found and corrected SQLite sidecar creation during offline reads.
The complete demo and workspace verifiers passed. See
[reference upgrade results](reference-upgrade-results.md).

- Stage 2's relations, migrations, retention and application acceptance are complete.
  The [integration preparation](restrict-reference-integration-plan.md) records
  the transaction and maintenance boundaries.
- Stage 3 integrates actual planner/index execution and maintenance; prototype
  selector results do not prove maintained transactional index behavior.
- Stage 4 adds operator recovery, single-writer ownership, real identity/secrets
  setup, extension conformance, executable skills and final package acceptance.

No stage or whole-goal completion follows merely from the current tests passing.

## Stage 3 preparation

The core Resource and Runtime implementations now have cohesive internal modules
with stable public exports. Three additional query regressions preserve sort-field
denial, ID-page callback stopping and panic-before-filter behavior on both stores.
The full local verifier passed after the refactor. See
[foundation results](query-integration-foundation-results.md).

The [draft query design](../superpowers/specs/2026-10-03-maintained-query-strategies-design.md)
uses an adapter-owned coherent operation without application callbacks under
native connection locks. The physical-index and semantic-gate reviews identify
the remaining implementation contracts. This is preparation; maintained index
selection, index recovery and new performance measurements are not complete.
