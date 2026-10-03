# Operator recovery implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver bounded authorized work inspection and atomic retry/reconciliation, with ordinary Resource actions for explicit compensation.

**Architecture:** Core authorizes and prepares controls; a shared ledger state machine applies CAS and durable operator receipts atomically. Native adapters persist the same transition. HTTP and CLI expose that contract without raw work payloads.

**Tech Stack:** Existing Rust 1.99 core, SQLite/redb storage, HTTP adapter and CLI; current actor gates, retry epochs, supervision and maintenance archives.

**Spec:** [Authorized operator work recovery](../specs/2026-10-03-operator-recovery-design.md).

## Global constraints

- Resource remains the only application entity. Ledger controls are infrastructure operations, not freely mutable domain records.
- Preserve existing at-least-once delivery behavior by default; reconciliation hold is an explicit host/channel profile.
- Never reset consumed budgets, change frozen execution identity or infer compensation from a failure.
- Current operator authority and existing service/Resource authority remain separate checks.
- No application callback, verifier or network operation executes inside a native mutex/transaction.
- Keep defaults denied, views redacted, inputs bounded and old workers fenced.
- Keep all facade files free of implementation logic and tests.
- Stage 4.1 ownership must be integrated before claiming combined release readiness.
- This is a plan only. No builds, code changes or newly executed recovery tests accompany it.

## Review focus

1. A generation-preserving delivery/state update must invalidate operator CAS; task 1 tests revision increments on every transition.
2. Identical retry keys must survive lost acknowledgement without bypassing current authority; tasks 2 and 3 test replay and revocation.
3. A provider lookup miss must not masquerade as terminal rejection; task 4 tests Unresolved and late acceptance.
4. A hidden work record must not leak through counts, pagination, cursor fields or error text; task 3 tests projection and denial.
5. A well-formed wrong-target CLI response must not appear successful; task 5 extends existing response-validation and interruption fixtures.

## File boundaries

| Module | Responsibility |
| --- | --- |
| `crates/rom/src/operator.rs` facade and `operator/protocol.rs` | Public bounded request/view/result types and exports |
| `operator/authorization.rs`, `operator/runtime.rs`, `operator/projection.rs` | Host policy, supervised orchestration, redacted views |
| `operator/ledger.rs`, `operator/receipts.rs` | Pure transition/CAS validation and durable identity arbitration |
| `reaction_work.rs` and cohesive child modules | Record revision and profile-aware lifecycle; split unrelated behavior while preserving public exports |
| `storage_state/operator.rs`, `persistence/operator.rs` | Shared bounded state and adapter port contracts |
| SQLite/redb named operator modules | Native atomic integration, not duplicated domain policy |
| `rom-backup` model/upgrade/validation modules | Explicit format transition and preservation of new durable metadata |
| `rom-http/src/operator.rs` | Optional generic operator endpoints |
| `rom-cli/src/operator.rs` plus existing input/response/client modules | CLI command mapping and validated output |
| Dedicated core, native, HTTP and CLI operator tests | Conformance, races, fault injection and public consumer journeys |

Native table inventories and archive changes need one coordinated owner.
Do not introduce a separate SQL repository for each Resource or an unbounded audit database.

## Task 1: Versioned work state and bounded protocol

**Produces:** `WorkVersion { generation: String, revision: u64 }`, monotonic WorkRecord revision, opaque work handles and the public control/view types in the spec.

- [ ] Add RED unit tests covering every existing ledger update: accepted change increments revision, rejection leaves it unchanged, overflow rejects atomically.
- [ ] Add stale-version tests where claim generation stays unchanged but delivery or terminal state changes.
- [ ] Add protocol tests for unknown fields, oversized keys/evidence references, invalid bounds and opaque-handle collision rejection.
- [ ] Run those tests and record the expected failures before implementation.
- [ ] Implement revision updates in one shared ledger transition boundary; avoid incrementing twice during nested DeliveryFinished/Finish transitions.
- [ ] Define bounded response types and stable handle derivation without exposing hashes of secret payload values.
- [ ] Expose existing storage generation through the bounded operator port; do not create another independent incarnation scheme.
- [ ] Run focused core tests, serialization fixtures and Clippy. Review all mutation and restore paths for revision changes.

