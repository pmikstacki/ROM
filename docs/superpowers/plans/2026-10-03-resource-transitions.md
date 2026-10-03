# Resource Transition Validation Implementation Plan

> Execute inline with test-driven-development; no further delegation needed.

**Goal:** Make declared Resource business invariants apply to every generic mutation.
**Architecture:** `Definition` has an optional typed callback. The shared authorized/CAS-checked precommit boundary calls one erased method. There are no adapter or transport special cases.
**Tech Stack:** Existing Rust ROM core, SQLite/redb conformance, Tokio.

- [x] Add `demo/tests/transitions.rs` regression proving terminal outcome direct write bypass and unchanged revision/journal/work on rejection; observe failure.
- [x] Extend `crates/rom/src/resource.rs` with optional validator and typed erased dispatch; add shared call in `execution.rs` with local panic isolation.
- [x] Add demo Stock/Checkout validators in `demo/src/compensation.rs`.
- [x] Add `tests/persistence/tests/transitions.rs` covering create/replace/patch/action/delete, replay, authorization, normalized values, no durable effects and validator panic on both adapters.
- [x] Run targeted suites, existing compensation, fmt/Clippy, and strict OpenSpec. Record evidence and commit.
