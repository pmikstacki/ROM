# Resource transition validation

## Problem and implementation
On baseline `4927374`, `demo/tests/transitions.rs::generic_patch_cannot_reverse_a_confirmed_checkout` failed: generic patch changed a confirmed rejected payment into succeeded. Checks inside actions did not protect the generic mutation path.

`Definition<R>::validate_transition` now declares one typed callback for the actor and the previous and candidate values. The shared core first looks up the final receipt and verifies current authority, revision, and fields. It then calls the callback before it creates the commit bundle.

Create operations have no previous value. Delete operations have no candidate value. Codec normalization occurs before validation. Local unwind handling returns `Error::Panicked` without poisoning the commit gate. Definitions without callbacks retain prior behavior.

Matching receipt replay does not revalidate historical transitions; current authority and disclosure checks still apply. Validation cannot grant permissions. Stock and Checkout use the same hook for capacity, valid reservation state, deletion restrictions, immutable compensation context and confirmed terminal outcomes. These are demo policies, not framework-wide defaults.

## Executed evidence
Using Rust/Cargo 1.99.0 in `rom-dev`, from the isolated checkout, with `CARGO_TARGET_DIR=/var/tmp/rom-release-transitions`:

- `cargo test -p rom-storage-conformance --test transitions`: **4 passed**, each exercises SQLite and redb. Covers all mutation kinds, typed and generic invocation, unchanged journal/work/revisions on rejection, absence of rejected receipts, obsolete-transition receipt replay, revoked and row-policy-denied callers, panic isolation, normalized codecs, and stale-revision rejection before validation.
- `cargo test -p rom-demo --test transitions --test compensation`: **3 transition tests plus 1 existing compensation test passed**. Actual TCP `/invoke` replacements are rejected on both adapters. Context-retarget patches, invalid reservation states, overflow and protected deletion are rejected. Valid targeted compensation and service revocation still work.
- `cargo clippy -p rom -p rom-demo -p rom-storage-conformance --all-targets -- -D warnings`: passed.
- `cargo fmt --all`: applied; final format check and `git diff --check` passed.
- OpenSpec `enforce-resource-transitions --strict`: passed.

The failing baseline regression log is `/var/tmp/rom-transitions-red.log` inside the container. There are no adapter-specific controllers, wire changes, storage migrations or new dependencies.

## Limits
The callback is trusted native code. The author must keep it pure and bounded, with no external I/O or runtime reentry. It executes under the existing commit gate on the supervised blocking executor. Thus, expensive callbacks serialize commits. This implementation provides no preemption or latency guarantee.

Declared validators decode current and candidate values. Definitions without a validator do not add this decoding. Stock validation takes time proportional to the reservation count. The existing Resource size limit bounds that count; validation is not constant-time.

This interface validates one Resource transition. It does not add cross-Resource referential integrity or an exactly-once external effect mechanism. Existing invalid stored state is not silently repaired. Upgrade/recovery integration and broad performance measurements belong to subsequent release stages.
