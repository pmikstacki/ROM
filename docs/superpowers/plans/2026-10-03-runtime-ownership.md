# Runtime ownership implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce one Runtime owner and one native deployment owner, including offline maintenance and restart recovery.

**Architecture:** Core uses a driver-neutral RAII claim. Native adapters use a separate canonical-path lock file held for the native handle lifetime. Maintenance retains source and destination guards through publication.

**Tech Stack:** Rust 1.99, `std::fs::File::try_lock`, current SQLite/redb adapters, existing private `Stage` publication.

**Spec:** [Runtime and native storage ownership](../specs/2026-10-03-runtime-ownership-design.md).

## Global constraints

- Keep the core independent of filesystem paths and database drivers.
- Keep Resource actions, queries, reference integrity, receipts and reaction recovery semantics unchanged.
- Use the supported local Linux profile first; reject unvalidated platforms explicitly.
- Support canonical symlink aliases; reject database hard links and unsafe ownership sidecars.
- Use nonblocking locks. Never delete an ownership sidecar to steal a lock.
- Release the Runtime claim at final Runtime/work drop, not at shutdown acknowledgement.
- Preserve source database and WAL bytes. Persistent owner sidecars are documented metadata.
- Preserve non-overwriting publication and bounded snapshot validation.
- Keep `lib.rs` and `mod.rs` as facades; implementation belongs in cohesive named modules.
- No implementation or compilation occurred while writing this plan. No release task is marked complete here.

## Review focus

1. Wrapper-local ownership state must not permit a second Runtime on the same store; task 1 tests forwarding.
2. A retained clone or cancelled waiter must not release a live claim; task 1 tests actual owner lifetime.
3. A symlink, hard link or replaced sidecar must not create an independent owner; task 2 tests identity checks.
4. Publishing a destination must not introduce an unlocked interval or strand an unexplained two-link artifact; task 4 tests both publication windows.
5. Runtime exclusion must not remove atomic reference race evidence; task 3 moves the race to the adapter contract.

## File and responsibility map

| Area | Files | Responsibility |
| --- | --- | --- |
| Core ownership | `crates/rom/src/storage_ownership.rs`, `storage_ownership/tests.rs` | Shared claim and non-cloneable guard |
| Runtime integration | `crates/rom/src/persistence.rs`, `execution/builder.rs`, `execution/state.rs`, `lib.rs` | Trait method, acquisition and guard lifetime; facade export only |
| Native helper | `crates/rom-backup/src/native_ownership.rs`, `native_ownership/path.rs`, `native_ownership/tests.rs`, `lib.rs` | OS lock, path checks and facade exports |
| Native adapters | SQLite/redb `store.rs`, `persistence.rs` | Open-before-engine lock, retained guard, Runtime claim forwarding |
| Maintenance | SQLite `snapshot.rs`, `maintenance.rs`, `upgrade.rs`, `migration.rs`, `retention.rs`, `index_rebuild.rs`; redb `preflight.rs`, `maintenance.rs`, `migration.rs`, `retention.rs` | Whole-operation source exclusion and destination transfer |
| Publication | `crates/rom-backup/src/publication.rs`, separate publication tests | Remove only known staging alias after durable publication |
| Conformance | `tests/persistence/tests/ownership.rs`, `references.rs`; affected consumer/wrapper fixtures | Lifetime, processes, aliases, maintenance and preserved atomic race |
| Documentation | adapter READMEs, core author/adapter guide, maintenance guide, release OpenSpec | Public contract, migration and precise recovery procedure |

The helper crate already provides native maintenance utilities to both adapters.
Do not put ownership logic in archive codecs or duplicate it in both native crates.
Use a private adapter open helper that consumes an acquired native guard.

## Task 1: Core claim and custom adapter migration

**Produces:** `StorageOwnership::acquire() -> Result<StorageOwner>` and object-safe `Storage::acquire_owner() -> Result<StorageOwner>`.
The trait default returns `Unsupported`; duplicate acquisition returns `Conflict`.

