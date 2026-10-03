# Task 1: Host signal lifecycle

Status: source frozen for independent review. No commit made.

Baseline: `60960510d455bee77906e4cb9807778a21f4082c`.
The nine owned files and Cargo lock identity are recorded in `source.json`.
`source/` contains the frozen files. `before/` contains their tracked baseline versions.
`changes.patch` includes tracked changes and new files.

## Changes

`host_signals.rs` installs SIGINT and SIGTERM receivers synchronously before readiness.
Both serving modes use this receiver. Provider authentication drain ordering remains unchanged.

`serving.rs` owns the existing synthetic server flow.
The command delegates after its existing argument validation.
The inspected synthetic cleanup awaits BlobService shutdown before HTTP closes Runtime.
Common cleanup also closes the blob service and Runtime after serving errors, preserving the first error.
The synthetic readiness and final status output remain unchanged.
Existing public APIs remain available. `rom_demo::serving::run` is additive.
The library facade contains declarations and exports only.

Four new default, Node-free tests run the actual binary with an ephemeral loopback port.
They send SIGINT or SIGTERM when the first readiness line becomes visible.
Both SQLite and redb must exit successfully and report stopped intake, zero owned work, and no failure.
Each test then reopens its database and reads the existing attachment through the public BlobService API.
The attachment upload finishes before readiness. These tests do not pause an accepted attachment operation.

The review amendment adds a provider SIGTERM test using the existing immediate-readiness fixture.
One provider process per store must exit successfully and leave its reopened database empty.
The existing SIGINT test keeps its sixteen launches per store through the same helper.
Both provider signal tests cover idle serving, with no accepted authentication job.

Tests reuse the existing Scratch and bounded Process helpers.
Process has one additive `pid()` accessor.
Its narrow `dead_code` allowance permits separately compiled fixtures that only need wait or kill.
Readiness and child completion have ten-second bounds. Captured output has a 64 KiB read bound.
Failed Scratch directories remain available for diagnosis.

## Executed evidence

Commands ran in `rom-dev` at `/workspace/ROM/.worktrees/release-query-index`.
Cargo commands used this environment:

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-release-measured-verification-target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2
```

Compiler: `rustc 1.99.0 (b940084d7 2026-09-28)`.
Cargo: `cargo 1.99.0 (5f94df478 2026-08-27)`.

| Command | Result | Evidence under `../evidence/` |
| --- | --- | --- |
| `cargo test -p rom-demo --test server_lifecycle --locked -- --test-threads=1` before implementation | Both SIGTERM cases failed with signal 15. Both SIGINT cases passed. | `task1-signals-red.log` |
| `cargo test -p rom-demo --test server_lifecycle --locked` after implementation | Four cases passed. | `task1-signals-green.log` |
| `cargo test -p rom-demo --locked` | Passed; provider cases disabled by their feature gates. | `task1-covering-green.log` |
| `cargo test -p rom-demo --all-features --locked` | Passed, including twelve provider-profile tests and four signal cases. | `task1-covering-green.log` |
| `cargo clippy -p rom-demo --all-targets --all-features --locked -- -D warnings` | Passed. | `task1-covering-green.log`, `task1-final-green.log` |
| `cargo clippy -p rom-storage-conformance --all-targets --locked -- -D warnings` | Passed after the shared accessor allowance. | `task1-final-green.log` |
| `cargo test -p rom-demo --test server_lifecycle --locked` on the final helper/source | Four cases passed. | `task1-final-green.log` |
| `rustfmt --edition 2024 --config skip_children=true --check` with all eight owned paths | Passed. | `task1-final-green.log` |
| `git diff --check` | Passed. | Executed during freeze. |
| `cargo test -p rom-demo --features provider-profile --test provider_profile readiness_allows_immediate --locked` | Both SIGINT and SIGTERM provider tests passed on both stores. | `task1-provider-signal-amendment-green.log` |
| `cargo clippy -p rom-demo --all-targets --all-features --locked -- -D warnings` after the test amendment | Passed. | `task1-provider-signal-amendment-green.log` |
| `rustfmt --edition 2024 --config skip_children=true --check demo/tests/provider_profile/commands.rs` | Passed. | `task1-provider-signal-amendment-green.log` |

The initial test import compile error is retained in `task1-signals-compile-red.log`.
It is separate from the behavioral RED evidence.
The host had no rustfmt executable, so formatting ran inside `rom-dev`.
A mistyped package name is retained in `task1-helper-fmt-clippy.log`.
The subsequent native Clippy diagnostic is retained in `task1-shared-helper-clippy.log`.

The default broad test command preceded a formatting-only change.
The all-feature test command compiled the formatted implementation.
Final focused tests and both Clippy checks ran after the test-helper allowance.
No behavior source changed after these checks.
The review amendment changes only `demo/tests/provider_profile/commands.rs`.
Its before/after patch and hash are in `signal-amendment/`.
The original report and source manifest remain there as historical evidence.
This added coverage passed against the existing fix; it has no separate pre-fix RED claim.

## Scope and limits

SIGINT did not reproduce the old registration race in these tests.
The fix's ordering is visible in source; the new bounded tests verify the resulting shutdown behavior.
These tests cover Unix signals, ordinary process shutdown, and exclusive reopen.
They do not claim power-loss, SIGKILL, or arbitrary Rust callback preemption guarantees.
The provider suite uses its existing synthetic provider and immediate SIGINT/SIGTERM process tests.
This task does not add a new real-provider acceptance claim.

| Component | Evidence and limit |
| --- | --- |
| Synthetic process | Four readiness-time signal cases report zero owned work and reopen completed durable attachment content. No paused attachment-drain assertion. |
| Provider process | SIGINT and SIGTERM are delivered on readiness; successful exit and empty native reopen show idle process shutdown. |
| Provider authentication | Existing `demo/tests/provider_profile/serving.rs::stop_drains_paused_authentication_before_closing_runtime` ran in the all-feature suite. It pauses accepted authentication and verifies ordered drain through the public serving seam. |
| Blob publication | Existing `crates/rom-blob/tests/lifecycle.rs::canceled_waiter_retains_capacity_and_shutdown_drains_publication` exercises accepted paused publication. Its source was inspected here; this task did not rerun it. |
| Synthetic cleanup | Source inspection confirms that BlobService shutdown is awaited before Runtime shutdown. Process tests do not establish every active-operation case. |

Native format 8 and archive format 6 remain unchanged.
No dependency or manifest changed in this scope.
Concurrent release/package scripts and documentation changes belong to other owners.
The root coordinator owns the full local verifier before integration.
