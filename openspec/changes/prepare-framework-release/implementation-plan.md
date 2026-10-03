# Reference application recovery implementation plan

> **For agentic workers:** Use superpowers:executing-plans for task-by-task execution; independently review the completed slice.

**Goal:** Demonstrate an interrupted application compensation chain with public APIs on both maintained stores.

**Architecture:** Extend the existing demo, reuse its registered Resources/actions and ordinary runtime work processing. Keep process fault injection in integration tests. Core owns durable state and work; the application owns business meaning.

**Tech Stack:** Existing Rust 1.99, Tokio, ROM, SQLite/redb; no new dependencies.

**Spec:** `specs/framework-readiness/spec.md`, first requirement and scenario.

## Global constraints

- No direct database writes, new controller or alternative mutation path.
- Preserve unrelated committed reservations and command identities.
- The command's close/reopen and the test's process exit are different evidence.
- Full release stages remain open after this slice.

## Review focus

- Replay after reopening must not duplicate downstream effects.
- Unknown payment outcome must not be inferred as confirmed rejection.
- An unrelated reservation must survive compensation.
- Current authority must govern query/live access after restarting.
- Test process exit must occur after commit and before work processing, without Runtime shutdown.

## Task 1: Public application journey

Files: create `demo/src/reference.rs`, `demo/tests/reference.rs`; modify
`demo/src/lib.rs`, `demo/src/main.rs`, `demo/README.md`.

Interfaces: `reference::prepare(&Runtime) -> rom::Result<()>`,
`reference::recover(&Runtime) -> rom::Result<()>`,
`reference::run(bool) -> smoke::SmokeResult<()>`.

- [x] Write a CLI test invoking `rom-demo reference sqlite` and `redb`; assert successful completion and the recovery summary. Observe failure for the absent command.
- [x] Implement preparation through existing public Resource actions with stable identities; unknown outcome holds both reservations; confirmed rejection remains pending.
- [x] Implement recovery: verify initial state, observe filtered live stock, process bounded work, assert only checkout-a released, replay rejection and assert unchanged Stock and Checkout journal heads.
- [x] Expose the command using fresh scratch databases, shutdown and reopen; clean up only its owned scratch directory.
- [x] Add subprocess tests: child prepares then exits with code 86 without destructors; parent verifies code and recovers persisted data on each store. Check denied identity and unknown outcomes with existing application authority.
- [x] Run demo tests, formatting and Clippy; document exact limits and reproduce commands.
- [x] Obtain independent review, resolve findings, run repository check and commit.

Later stages receive separate implementation plans after source inspection; this
plan deliberately does not invent migration or index interfaces before that work.
