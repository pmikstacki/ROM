# Framework release progress

Active owner goal: complete the four sequential stages in
[the release checklist](../../openspec/changes/prepare-framework-release/tasks.md).
This is an implementation ledger, not a release readiness declaration.

## Stage 1: reference application and author ergonomics

The first maintained slice extends the existing application with
`./demo/run reference sqlite` and `./demo/run reference redb`. Its public API
journey combines inventory filtering/sorting/moving pages, pending checkout
compensation, replay, live membership, Task-to-Dashboard reactions and actual
folder attachments read after reopening without reuploading.

The initial executable test failed on the absent `reference` command. After
implementation, two parent integration tests passed: the executable journey and
actual child-process exit/recovery, each on SQLite and redb. The ignored child
fixture is explicitly invoked twice; it exits with code 86 immediately after the
confirmed rejection commit, without shutdown, destructors or work processing.
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
Checkout references remain strings and ResourceRef currently promises identity
only; enforced reference integrity belongs to stage 2. Nested structured Inputs,
generic payload declarations and input discovery schemas are not implemented.

## Remaining stage boundaries

- Stage 2 adds enforced restrict references, versioned migrations, retention and
  reference application upgrade/backup/restore evidence. The
  [integration preparation](restrict-reference-integration-plan.md) explains why
  checks must share the adapter transaction and how edges affect backup/migration.
- Stage 3 integrates actual planner/index execution and maintenance; prototype
  selector results do not prove maintained transactional index behavior.
- Stage 4 adds operator recovery, single-writer ownership, real identity/secrets
  setup, extension conformance, executable skills and final package acceptance.

No stage or whole-goal completion follows merely from the current tests passing.