- [ ] Add RED unit tests for acquire/reject/drop/reacquire and helper clones sharing one claim.
- [ ] Add a public consumer RED test for two builders on one store, including a transparent wrapper.
- [ ] Add a failure-after-acquisition test proving registration failure releases the claim.
- [ ] Add lifecycle tests: stopped retained clone rejects replacement; final drop permits it; cancelled action/drain waiter preserves accepted-work ownership.
- [ ] Run these tests and record the failing assertions or missing API diagnostics.
- [ ] Implement the shared claim in the named module and export only its public types from `lib.rs`.
- [ ] Acquire after pure builder validation, before retry-boundary reads or descriptor registration; retain the guard in `Inner`.
- [ ] Migrate maintained custom stores to one shared helper per backing state; make transparent wrappers forward acquisition.
- [ ] Preserve intentionally unsupported test adapters. Do not add successful blanket ownership to bypass build failures.
- [ ] Run core and consumer lifecycle tests, plus affected public compile fixtures and Clippy.

**Gate:** Shared backing state has one Runtime claim across wrappers, and accepted work retains it independently of callers.

## Task 2: Shared native path guard

**Produces:** `NativeAccess::{OpenOrCreate, Existing, Fresh}` and `NativeOwnership::{acquire(&Path, NativeAccess), path(&self)}` from the spec.

- [ ] Add RED tests for separate opens in one process, relative/absolute aliases, symlinks, non-UTF-8 names and missing-source behavior.
- [ ] Add RED tests for hard-linked databases, lock-file symlinks, non-regular files, multiple links, private permissions and metadata failure.
- [ ] Add a bounded child test: child holds lock, parent gets `Conflict`, child exits, parent acquires without deleting the sidecar.
- [ ] Add two-process fresh-path creation contention. Assert only one owner can pass acquisition before engine setup.
- [ ] Run focused helper tests and record RED evidence.
- [ ] Implement canonical path handling and persistent mode-`0600` sidecar creation without truncation.
- [ ] Implement `File::try_lock`, post-acquisition identity checks and specified error mapping. Reject unsupported targets rather than silently disabling checks.
- [ ] Keep checks and acquisition in separate named modules; do not embed implementation in facades.
- [ ] Run helper tests and all-feature Clippy. Use bounded wait/kill cleanup for every child process.

**Gate:** A native guard identifies one supported canonical path, fails closed for unsupported aliases, and releases through OS handle lifetime.

## Task 3: Native open and reference-race preservation

**Consumes:** Both ownership helpers.
**Produces:** Public native open signatures stay unchanged; private open helpers consume native guards.

- [ ] Add RED SQLite/redb tests proving second handles and second processes fail before engine setup or schema registration.
- [ ] Add RED native tests for builder claim forwarding, failed open cleanup and persisted pending-work recovery after owner process exit.
- [ ] Assert unsupported native formats still leave source bytes unchanged, including SQLite WAL and dirty redb source fixtures.
- [ ] Replace the old two-Runtime reference-race setup with a separate second-owner rejection case.
- [ ] Preserve its original create-reference versus delete-target assertions using concurrent direct Bundle commits on the same adapter.
- [ ] Run the focused tests to capture expected ownership failures before production changes.
- [ ] Acquire native ownership before SQLite connection setup and before redb preflight/recovery; open the engine at the canonical guarded path.
- [ ] Store the native guard after the engine fields; store a shared Runtime claim helper and forward `acquire_owner`.
- [ ] Audit explicit connection extraction/close paths so native ownership cannot drop while the engine remains open.
- [ ] Adapt sequential restart fixtures to drop Runtime clones and adapter handles deliberately. Do not weaken concurrency expectations globally.
- [ ] Migrate `tools/query-measure/src/run.rs`: its observed Runtime and production-control Runtime currently share one SQLite adapter. Run those read trials sequentially, drain and drop each owner before building the next, and retain the identical dataset. Preserve instrumentation-overhead measurements without bypassing the new guard.
- [ ] Run native ownership, references, existing crash recovery and focused consumer tests.

