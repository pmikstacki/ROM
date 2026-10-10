# Captured transition validation implementation plan

> **For agentic workers:** Use superpowers:executing-plans for the assigned implementation. Preserve evidence and do not commit without coordinator authorization.

**Goal:** Let independent runtimes validate transitions against captured application snapshots without changing commit or replay semantics.

**Architecture:** Retain the existing Definition setter and shared command pipeline. Store a private Arc callable and accept a generic synchronous callback.

**Tech Stack:** Rust 1.99, existing ROM workspace, real SQLite and redb, existing Node compile-fixture harness.

**Spec:** `openspec/changes/support-captured-transition-validation/design.md` and its `specs/resource-transition-validation/spec.md`.

## Global constraints

- No new dependencies, formats, public API paths, global catalogs, or changes to other callback contracts.
- Callback bounds are `Fn(&Actor, Option<&R>, Option<&R>) -> Result<()> + Send + Sync + 'static`.
- Preserve unrelated changes, historical artifacts and test evidence.
- Wait for coordinator toolchain/target allocation before builds. Use separate logs for expected failure and passing checks.
- The coordinator owns full local verification and compile-harness integration. This worker owns only its assigned files and new captured-validator fixtures.

## Review focus

- Independent catalogs: two distinct Runtime instances, not two Definition values.
- Rejection/panic: verify complete durable snapshots and a later valid operation.
- Replay after catalog change: callback count stays unchanged and no new event/work appears.
- Revocation/input mismatch: current authority and exact identity still control receipt access.
- Compatibility bounds: named function/pointer/non-capturing closure pass; Rc, Cell, and borrowed captures fail for intended reasons.

## Task 1: Write the failing acceptance tests

**Files:** `tests/persistence/tests/transitions.rs`, `crates/rom/src/resource/definition_tests.rs`; new `tests/compile/src/bin/captured_validator_{valid,non_send,non_sync,borrowed}.rs`.

**Interfaces:** Reuse `Fixture::with_builder` and its actual native adapters. Add captured acceptance cases directly to transitions.rs to avoid integration-test autodiscovery ambiguity.

- [x] Add two-Runtime conflicting catalog acceptance using `Arc<RwLock<Arc<u64>>>` snapshots and separate Fixture databases.
- [x] Add captured rejection/panic, exact replay after catalog update, input mismatch and revoked authority assertions.
- [x] Add public compile-pass and bound-rejection fixtures. Coordinator adds intended primary diagnostics to the shared harness.
- [x] Run `cargo test -p rom-storage-conformance --test transitions --locked` with coordinator allocation. Expected pre-change failure: captured closures cannot coerce to the current function pointer.

## Task 2: Make the narrow callable change

**Files:** `crates/rom/src/resource/definition.rs`.

**Interfaces:** `pub fn validate_transition<F>(self, validate: F) -> Self` with the global callback bounds.

- [x] Replace private fn-pointer storage with Arc callable ownership.
- [x] Borrow the callback at erased validation and retain existing decoding/order.
- [x] Document per-evaluation immutable snapshots, no I/O, bounded/non-reentrant execution, lock ordering and current-authority replay.
- [x] Run the targeted transition suite and `cargo test -p rom --lib --locked`.

## Task 3: Verify and hand off

- [x] Run positive external caller and intended compile-failure fixtures. Preserve diagnostic source location and reason.
- [x] Run allocated rustfmt, affected Clippy, and strict OpenSpec validation.
- [x] Record source identity, dirty diff, lock hash, compiler, commands and outcomes in coordinator evidence.
- [ ] Request independent review through coordinator. Report full-verifier completion separately from targeted results.

## Execution ledger

- Source investigation completed before this plan. The owner authorized inline implementation without another approval flow.
- Plan self-review maps all requirements to tasks. No application data migration is needed.

- Baseline: four existing transition tests passed. New captured cases failed with E0308 before production changes.
- Green: six transition tests and 96 core unit tests passed. Three external negative fixtures failed at primary line 9 for intended bounds.
- Initial external positive fixture omitted required Resource metadata; corrected it and retained the failed log.
- Initial affected Clippy rejected a test-only tuple type; introduced the Counts alias and retained the failed log.
- Corrected affected Clippy and strict change validation passed. Full workspace verification remains coordinator-owned.
- Evidence: `/var/tmp/rom-010-r1-evidence/`. Toolchain: Rust/Cargo 1.99.0; jobs 2; incremental disabled.
- Final transition rerun passed all six tests; full integrated compile harness passed all intended diagnostics and positive programs.