**Gate:** Any relevant work change invalidates an old view token, including changes that preserve claim generation.

## Task 2: Atomic control receipts and native persistence

**Produces:** Shared operator state transition and native storage methods for bounded snapshots/lookup and atomic control.
The control result distinguishes an accepted transition from replay of the exact existing operator receipt.
Use `Storage::work_snapshot(max_records: usize, max_bytes: usize) -> Result<StorageWorkSnapshot>` and `Storage::control_work(&StorageWorkControl) -> Result<WorkControlReceipt>`.
These public trusted adapter types are not remotely deserializable verification authority.

- [ ] Write RED cases for version mismatch, matching replay, changed request under the same key, wrong principal namespace, future/expired retry epochs and receipt capacity.
- [ ] Write RED cases for preserved payload/identity/budgets, blocked exhausted chains, active lease refusal and stale worker completion.
- [ ] Define private prepared-control types: expected WorkVersion, exact command fingerprint, operator identity/epoch, verified transition evidence and timestamp.
- [ ] Implement candidate-state validation and atomic receipt/state publication in `StorageState`; errors must leave both unchanged.
- [ ] Extend lifecycle headroom reservation for revision widths and test that full operator-receipt admission cannot prevent accepted worker completion.
- [ ] Keep raw adapter control methods unsupported by default. Add the same transition to both native transactions.
- [ ] Bump native/archive versions and add explicit prior-format upgrade. Assign initial work revisions and preserve at-least-once delivery semantics.
- [ ] Include operator receipts in archive bounds, restore and maintenance. Preserve them rather than silently pruning at capacity.
- [ ] Add shared native fault tests before write, after state write, before commit and after commit acknowledgement loss.
- [ ] Add subprocess exit/reopen tests with bounded child waits. Verify one accepted operator receipt and no duplicated transition.
- [ ] Run native operator tests plus relevant backup, upgrade, retention and worker conformance suites.

**Gate:** SQLite and redb persist identical CAS/idempotency semantics, and maintenance cannot discard operator evidence silently.

## Task 3: Authorized inspection and supervised retry

**Produces:** Runtime methods `operator_capabilities`, `work_list`, `work_read`, `work_control`, plus host `OperatorAuthorizer` registration.

- [ ] Add RED consumer tests for default denial, independent inspect/control permissions, current actor-gate revocation and denied replay of an old operator receipt.
- [ ] Add projection tests with sentinel secrets in frozen rows, action inputs, service keys and provider errors; none may reach output or error text.
- [ ] Add filtered moving-page tests proving authorization occurs before visible page limits and response bounds never truncate a serialized record.
- [ ] Add RED retry cases for Pending scheduling and eligible stopped work; assert unchanged payload, start time, attempts and root consumption.
- [ ] Add committed-but-unacknowledged Resource action resolution; event, receipt and effect counts must stay unchanged.
- [ ] Implement public methods through existing bounded supervised I/O and current authority checks; keep callbacks outside native locks.
- [ ] Implement exact operator receipt replay before CAS, with current authorization before disclosure.
- [ ] Reuse existing service and Resource authorization when the ordinary worker consumes newly due work; do not execute as the operator.
- [ ] Reuse the shared frozen-action receipt resolver rather than duplicate action replay semantics in the operator module.
- [ ] Run consumer/native authority races, cancellation and bounded-list tests.

**Gate:** A user can operate only currently authorized work, and operator retries preserve the original domain execution contract.

## Task 4: Explicit delivery profiles and verified reconciliation

