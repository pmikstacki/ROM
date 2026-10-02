# Maintained Configuration Ingestion Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans in the assigned worktree. Root coordinates integration and independent review.

**Goal:** Load bounded JSON/TOML values into one existing Resource per reload, with compiled source ownership, durable provenance, and stale-reload protection through ordinary execution.

**Architecture:** `rom-config` contains the optional config-rs adapter and a derived SourceActivation Resource. Core knows only trusted source permits, compiled ownership, generic revision preconditions, and protected commit metadata. Target value and accepted provenance persist in the same native bundle; requested source generation lives in SourceActivation and is distinguished from accepted generation.

**Tech Stack:** Rust 1.99, config 0.15.27 with only JSON/TOML features, existing ROM runtime and SQLite/redb adapters.

**Spec:** `docs/research/configuration-resource-contract.md`, `docs/research/configuration-trial-results.md`, `openspec/changes/establish-rom/specs/resource-model/spec.md`.

## Global Constraints

- A source loads values into already registered Resource definitions; it supplies no kind/schema/permission definition.
- Source authority and compiled ownership are checked before any precedence consideration.
- Resource identities are kept outside config-rs path processing; the parsed document contains only one Resource's field values.
- Runtime policy, field checks, normalization, revisions, idempotency and atomic bundle remain the only commit path.
- Invalid/unavailable reload leaves the last accepted value and provenance intact; it is not an empty source or a delete.
- No atomic multi-Resource reload, overlay layering, partial patches, environment discovery, external writeback or remote activation is advertised by this first profile.
- Jobs=2, locked Rust 1.99 in rom-dev, own target directory; no host/network changes or main commits.

## Review Focus

- A newer failed request still invalidates an older pending completion.
- A source cannot enable itself, change its target scope, own another source's fields, or install new trust through document content.
- Metadata-only accepted changes must survive no-op value equality and restart.
- Raw parser errors and source locations can contain secrets; public errors must not contain raw values or filesystem paths.
- A crash after native commit but before acknowledgment must preserve one target/provenance pair and permit safe identity replay.

### Task 1: Generic source permission and atomic provenance

**Files:** new `crates/rom/src/source.rs`; coordinated `resource.rs`, `execution.rs`, `persistence.rs`; adapter conformance fixtures.

**Interfaces:** immutable native `SourcePermit` containing owner id, target Key, source generation/version, and one generic `(Key, expected revision)` precondition; serializable `SourceProvenance`; optional provenance on persisted Row; `Definition::source_owner(&'static str)`; `Runtime::invoke_sourced(actor, invocation, permit)` returning a projected outcome.

- [ ] Add failing tests for wrong/absent source permit, mismatched target scope, expired source actor and stale precondition.
- [ ] Route sourced invocation through the existing run function. Check the generic precondition under the commit gate before receipt lookup and conditional commit, using bounded point reads. Do not teach either adapter about configuration kinds.
- [ ] Reject source writes on kinds without explicit ownership and ordinary writes on externally owned kinds. Row and field policies still apply; a permit is not an authorization grant.
- [ ] Include provenance in the idempotency fingerprint and atomic Row/receipt bundle. Define metadata-only change as a Resource revision change with a journal fact, even if field values are unchanged. Exact same value/provenance replay stays a no-op.
- [ ] Keep provenance out of ordinary projections. Add an explicit protected metadata read policy/API for host inspection; source names and locations must not leak through allow-all field policies.
- [ ] Verify SQLite/redb persistence and restart, no-op metadata, rollback and unknown acknowledgment. Add backward-compatible absent metadata decoding only if the existing storage-version policy permits it; otherwise reject old formats explicitly.
- [ ] Run shared conformance and commit this independently reviewable core change.

### Task 2: Resource-managed source requests and bounded parsing

**Files:** new `crates/rom-config/{Cargo.toml,README.md,src/lib.rs,src/parser.rs,tests/ingestion.rs,verify}`, workspace manifest/lock.

**Interfaces:** derived `SourceActivation` with enabled, target kind/id, requested generation, safe source version and expiry; pure registered begin-reload Action increments generation; immutable ReloadTicket captures actor/activation revision/target/permit; `parse(format, document)` returns candidate fields or safe error; ticket `apply(runtime,candidate)` invokes the registered target.

- [ ] Create source activation through an explicit host actor and ordinary Resource actions. Only host policy may change enabled/scope; source actor can request a new generation only through declared allowed fields/action.
- [ ] Capture a new requested generation before fetch/parse. Failure therefore invalidates older pending completions without modifying accepted target state.
- [ ] Parse one bounded JSON/TOML field object with config-rs `Source::collect`, capturing safe origins before generic conversion. Do not merge through config path syntax or hide invalid lower contributions under an overlay.
- [ ] Keep source id, target kind/id, generation and ownership outside document control. Reject missing required fields, unknown fields, wrong/null values through ordinary Resource decoding. Arrays/maps retain the compiled field codec's semantics.
- [ ] Expose requested versus accepted generation honestly. SourceActivation describes the request and grant; target provenance is the atomic accepted state. No second active-state write is needed after target commit.
- [ ] Map parse/fetch/runtime failures to safe structured categories; do not persist raw credentials, candidate strings or parser error messages in diagnostics.
- [ ] Document complete-value replacement and explicit deletion; omission/missing fetch never means lifecycle delete. Defer partial-value and overlay handling until root's PATCH contract is integrated.

### Task 3: Integrated evidence and package gates

**Files:** configuration integration tests, README, `docs/research/maintained-configuration-results.md`.

- [ ] Use native User, IdentityProvider and application Settings declarations as the loaded targets, with explicit ownership and trust-management permission differences.
- [ ] Verify valid JSON/TOML, dotted literal Resource ids, empty/false/zero/null, invalid/unknown fields, secret-safe parser errors and source-generated schema/authority rejection.
- [ ] Verify last-valid preservation, source disable/re-enable, expired tickets, late reverse-order completion including a newer failed fetch, and concurrent target revision conflict.
- [ ] Reopen both database adapters and recover exact accepted provenance/activation state; inject rollback and lost acknowledgment to prove no split target/provenance publication.
- [ ] Run fmt, workspace tests/clippy, package docs, dependency-tree isolation, locked verifier and cargo-audit. Record exact counts and database coverage rather than claiming universal loader/provider support.

## Decisions deliberately deferred

Whole-Resource ownership is the first supported source profile. Field-level source
ownership, ordered overlays and removing an override require their own compiled
merge contract; no source-controlled priority is accepted. Expiry bounds source
write authority. Retained accepted values are ordinary persisted Resources, not
live external-service handles; consumers must separately enforce any policy that
requires their effective values to expire. Secret resolution, remote provider
activation, multi-host convergence and atomic multi-Resource publication remain
outside this bounded ingestion package.
