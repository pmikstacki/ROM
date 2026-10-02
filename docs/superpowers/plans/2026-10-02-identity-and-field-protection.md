# Identity and Field Protection Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement each task in this assigned worktree. Root coordinates parallel core changes.

**Goal:** Connect verified identities to ordinary User/provider Resources and protect current identity and field access at every disclosure boundary.

**Architecture:** Credential verification remains in optional `rom-auth`; generic principal identity and authorization checkpoints live in core. A separate `rom-identity` package supplies native derived Resources and host integration. Partial projected views never masquerade as complete typed Resources.

**Tech Stack:** Rust 1.99, existing ROM derive/runtime/storage, optional rom-auth; no new policy engine.

**Spec:** `openspec/changes/design-provider-neutral-auth/specs/provider-neutral-auth/spec.md`, `openspec/changes/integrate-resource-mvp/specs/integrated-mvp/spec.md`.

## Global Constraints

- Resource is the sole managed domain model, including User and IdentityProvider configuration.
- No credentials or serialized actor construction in core, receipts, events, or diagnostics.
- Blocking authoritative reads run through bounded I/O work; no recursive Runtime calls from an authorizer.
- Current authorization before commit and receipt/live/journal disclosure; generation revalidation before returning I/O results.
- Only this worktree is writable; coordinate execution changes with transport and reaction owners.
- Native rom-dev Rust 1.99, CARGO_BUILD_JOBS=2, locked dependencies, own target directory.

## Review Focus

- Human and service with identical authority and subject must have distinct revocation and receipt identities.
- Disable/re-enable and link/config replacement must not revive previously issued actors.
- A mutation racing with authorization must not disclose through an old completion or cached receipt.
- Filtering a hidden field must be denied before observing whether any row matches.
- A writable hidden field must commit without appearing in action responses or subsequent historical outcomes.

### Task 1: Generic identity scope and authoritative gate

**Files:** `crates/rom/src/policy.rs`, coordinated sections of `execution.rs`, new integration tests.

**Interfaces:** `PrincipalKind::{Embedded,Human,Service}`, `Actor::principal_kind`, trusted kind construction, private host stamp getter/builder; `ActorGate::check(actor, read_only_storage)` and `Builder::actor_gate`.

- [ ] Write failing tests for kind-separated revocation/receipt scope and denied/error/panicking gates.
- [ ] Add kind to Actor key and receipt identity without serializing actor or credential state.
- [ ] Keep `check_actor` cheap; invoke the authoritative gate inside bounded I/O and mutation commit gates. Preserve generation revalidation before async delivery.
- [ ] Test gate storage reads execute off async threads, current denial blocks initial and cached actions, and gate errors fail closed.
- [ ] Run core tests/clippy and commit independently.

### Task 2: Native identity Resources and proof mapping

**Files:** new `crates/rom-identity/{Cargo.toml,README.md,src/lib.rs,tests/integration.rs}`, workspace manifest/lock.

**Interfaces:** derived `User`, `IdentityProvider`, `IdentityLink`; explicit canonical `(authority,kind,subject)` link key; host mapping verified proof plus activation revision to stamped Actor; gate resolves bounded direct lookups.

- [ ] Write tests creating all identity records through ordinary Runtime actions, with real verified evidence supplied by auth fixture.
- [ ] Implement explicit linking without email matching, duplicate entity engines, or unbounded scans. Missing/disabled/mismatched records deny.
- [ ] Bind provider verification to its captured activation/config revision; capture link and User revisions in private host stamp. No automatic allow-list for embedded actors: host explicitly configures administrative/service trust.
- [ ] Test issuer/kind collision, expired proof, unlinked subject, wrong provider revision, disabled User/provider, unlink/relink, disable/re-enable, commit race, and cached retry/live delivery after revocation.
- [ ] Document first-admin bootstrap as host-owned setup using shared actions, with no default administrative identity or bypass endpoint.
- [ ] Run package and workspace checks and commit.

### Task 3: Field authorization and projected views

**Files:** `resource.rs`, `query.rs`, new `projection.rs`, coordinated `execution.rs`/transport disclosure; integration tests/examples.

**Interfaces:** explicit field read/write policy and schema-level query-field permission; `ProjectedView` with key/revision/optional authorized field map; `invoke_projected`, `read_projected`, `query_projected`, `live_projected`.

- [ ] Test hidden fields absent from projected results, full typed reads denied when incomplete, and hidden filter predicates denied even on empty datasets.
- [ ] Add default-deny field permissions, validate requested and derived mutation fields before commit, and protect both current and historical receipt fields.
- [ ] Keep full typed Resource results complete; expose partial views only through projection API. Shared execution runs once for typed and projected callers.
- [ ] Apply projection to live and journal delivery with current authorization; coordinate HTTP consumption with transport owner.
- [ ] Update fixtures to explicitly allow intended fields; test mixed forbidden writes roll back and write-without-read outcomes omit protected values.
- [ ] Run locked workspace/auth/consumer checks, document unsupported query forms, and commit.

## Validation command

Run within rom-dev at `/workspace/ROM/.worktrees/configuration-trials`, using `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=$PWD/target/identity`, `cargo test --workspace --locked`, workspace clippy with warnings denied, auth verifier, and repository check script. Capture actual outcomes before completion claims.