**Gate:** Both adapters enforce ownership without changing their atomic reference or crash-recovery contracts.

## Task 4: Offline exclusion and publication transfer

**Consumes:** `NativeOwnership` and existing `Stage`.
**Produces:** Unchanged public maintenance signatures and a named `Stage` native-finalization method.
Use `Stage::finish_native_publication() -> Result<()>` after successful `publish_with` to verify/remove only the owned alias and sync its directory.
Post-publication errors map to `Unknown`; public adapter return transfers the destination guard.

- [ ] Add RED tests for live-source rejection across upgrade, migration, retention and SQLite index rebuild.
- [ ] Pause an offline operation in its existing observer; assert a concurrent source open and destination open both fail.
- [ ] Test same-source/destination aliases and existing destinations; no transformation may overwrite or publish into them.
- [ ] Test `backup_to` on the existing owner without recursive acquisition, then restore under a fresh destination reservation.
- [ ] Add publication interruption hooks only under `test-support`: before hard link and after directory sync but before private-link removal.
- [ ] Before publication, child exit leaves no destination; after publication, child exit leaves a detectable two-link destination that ordinary open rejects.
- [ ] In the owned fixture, lock the known destination sidecar directly with `File::try_lock`, without opening the database. Verify the exact private artifact inode and remove only that staging link. Sync, release, then confirm normal reopen and data integrity.
- [ ] Assert unknown unrelated hard links remain untouched and rejected. Do not add broad automatic directory cleanup.
- [ ] Run RED tests; keep process timeouts and narrow failure diagnostics.
- [ ] Move source guards to top-level maintenance operations so they survive collection, conversion, publication and reopen.
- [ ] Reserve the destination before staging and consume its guard in the private final-open helper.
- [ ] Add the native finalization helper. Preserve hard-link no-overwrite publication and destination-directory fsync before removing the private alias.
- [ ] Run ownership/maintenance tests, backup/upgrade/migration/retention/rebuild tests, and the reference demo upgrade journey.

**Gate:** Maintenance and live opens exclude each other, source bytes remain unchanged, and publication interruptions have a tested recovery procedure.

## Task 5: Documentation, review and integrated release evidence

- [ ] Document custom adapter ownership migration, wrapper forwarding and both guard lifetimes.
- [ ] Document local-platform support, reserved sidecars, hard-link rejection and the prohibition on replacing files while open.
- [ ] Document interrupted-publication cleanup separately from normal process-exit recovery. Never advise deleting a lock file to acquire ownership.
- [ ] Add OpenSpec scenarios for each gate; mark task 4.1 complete only after the combined verifier passes.
- [ ] Review the implementation for guard drop order, early returns, source guard scope, callback lock re-entry and direct-storage trust boundaries.
- [ ] Verify that no unchanged data format received a needless format bump and no historical evidence was removed.
- [ ] Run formatting, scoped Clippy and the full local `./scripts/check` once on the final combined tree, after measurement work releases the build target.
- [ ] Record source revision, dirty changes, compiler, command, tests and supported deployment limitations in the stage report.
- [ ] Hand the verified changes to the coordinator for integration. This plan does not authorize a separate agent commit.

## Verification commands

Run focused gates inside `rom-dev`, from `/workspace/ROM/.worktrees/release-query-index`.
Use the existing warm target and low-debug profiles; coordinate with measurements before starting builds.

```sh
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_TARGET_DIR=/var/tmp/rom-release-relations-tests-target
cargo test -p rom storage_ownership
cargo test -p rom-backup native_ownership
cargo test --manifest-path tests/persistence/Cargo.toml --test ownership
cargo test --manifest-path tests/persistence/Cargo.toml --test references
cargo clippy -p rom -p rom-backup -p rom-sqlite -p rom-redb --all-targets --all-features -- -D warnings
```

Each focused command must pass before its task gate is reported complete.
The coordinator selects the existing lifecycle, maintenance and demo targets appropriate to changed files, then runs the final verifier.
No test execution is claimed by this planning document.
