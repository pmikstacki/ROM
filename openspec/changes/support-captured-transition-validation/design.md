# Captured transition validation design

## Agreed direction

The owner authorized the narrow callable change and its acceptance evidence.
Keep Resource as the domain entity. Definitions retain runtime-owned callback dependencies.

`Definition::validate_transition<F>(self, validate: F) -> Self` accepts `F: Fn(&Actor, Option<&R>, Option<&R>) -> Result<()> + Send + Sync + 'static`.
The private storage is `Arc<dyn Fn(&Actor, Option<&R>, Option<&R>) -> Result<()> + Send + Sync>`.
Invocation borrows the stored callback. Existing public paths and all other callback contracts remain unchanged.

## Alternatives

Keep function pointers and a global catalog: rejected because independent runtimes would share application state.
Add a second captured-validator setter: rejected because it duplicates one semantic contract.
Accept an owned generic callback through the existing setter: selected because existing ordinary callers continue to compile.

## Ownership and execution

Each registered Definition owns the callback through Arc. Independent runtimes can capture different catalogs.
The callback reads one immutable dependency snapshot for each evaluation.
Later catalog publication can affect later new transitions. It cannot reinterpret matching receipt replay.
Captured dependencies remain trusted application code. Rust bounds do not prove bounded execution or freedom from deadlock.

The callback remains synchronous, deterministic for its selected snapshot, bounded, and non-reentrant.
It must not perform I/O or create external effects. Do not hold a catalog write lock while invoking ROM.
Keep lock acquisition short. Release a snapshot publication lock before calling Runtime.

## Transaction and security boundaries

The existing command path checks current authority and revision before transition validation.
Validation runs under the existing commit gate. Panic becomes `Error::Panicked` without poisoning that gate.
A rejected transition writes no row, receipt, event, effect, or pending work.
Matching receipt replay keeps its original request interpretation and bypasses obsolete transition validation.
Current authority and disclosure remain mandatory. A validator cannot grant authority.

## Acceptance

Use two distinct Runtime instances and separate real database files for both SQLite and redb.
Conflicting catalogs must produce independent admission results. Updating one catalog must not change the other runtime.
Preserve rejected and panicked mutations as complete durable no-ops. A later valid action must still commit.
After a catalog change, an exact committed request must replay once without new events or work.
Revoked authority must deny replay disclosure. Changed input must fail identity matching.
Compile positive public callers and intended failures for non-Send, non-Sync, and borrowed captures.

## Evidence limits

Run targeted checks before the complete local verifier. Record source, dirty changes, lock, toolchain, command, and result.
Passing these tests does not establish arbitrary callback termination, production performance, or power-loss durability.
