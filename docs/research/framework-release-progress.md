# Framework release progress

Active owner goal: complete the four sequential stages in
[the release checklist](../../openspec/changes/prepare-framework-release/tasks.md).
This is an implementation ledger, not a release readiness declaration.

## Stage 1: reference application and author ergonomics

The first maintained slice extends the existing application with
`./demo/run reference sqlite` and `./demo/run reference redb`. Its public API
journey combines inventory filtering/sorting/moving pages, pending checkout
compensation, replay, live membership and Task-to-Dashboard reactions.

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
The correction in progress adds a typed Resource transition validator through
the shared mutation pipeline, rather than per-kind transport controllers.

Other identified friction remains open: structured action inputs require manual
Field/Input implementations; discovery declarations repeat field/action strings;
Checkout references are strings and ResourceRef currently promises identity only.
Reference integrity belongs to stage 2. These findings are not silently marked
resolved by the new recovery journey.

## Remaining stage boundaries

- Complete stage 1 with invariant enforcement, author ergonomics, a coherent
  reference workflow including attachments and independent review.
- Stage 2 adds enforced restrict references, versioned migrations, retention and
  reference application upgrade/backup/restore evidence.
- Stage 3 integrates actual planner/index execution and maintenance; prototype
  selector results do not prove maintained transactional index behavior.
- Stage 4 adds operator recovery, single-writer ownership, real identity/secrets
  setup, extension conformance, executable skills and final package acceptance.

No stage or whole-goal completion follows merely from the current tests passing.
