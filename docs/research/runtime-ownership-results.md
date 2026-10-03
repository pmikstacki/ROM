# Runtime ownership results

Date: 2026-10-03. Status: release task 4.1 implementation, source review and combined verification complete.

## Scope and source

ROM now enforces one Runtime per shared store and one native adapter per supported database path.
The two guards have separate lifetimes. Shutdown alone does not release a retained Runtime's claim.
The [ownership guide](../storage-ownership.md) defines the public contract, adapter changes and recovery procedure.

The source baseline is `dcb1959fc01f22c3e42ced43802f2ab99af6c608`.
The tested tree includes the ownership changes recorded in [source status](evidence/runtime-ownership-2026-10-03/source-status.txt).
The [source manifest](evidence/runtime-ownership-2026-10-03/source-files.sha256) identifies Rust sources, Cargo manifests, lockfiles and verification scripts.
[Environment details](evidence/runtime-ownership-2026-10-03/environment.txt) identify the compiler and Linux container.
The [log checksums](evidence/runtime-ownership-2026-10-03/evidence.sha256) cover the retained raw logs.

## Implementation

`StorageOwnership` provides the driver-independent claim. Transparent storage wrappers forward acquisition to their underlying store.
Runtime acquires the claim before storage reads or registration and retains it with accepted work.
Incomplete custom adapters return `Unsupported` at Runtime construction.

SQLite and redb use the shared `NativeOwnership` helper before engine setup.
It resolves supported path aliases and locks a persistent private `.rom-owner` sidecar.
Unsafe sidecars and database hard links are rejected. The helper never deletes the sidecar to acquire ownership.

Maintenance retains source and destination reservations through conversion, publication and final reopen.
Publication removes only its verified private staging link after the destination directory is synced.
Errors after publication return `Unknown`. The complete destination can already exist.
Ownership adds no storage or archive format version.

Implementation stays in named modules. Crate facades contain declarations and exports.
The existing procedural macro entry points remain the Rust-required exception.
Shared helpers remove duplicate path locking, Runtime claims and subprocess supervision.

## Acceptance evidence

| Contract | Executed evidence |
| --- | --- |
| One Runtime, including wrappers | Core ownership and public consumer tests reject duplicates and permit replacement after final drop. |
| Failure and cancellation | Builder failures release claims. Retained handles and accepted work preserve claims after shutdown or caller cancellation. |
| One native owner | Real SQLite/redb opens, child processes, symlink aliases and fresh-path contention enforce exclusion. |
| Unsafe path rejection | Helper tests cover hard links, sidecar permissions, symlinks, non-regular files, metadata errors and native path encoding. |
| Pending work recovery | A Runtime action commits a notification before SIGKILL. Replacement delivers it and persists `Accepted`/`Done`; another restart sends nothing. |
| Offline exclusion | Migration, upgrade, retention and SQLite rebuild reject a live source. Conversion and publication retain both reservations. |
| Source preservation | SQLite main/WAL and dirty redb fixtures preserve source bytes. A callback that changes cwd cannot redirect publication. |
| Publication interruption | Before publication, the destination remains absent. After publication, the exact two-link artifact is rejected and recovered under its sidecar lock. |
| Atomic references | The create-reference/delete-target race uses concurrent adapter commits. Exactly one succeeds, with no dangling reference or extra event. |
| Public application | Both reference upgrade journeys preserve pending work, receipts, migrations, backup/restore, references and external attachment bytes. |

The final ownership target contains 13 tests, including child entry points.
One test exercises both native backends; the count is not a count of unique failure scenarios.
The tests use actual local databases and OS processes. Notification delivery uses a controlled in-process provider.

## Findings and corrections

The initial native tests accepted duplicate opens and hard-link aliases. The shared native guard makes these cases fail closed.
Initial maintenance tests also exposed missing source/destination exclusion; reservations now span the whole operation.

Closing a locked file alone can retain its lock while an incidental fork/exec descriptor remains open.
The guard now explicitly unlocks before closure, including failures after acquisition.
A deterministic duplicate-descriptor regression failed before this correction and passed afterward.
Drop cannot return an unlock error; descriptor closure remains the fallback.
Abrupt process exit bypasses Drop and relies on OS handle closure.

Canonicalization initially risked changing SQLite's special path behavior.
Exact `:memory:` and empty-path inputs remain independent ephemeral stores.
URI inputs beginning with `file:` are explicitly unsupported. Explicit filesystem names such as `./file:literal` remain valid.

The first process-recovery fixture proved row and receipt recovery but queued no work.
Review identified this evidence gap. An added assertion failed with zero pending records.
The corrected fixture commits a real notification through Runtime and verifies recovery across two subsequent owners.
This correction strengthens the fixture; it does not claim a newly discovered notification-runtime defect.

The query measurement harness and transitions fixture previously constructed two Runtime owners on one store.
The harness now drains and drops each owner between observed and production-control runs, outside measured intervals.
The transitions fixture uses one common constructor with its intended Resource definition.
Historical measurement archives remain unchanged; this stage adds no performance claim.

## Verification and review

The coordinator ran the following sequence on the final source tree in `rom-dev`:

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-release-measured-verification-target
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_BUILD_JOBS=2
./scripts/check
./demo/verify
./demo/run upgrade sqlite
./demo/run upgrade redb
```

The complete sequence exited with status zero.
The [combined log](evidence/runtime-ownership-2026-10-03/rom-runtime-ownership-full-check-final.log) records the checks and both upgrade results.
It includes strict OpenSpec validation, formatting, Clippy, workspace tests, documentation, compile fixtures, core dependency isolation and identity/authentication profile tests.
Demo verification also covers serve, reopen and SIGINT drain on both stores.
Separate logs retain the focused RED/GREEN checks and all-feature helper checks.

Independent specification and quality reviewers inspected the final source.
The quality reviewer requested pending-work recovery evidence and one shared child-process helper; both changes passed subsequent source review.
Reviewers reported no remaining findings. Their source reviews did not execute tests; the combined run above provides coordinator execution evidence.

## Supported limits and remaining release work

The supported native profile is local Linux storage with a trusted deployment directory.
Other platforms, network filesystems, bind-mount aliases and distributed writers are outside this acceptance result.
The protocol does not protect against an administrator who replaces files while owners run.
Direct storage calls remain trusted host interfaces, not authorization boundaries.

Process-exit tests establish the observed software recovery behavior. They do not certify storage hardware against power loss.
The controlled provider test does not promise exactly-once external delivery after an ambiguous provider response.
Operator inspection, retry/reconciliation, the real identity deployment profile, extension conformance and final packaging remain separate release tasks.
