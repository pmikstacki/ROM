# Resource transition validation

## Problem and implementation
On baseline `4927374`, `demo/tests/transitions.rs::generic_patch_cannot_reverse_a_confirmed_checkout` failed: generic patch changed a confirmed rejected payment into succeeded. Action-only checks did not protect the generic mutation path.

`Definition<R>::validate_transition` now declares one typed callback over actor, previous and candidate values. The shared core calls it after final receipt lookup, current authority/revision/field checks and before commit-bundle creation. Create/delete use absent previous/candidate values. Codec normalization precedes validation. Local unwind handling returns `Error::Panicked` without poisoning the commit gate. Definitions without callbacks retain prior behavior.

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
The callback is trusted native code: purity, bounded execution and absence of external I/O/runtime reentry are author obligations. It executes under the existing commit gate on the supervised blocking executor, so expensive callbacks serialize commits; no preemption or latency guarantee is claimed. Opted-in validators decode current/candidate values; undeclared validators do not add this decoding. Stock validation is linear in reservation count, bounded by the existing Resource size limit, not constant-time.

This seam validates one Resource transition; it does not add cross-Resource referential integrity or an exactly-once external effect mechanism. Existing invalid stored state is not silently repaired. Upgrade/recovery integration and broad performance measurements belong to subsequent release stages.