**Produces:** Declared at-least-once, provider-deduplicated and reconcile-before-retry profiles; registered bounded verification; persisted hold state.

- [ ] Add compatibility tests proving the existing default still retries Unknown/TimedOut within budgets and can duplicate delivery without provider deduplication.
- [ ] Add RED hold-profile tests for timeout, restart after DeliveryStarted and expired original claim; no additional send may occur before resolution.
- [ ] Add verifier cases for Accepted, terminal NotAccepted, Unresolved, timeout, panic, stale work version and operator revocation while verification waits.
- [ ] Add an eventually consistent provider fixture where an initial lookup miss later becomes Accepted; the miss must remain Unresolved.
- [ ] Add legacy/current profile mismatch tests; startup or control must refuse reinterpretation of persisted work.
- [ ] Implement profile registration and persist the profile with work identity/version; preserve the compatibility default explicitly.
- [ ] Implement verifier supervision with finite timeout and bounded evidence identifiers. Never hold the core commit gate during verification.
- [ ] Recheck authority and WorkVersion after verification, then submit the atomic prepared control.
- [ ] Ensure Accepted completes without sending; terminal NotAccepted only permits retry within original budgets; Unresolved performs no transition.
- [ ] Run channel, native restart and reconciliation races on both adapters.

**Gate:** Each profile has its stated duplicate/reconciliation behavior. Adding operator tools does not silently redefine existing delivery guarantees.

## Task 5: HTTP, CLI and explicit compensation journey

**Produces:** Generic operator endpoints and `rom work` commands that call core operations.

- [ ] Add RED HTTP tests for verified actor use, unsupported capabilities, bounded request bodies and absence of raw ledger serialization.
- [ ] Add CLI tests for work list/show, exact retry/reconcile request files, invalid view tokens and capability denial.
- [ ] Add valid-JSON wrong-handle, wrong-operation-identity and wrong-protocol-version response fixtures; all must fail before success output.
- [ ] Add lost-response replay and Ctrl-C tests with blocked stdout/stderr using existing CLI harness helpers.
- [ ] Implement a cohesive HTTP operator module and thin router additions; keep domain and recovery policy in core.
- [ ] Implement CLI argument/input/client/response mapping without automatic retries or inline secrets.
- [ ] Extend the public reference journey: inspect failure, reconcile or retry where permitted, invoke its existing compensation action, then inspect resulting work and Resources.
- [ ] Verify compensation uses the same registered action/revision/idempotency path as any application action, with no generic rollback command.
- [ ] Run HTTP/CLI suites and the reference journey against both native adapters.

**Gate:** Operators can complete the documented recovery flow through generic public APIs without special Resource controllers.

## Task 6: Policy review and release evidence

- [ ] Document the host's authorizer and delivery profile choices. Do not invent an administrator role or tenant rule in the framework.
- [ ] Record two optional product decisions explicitly: whether to change the existing default delivery profile, and whether future reconciliation may accept human attestations.
- [ ] Keep both outside this slice unless the user chooses them; the compatibility default and registered verifier route can ship without those changes.
- [ ] Document bounded audit capacity, exact-request retry, hard budget stops and unresolved provider outcomes.
- [ ] Review native/archive format upgrade coverage, callback lock boundaries, current authority and response redaction independently.
- [ ] Update stage 4.2 OpenSpec scenarios with executed evidence only; do not mark the task complete from the plan.
- [ ] Run focused Clippy, public consumer checks and the final combined local verifier after ownership and recovery are integrated.
- [ ] Record commands, source revision, measured bounds and remaining support limits. Let the coordinator integrate; do not create an independent commit.

## Verification scheduling

Implementation gates use the existing `rom-dev` container and warm target after measurement work releases them.
Use `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`.
Core transition tests run before native integration; native tests run before HTTP/CLI acceptance.
The coordinator runs one final `./scripts/check` on the combined tree.
This design task ran none of those tests and changed no product code.
