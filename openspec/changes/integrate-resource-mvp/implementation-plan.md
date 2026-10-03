# Maintained Resource foundation implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development or superpowers:executing-plans task-by-task. This is the first integration package; the wider MVP remains governed by tasks.md.

**Goal:** Promote the verified typed probe into reusable library crates. Before you attach providers and transports, correct the bounded I/O and lifecycle defects.

**Architecture:** Preserve the probe's Resource/Field/Definition/Action contract and single registration authority. Separate public authoring from execution and persistence internals. Keep concrete databases outside `rom`; introduce a bounded blocking-I/O boundary for synchronous adapters and use the host's shared Rayon pool for CPU proposals.

**Tech stack:** Rust 1.99 initial tested floor, Tokio 1.53.1, Rayon 1.12.0, Syn 3.0.6, SQLite reference adapter. Existing probe pins are starting inputs, subject to dependency review before MVP release.

**Spec:** design.md, specs/integrated-mvp/spec.md, ../../config.yaml, ../../../docs/quality.md.

## Global constraints

- One Resource model; no per-kind repository/controller/subscription code.
- The first verified deployment profile is one runtime owning writes.
- Native Rust plugins first; HTTP and database drivers stay outside core.
- Retain upstream commits on downstream failure; no implicit compensation.
- `#![forbid(unsafe_code)]` and `#![deny(unused_must_use)]` in maintained core.
- One explicit test command runs in native rom-dev; no GitHub Actions.
- This package is an incomplete foundation, not permission to mark MVP complete.

## Review focus

- Two simultaneous shutdown callers, or cancellation of the first: accepted work must remain drainable.
- Expired/revoked actors while waiting for output: no disclosure of stale results.
- Unbounded scan and many live handles: fail at configured limits before uncontrolled retention.
- Handwritten definitions with unstable descriptors: use the frozen accepted descriptor consistently.
- Adapter or policy panic: return a classified error or explicit terminal runtime state without accidental partial commit.

### Task 1: Retain tested source as maintained workspace

**Files:** root Cargo.toml/Cargo.lock; crates/rom, crates/rom-derive, crates/rom-sqlite; examples/consumer; tests/compile; scripts/check and scripts/check-rust.

**Interfaces:** preserve `Resource`, `Field`, `Definition<R>`, `Action<R,I>`, `Command<R>`, `Storage`, `Runtime`, `Snapshot<R>` while renaming packages from rom-probe to rom. `rom` reexports derive behind a default `derive` feature; manual Resource implementations work with default features disabled.

- [ ] Copy tracked source only from probe commit 86780041a9824c434054fae25509dff0e56a7212, excluding targets, fixtures lockfile drift and generated docs.
- [ ] Rename packages/imports, preserve explicit derive crate override, set version 0.1.0-alpha.1 and rust-version 1.99; add MIT metadata, workspace lints and the root lockfile.
- [ ] Run the original 18 integration tests plus five compiler fixtures and renamed consumer unchanged semantically; expected all pass after package renaming.
- [ ] Add `cargo check -p rom --no-default-features --locked`; assert no concrete driver, HTTP or policy-engine package in core's normal dependency graph.
- [ ] Add a runnable rustdoc example for a minimal Resource, registration and error handling; run doctests and rustdoc with warnings denied.
- [ ] Commit foundation with explicit remaining gaps, without claiming the full MVP is ready.

### Task 2: Bounded read and observation lifecycle

**Files:** crates/rom/src/{lib,resource,execution,persistence,query}.rs; crates/rom-sqlite/src/lib.rs; examples/consumer/tests/{integrated,lifecycle}.rs.

**Interfaces:** introduce public `Limits` with `actions`, `io_jobs`, `subscriptions`, `snapshot_rows`, `snapshot_bytes`, `command_bytes`; `Builder::limits(self, Limits) -> Self`. Defaults are conservative testable policy (8, 8, 64, 1024, 1 MiB, 16 KiB respectively), validated nonzero. Change `Runtime::read` and `Runtime::query` to async, retaining their semantic result types. Change `Storage::snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>>`; adapters reject overflow without returning a truncated success. `Runtime::live` takes a lifetime subscription permit; `Live::changed` performs authorized async query.

- [ ] Write regressions `snapshot_limits_reject_before_return`, `subscription_capacity_released_on_drop`, `blocking_storage_does_not_block_tokio_heartbeat`, and `live_closes_on_shutdown`; confirm expected failure before implementation.
- [ ] Split probe modules by responsibility while retaining public reexports; keep one frozen registry and no duplicated schema logic.
- [ ] Implement bounded blocking-I/O scheduling with work-owned permits and tracked jobs; do not release a permit on caller cancellation while the blocking operation continues.
- [ ] Enforce scan bounds inside the SQLite adapter and the contract; use checked byte arithmetic. Test exactly-at-limit, limit-plus-one, and unsupported capability errors.
- [ ] Re-run original suite and new lifecycle tests; ensure resource-specific application code only changes `.await` and imports.
- [ ] Commit after fmt, Clippy and tests.

### Task 3: Cancellation-safe drain and freshness

**Files:** crates/rom/src/{execution,policy}.rs; examples/consumer/tests/lifecycle.rs.

**Interfaces:** preserve `Runtime::shutdown(&self) -> impl Future<Output=Result<()>>`; closure is durable runtime state, not a JoinSet moved into the caller future. Trusted Actor gains optional expiry through a host constructor/builder; existing trusted embedded construction remains explicit. Core owns a clock seam for deterministic tests. No transport may deserialize a request directly into a trusted actor.

- [ ] Write `cancelled_shutdown_can_be_joined_again`, `concurrent_shutdown_waits_for_same_work`, `expired_actor_denied_at_result_and_live_delivery`, and `shutdown_races_admission_without_orphan`; confirm failures in the promoted baseline.
- [ ] Keep accepted work and completion state owned by Runtime; every shutdown observer waits for the same drain condition. Close live observers and admission consistently.
- [ ] Test expiry and revocation at admission, immediately before commit, and before result/live disclosure. Use a test clock/gates to keep tests independent of wall-clock sleeps.
- [ ] Apply independent promotion review findings with a regression for each substantive defect; preserve explicit current-state and durable-idempotency contracts.
- [ ] Run the full native verifier plus no-default-feature check and commit. Produce a source-pinned verification note listing remaining auth projection, durable worker, transport and packaging work.

## Subsequent integration packages

After this foundation stabilizes public interfaces, add separately reviewable plans for:

- Shared SQLite/redb conformance and versioned journal/receipt budgets.
- Typed durable reaction registrations/work recovery.
- Projected policy and provider/User mapping.
- Configuration/blob/notification adapters.
- Generic HTTP and packaged consumer.

Those packages must reference these actual APIs and their own focused acceptance tests. They are required by tasks.md, not replaced by this planning list.
