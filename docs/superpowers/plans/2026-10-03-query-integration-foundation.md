# Query integration foundation plan

> **For agentic workers:** Use superpowers:executing-plans for assigned tasks.

**Goal:** Prepare cohesive core boundaries and specify maintained query strategy selection.
**Architecture:** One canonical query evaluator retains authorization and error semantics.
Adapters own coherent native read sessions, physical indexes and database planning.
**Tech Stack:** Existing Rust 1.99.0, Tokio/Rayon, SQLite and redb workspace.
**Spec:** Release stage 3 in `openspec/changes/prepare-framework-release/specs/framework-readiness/spec.md`.
The source review is `docs/research/maintained-selector-integration-review.md`.

## Constraints

Keep `lib.rs` and `mod.rs` as facades. Preserve existing public paths and behavior.
Keep mutation order, current authority checks, drain ownership and retry epochs intact.
Keep historical prototypes and evidence unchanged. No new execution engine dependency.
Do not infer authorization equivalence from function addresses or observed results.
Do not mark indexed execution complete from a reference-only adapter or an interface stub.

## Tasks

- [x] Split `crates/rom/src/resource.rs` into cohesive field, query authoring,
  definition, schema and command modules. Preserve crate-level exports and visibility.
  Use existing public contract, query, derive and consumer tests as behavioral oracles.
- [x] Split `crates/rom/src/execution.rs` into composition, lifecycle, authority,
  command execution and shared runtime state. Keep the complete mutation sequence
  in one command module. Preserve drop-before-drain ordering and actor rechecks.
- [x] Record the native index layout and semantic gate reviews. Select a coherent
  adapter boundary with exact admission metadata and explicit authorization support.
  Distinguish pure costs from capabilities and execution errors from estimate failure.
- [ ] Write the stage-3 design and executable implementation plan after review.
  Include atomic index maintenance, native format compatibility, rebuild/restore,
  differential tests, holdout benchmarks and measurement of writes and memory.
- [x] Review the structural changes independently. Run the full local verifier
  and the external consumer before integrating the foundation.

## Review focus

Opaque row callbacks can panic even on a row excluded by a later predicate.
Sorted queries check visible rows' sort-field grants before predicate filtering.
ID queries stop policy evaluation after filling a page. Preserve both behaviors.
Resource module privacy must not silently expose implementation types as public API.
Splitting lifecycle code must not publish a drained state before adapter owners drop.
The work in this plan does not complete stage 3.2 or establish a performance gain.
