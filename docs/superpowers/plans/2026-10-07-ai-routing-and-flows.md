# AI Routing and Flows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's authorized execution method; do not create another approval loop.

**Goal:** Add optional provider-neutral routing and Resource-backed flows with bounded typed tools and durable recovery.

**Architecture:** `rom-ai` owns neutral contracts and narrow flow composition. `rom-openrouter` supplies provider adaptation. Existing ROM channels and Work supervise ticks; no second scheduler is introduced.

**Tech Stack:** Existing Rust 1.99, Tokio, serde, serde_json, rayon, httpdate 1.0.3 and reqwest 0.13.5 with rustls. SQLite/redb are development fixtures only. No sha2 dependency was selected.

**Spec:** [AI flow design](../../../openspec/changes/add-provider-neutral-ai-flows/design.md), [routing requirements](../../../openspec/changes/add-provider-neutral-ai-flows/specs/ai-routing/spec.md), [flow requirements](../../../openspec/changes/add-provider-neutral-ai-flows/specs/durable-ai-flows/spec.md).

## Global Constraints

- Keep ROM core provider-neutral and driver-free. Keep public API paths stable.
- Use Rust 1.99 and existing dependency versions. Select no new third-party dependency without primary MSRV/license/advisory evidence.
- Prompt 32 KiB; output 64 KiB; registry 16 tools; response 8 tool calls; arguments/results 16 KiB each.
- Default policy: 8 generation attempts, 16 tool calls, 32 ticks, 300 seconds run age, zero paid authority.
- Ledger: 1024 entries and 256 KiB. Money: checked `UsdNanos(u64)` with `u128` intermediates.
- Host fixture: channel timeout 20 seconds, lease 30 seconds, at least two action and I/O admissions.
- No live provider calls, secrets, deployment or domain templates in offline acceptance.
- `lib.rs` and `mod.rs` are facades. Unit tests belong in separate named modules.
- Every task retains red failures, source/dirty/lock/compiler identity, commands and results. Commit only its owned files after review.

## Review Focus

- A duplicate delivery after result commit must resolve the checkpoint before dispatch; Task 3 covers this case.
- A successful run must not leave paid/candidate state for another run; Task 1 covers this case.
- A lost provider ID must keep uncertainty and cost reserved; Tasks 3/4 cover this case.
- Routine rate limits must resume automatically through persisted Work due, without consumer polling; Task 3 covers this case.
- Callback checkpoint actions must not starve behind their own Work permit; Task 3 covers this case.
- Cancellation or permission change between prepare and execution must not silently restart effects; Task 5 covers this case.

## Generic scheduling prerequisite

The root coordinator owns the generic delayed-channel seam and its separate conformance/migration work.
Before Task 3, require `Channel::intent_at(payload, not_before_unix_seconds)`, frozen optional scheduling data, and due-floor preservation through retry/operator/restore.
Require real SQLite/redb conformance and an enforced old-reader/native/archive compatibility fence.
The existing 100-millisecond Work idle wake handles due items; no AI timer task or second scheduler is allowed.
See the exact source map and acceptance list in the design. Task 3 must remain blocked on this prerequisite if it is unavailable.

## Files and ownership

Task 1 owns `crates/rom-ai/src/{request,response,error,provider,clock,telemetry}.rs` and `routing/`.
Task 2 owns `crates/rom-ai/src/flow/{resource,actions,codec,budget,reservation,checkpoint,view}.rs`.
Task 3 owns `crates/rom-ai/src/flow/{host,builder,worker,projection,client,operation}.rs` and its local fault fixture.
Task 4 owns `crates/rom-openrouter/`.
Task 5 owns `crates/rom-ai/src/tools/`.
Task 6 owns `examples/ai-flows/`, consumer admission fixtures and AI documentation.
One coordinator owns workspace manifests, lockfile, public facades and verifier/release integration after each task's interface is reviewed.
Do not edit shared facades concurrently. No edits to unrelated existing modules are planned.

### Task 1: Neutral contracts and pure route decisions

**Files:** Create the Task 1 module files, `crates/rom-ai/tests/routing.rs`, and crate manifest/facade through the coordinator.

**Interfaces:** Produce the design's `Provider`, `AiClock`, `OutputValidator`, `AiFuture`, `CompletionRequest`, `Completion`, `AttemptEvidence`, `Reconciliation`, `RoutingPolicy`, `RouteCursor`, `Deadline`, `AiError`, and `choose` signatures. Later tasks consume these types.

- [x] Add failing `independent_runs_keep_candidate_and_paid_state` assertions: rejecting model A in run one leaves run two at A and its paid authority false; serialization/reopen preserves run one's cursor.
- [x] Add failing `capability_prices_and_deadline_fail_closed` assertions: missing schema/tool/context capability, negative/overflow price and expired/backwards time produce their declared errors; no route is returned.
- [x] Run `cargo test -p rom-ai --test routing`. Record intended missing-contract failures.
- [x] Implement exact bounded types, strict codecs, defaults, checked money parsing and pure `choose` in the named modules. No global cursor, network or storage side effect.
- [x] Run the focused test again. All cases must pass.
- [x] Review public errors and Debug output with planted private text. Assert private text is absent in sanitized failures and telemetry.
- [x] Verify native assistant calls survive continuation, forged tool results fail, and reservation estimates use frozen rate ceilings rather than catalog quotes.
- [ ] Commit the reviewed Task 1 files and manifest/facade changes.

Task 1 verification: 13 focused routing tests and `cargo clippy --locked -p rom-ai --all-targets -- -D warnings` pass under Rust 1.99 with two jobs and incremental compilation disabled. Evidence is retained in `/var/tmp/rom-ai-task1-final-green-clippy.log`; intended missing-contract, overflow, JSON-depth, tool-continuation and reservation failures are retained separately. Integration review and the full verifier remain coordinator responsibilities.

The subsequent prepared-contract review added two behavioral regressions. They initially failed because external models and impossible decoded deadlines were accepted.
`PreparedAttempt::new` and public `PreparedAttempt::validate` now enforce policy/request/cursor/deadline coherence.
Forged tiers, indices, costs, rejected models and missing paid budget references fail. A changed request requires recomputed reservation cost.
The expanded 15-test suite and targeted Clippy pass; evidence is `/var/tmp/rom-ai-task1-prepared-green-clippy.log`.
The intended 13-pass, 2-fail result is `/var/tmp/rom-ai-task1-prepared-red.log`.
This checks neutral coherence only. Current time, catalog capability and Resource/budget authorization remain dispatch requirements.

### Task 2: Run actions and conservative reservations

**Files:** Create the Task 2 module files and `crates/rom-ai/tests/reservations.rs`.

**Interfaces:** Produce `AiRun`, `AiBudget`, `RunState`, `PreparedAttempt`, `RunView`, `OwnerIdentity`, and `UsdNanos` from the design. Pure `Action` definitions implement queued/prepared/executing/checkpoint/cancel transitions and reserve/settle actions. Channel payload is `FlowTick { run_id: String, sequence: u64 }` through the public `Input` contract.

- [x] Add failing `concurrent_reservations_authorize_one_dispatch` against SQLite and redb: capacity for one attempt admits one claimant; exact replay reserves once; changed replay returns identity mismatch.
- [x] Add failing `restart_retains_unknown_cost_and_limits`: prepared/executing states reopen with original identity, consumed count and reservation; no timeout releases money; policy changes cannot add paid authority.
- [x] Add failing `reservation_then_run_commit_is_conservative`: interrupt after account reservation; no request occurs; the reservation remains recoverable under its identity.
- [x] Run `cargo test -p rom-ai --test reservations`. Preserve the intended red results.
- [x] Implement canonical version-1 private storage codecs and pure revision-checked actions. Use immutable reservation entries and exact checked arithmetic. Reject ledger capacity overflow before dispatch.
- [x] Run the same tests. Compare stored receipts, events and work independently of returned messages.
- [ ] Commit Task 2 files after facade review.

Initial Task 2 implements `AiRun`, `AiBudget`, strict private codecs and reserve/prepare/start/hold/settle actions.
`ReservationEntry` stores the full immutable validated prepared request. A changed same-cost prompt fails identity matching.
Seven tests each execute against SQLite and redb: the three planned reservation journeys, free attempts without paid authority, late unknown evidence,
ledger byte limits and strict private codecs, and exact trusted settlement with frozen known usage.
They pass alongside all 15 routing tests and targeted Clippy in `/var/tmp/rom-ai-task2-final-green-clippy.log`.
The initial missing-contract RED is `/var/tmp/rom-ai-task2-contract-red.log`.
The later behavioral RED is `/var/tmp/rom-ai-task2-behavior-red.log`: three passed and two failed before free-authority and expired-evidence fixes.
`/var/tmp/rom-ai-task2-first-compile.log` retains a fixture timestamp mismatch; it is not product-failure evidence.

The expanded candidate supplies `RunView`, `RunHandle`, `RunMilestone`, `FlowTick`, `FLOW_TICKS`, checkpoint-result and cancel actions.
Fourteen tests run against both actual adapters, alongside 15 routing tests and targeted Clippy, in `/var/tmp/rom-ai-task2-checkpoint-final-green-clippy.log`.
The exact public `IdentityMismatch` regression and decoded/patch forged-policy amplification test preserve receipts, events and charges on rejection.
Checkpoint tests verify wrong-attempt rejection, current-authority projection, cancellation and late hidden output, atomic rounded delayed wake and replay, and no useful wake at expiry.
`HoldRun::with_evidence` preserves provider IDs. Explicit trusted `reconciled_not_accepted` resumes only the original held attempt, retains charge/counters and schedules through the existing channel.
Its intended missing-contract RED is `/var/tmp/rom-ai-task2-reconcile-red.log`; unresolved outcomes cannot enter ordinary waiting.
The candidate remains pending coordinator review and the full verifier. It has no worker/provider dispatch, tool registry or flow facade.
The coordinator must verify committed account entries before execution; serialized references do not authorize dispatch.
Finite 1 MiB runtime command admission permits JSON envelopes around private 256 KiB records without changing core defaults.
Current service revocation denies raw reads. Expired unknown evidence retains its original expiry and worst-case charge.

### Task 3: Existing Work composition and recovery

**Files:** Create the Task 3 module files and `crates/rom-ai/tests/work_flows.rs`.

**Interfaces:** Produce `FlowHost::new/install`, `FlowBuilder::build`, `FlowClient::submit/view/resume/cancel`, `Submission`, `RunHandle`, `FlowAuthority`, and the design's observer interface. Register one `ReconcileBeforeRetry` channel. Use an `Arc<Runtime>` owned by FlowClient and runtime-local weak callback attachment.

- [ ] Add failing `bind_is_local_and_has_no_runtime_cycle`: the consumed FlowBuilder attaches only its constructed Runtime; two hosts do not share Runtime state; dropping the client removes its weak attachment.
- [ ] Add failing `duplicate_tick_after_checkpoint_does_not_dispatch`: inject lost acknowledgement after checkpoint, replay/restart, assert one provider call and the stored next intent.
- [ ] Add failing `unknown_started_request_holds_work`: disconnect after executing commit; ordinary retry cannot send; verifier unresolved keeps run and reservation unchanged.
- [ ] Add failing `rate_limit_resumes_automatically_at_persisted_due`: confirmed 429 commits waiting plus exactly one delayed next intent; advancing the existing Work clock to due dispatches without FlowClient::resume; restart retains due and counters; no claim occurs before due; replay emits no duplicate intent.
- [ ] Add failing `restored_work_cannot_run_before_attachment`: reopen a fixture with already-due work; assert Builder::build performs zero callbacks; FlowBuilder attaches before returning handles; start_work then sees a live attachment and processes the original identity.
- [ ] Add failing `shutdown_or_dropped_client_prevents_unbound_dispatch`: drop the client or close Runtime before restored delivery; no provider request occurs; recorded failure/uncertainty remains conservative.
- [ ] Add failing `checkpoint_admission_and_time_budget_are_bounded`: one-capacity fixture returns bounded overload, supported two-capacity fixture completes; total deadline includes discovery/body/persistence; timeout is strictly below lease.
- [ ] Run `cargo test -p rom-ai --test work_flows`. Retain the intended failures.
- [ ] Implement one-tick callback over public Runtime methods. Reserve before dispatch, freeze attempt, persist execution start, validate result, commit checkpoint plus next intent, and reconcile original evidence. No polling loop.
- [ ] Persist counters for callback-originated roots. Do not rely on the previous root's Work budget to bound new callback-issued actions.
- [ ] Run the same tests on SQLite and redb. Assert original identity, current authority and bounded shutdown.
- [ ] Commit Task 3 files after a focused lifecycle/receipt review.

Task 3 is a frozen candidate awaiting coordinator lifecycle review and the full verifier.
`/var/tmp/rom-ai-task3-race-regression-green.log` records 30 flow, 14 reservation and 15 routing tests passing.
Every flow and reservation case uses actual SQLite and redb adapters. `/var/tmp/rom-ai-task3-race-clippy.log` passes with warnings denied.
The earlier 24-case candidate remains recorded in `/var/tmp/rom-ai-task3-regression-green.log`.
Earlier `/var/tmp/rom-ai-task3-recovery-first-green.log` records five shared SQLite/redb cases:
binding without auto-start or a Runtime cycle; unknown started effects held through expired unresolved lookup;
automatic confirmed-rate-limit resume at the persisted due time; current principal revocation; and database reopen with marker replay/forgery checks.
The public flow uses a creation-only reaction followed by a committed private run admission marker and channel tick.
The initial no-op ledger proposal failed ROM's no-op effect invariant on actual adapters; that evidence remains in `/var/tmp/rom-ai-task3-noop-diagnostic2.log`.
The corrected initial journey passes alongside all 14 reservation and 15 routing tests in `/var/tmp/rom-ai-task3-admission-first-green.log`.
`/var/tmp/rom-ai-task3-operation-red.log` retains the earlier cancel replay and repeated lookup failures, corrected by immutable operation admission.
Additional retained intended failures cover late cancellation, expired body results, known-cost settlement, lookup authority deadlines,
settlement interruption recovery, actual stage observations, single-slot overload, invalid output and restored executing recovery.
The expanded cases cover bounded lookup admission, cancelled Pending replay, competing nonces, queued owner revocation and revoked attempt grants.
`/var/tmp/rom-ai-task3-race-malformed-red.log` retains six intended race/response failures and one separate capacity-contention fixture failure.
The corrected worker rechecks exact durable attempt/account proof and uses current revision with one bounded CAS retry.
Unknown results never repeat generation. Malformed, foreign-attempt and unsupported-tool results retain internally generated evidence and reject supplied usage.
The queued owner fixture permits bounded overload during contention, then verifies denial on original-key replay after the operations drain.
Separate settlement fault cases distinguish confirmed noncommit from acknowledgement loss after actual commit.
Known settlement recovery reuses durable evidence and exact account proof; it makes no new generation call or duplicate charge.
Manual lookup setup shares two seconds; total recovery and callback processing each have an 18-second bound.
The implemented interface adds current trusted `FlowAuthority::resolve/attempt` and explicit bounded `FlowBuilder::limits`.
Tools remain outside Task 3. Explicit model rejection still needs a trusted nonacceptance classification before cursor advancement.
These are generic candidate tests. AP-UX-007/008 progress and queue/resume, AP-UX-015 bounded research/source navigation,
AP-UX-023/024 retained context and fresh execution, AP-UX-027 routing/retry policy and AP-UX-029 consumer output quality retain application acceptance requirements.
Domain interpretation, prompts, grounding sources and answer quality remain consumer-owned.

### Task 4: Bounded OpenRouter adapter

Current source review and concrete negative cases: [OpenRouter contracts](../../research/rom-0.1.0-openrouter-contracts.md).
The coordinator admitted the declaration-only crate and existing locked dependencies through offline metadata.
Fourteen adapter tests now pass: eleven real loopback HTTP cases and three wire admission cases. Targeted Clippy exits zero.
Global catalog, exact prices, native schema/tool codecs and metadata lookup are implemented candidates.
Endpoint discovery, cache isolation, broader negatives, content recovery and complete acceptance remain pending.
The reviewed compatible `Provider::complete_observed` and `DispatchOutcome` seam retains bounded uncertain evidence.
Its worker and late held-evidence enrichment cases pass 34 flow, 14 reservation and 15 routing tests, plus targeted Clippy.
Evidence and earlier intended RED are linked in the research document. These results do not close Task 4 or original-consumer acceptance.

The coordinator independently reran all 63 cases against the frozen AI source inventory.
Both pre-test and post-test hashes matched `/var/tmp/rom-ai-task4-late-observed-source-hashes.txt`.
Evidence: `container:/var/tmp/rom-010-root-ai-task4-late-observed-review.log`.
The review checked exact attempt binding, current authority, account proof, and the restricted held-evidence transition.
This result does not prove the OpenRouter implementation or packaged consumer acceptance.

The expanded adapter run executed 11 loopback tests: five passed and six failed for intended missing behavior.
The failures cover catalog rates and bounds, schema encoding, native tools, exact numeric cost, and generation lookup.
Evidence: `container:/var/tmp/rom-openrouter-expanded-behavior-red.log`.
Cargo stopped before the three wire tests. Do not report all 14 tests as executed in this run.
The revised GREEN executed all fourteen cases, followed by targeted Clippy with warnings denied.
Evidence: `/var/tmp/rom-openrouter-expanded-final-green-clippy.log`; source inventory: `/var/tmp/rom-openrouter-initial14-source-hashes.txt`.

**Files:** Create the design's `crates/rom-openrouter/src/` modules, manifest and facade; add `tests/loopback.rs` and `tests/wire_contract.rs`.

**Interfaces:** Produce `OpenRouterConfig`, `CredentialSource::resolve`, `SecretToken`, `OpenRouter::new`, and `impl Provider for OpenRouter`. Consume Task 1 prepared request/result/error contracts.

**Preflight placement review remains open.** Current locations below refer to the candidate `flow/worker.rs` before the demand-aware catalog patch.

| Current point | Durable state and authority | Interruption or proposed change |
| --- | --- | --- |
| Lines 133–138: `Provider::catalog` | Queued or due Waiting; current owner resolved; shared provider permit held | Approved `catalog_for(request, policy, deadline)` filters endpoint eligibility before route selection. No budget grant comes from catalog data. |
| Lines 191–198: construct exact `PreparedAttempt` and `ReserveBudget` | Exact request, policy, route, attempt ID and deadline exist; account charge has not committed | Proposed preflight could examine this exact identity. Placement is not approved. |
| Lines 199–205: current attempt authorization | Trusted authority checks full prepared request and budget reference | Any preflight must retain this check and recheck current authorization before reservation. |
| Lines 210–222: reserve account and verify exact entry | Account ceiling commits; run can still remain Queued | Interruption retains the full reservation. Recovery cannot invent a new dispatch identity. |
| Lines 223–245: prepare run | Run becomes Prepared and records a genuine stage | Account and run are separate commits. Existing prepared recovery must keep its exact reservation and request. |
| Lines 246–269: authorize, verify account and start | Current authorization and exact committed reservation precede Started evidence | Interruption after START_RUN is uncertain; generation must not restart blindly. |
| Lines 270–285: authorize and call provider | Run is Executing; provider call receives the exact prepared request | Current top-level provider errors enter HOLD. Definite pre-POST refusal needs separate reviewed handling. |

One absolute deadline covers catalog discovery, endpoint checks, persistence, permit wait and generation.
Configured endpoint discovery retains all bounded identities, with at most four concurrent refreshes. It does not require repeated caller invocations.
Final wire `max_price`, provider whitelist and `require_parameters` remain mandatory after catalog eligibility filtering.
A preflight-before-reservation design also needs a durable queued-failure contract and explicit replay rules.
The current queued checkpoint cannot simply become a failed prepared checkpoint. No such state or settlement change is implemented.
Add concrete interruption and refusal RED cases before changing that ordering. Preserve the known-refusal versus unknown-dispatch distinction.

- [ ] Before manifest edits, record reqwest 0.13.5 features, versioned upstream MSRV/license and current advisory graph. Reuse rustls only, no defaults, no SDK/schema/decimal library. Preserve complete license notices through existing gates.
- [ ] Add failing `catalog_delay_is_inside_deadline_and_endpoint_refreshes_are_independent`: one slow endpoint cannot lock another; expired deadline causes zero dispatch; bounds reject catalog >4 MiB or >4096 models.
- [ ] Add failing `schema_preservation_and_capability_enforcement`: schemas without selections remain unchanged; unknown schema requirements are unsupported; local semantic validation remains decisive.
- [ ] Add failing `typed_http_and_body_errors`: 200 error envelope, 400 request error, 401/403 denial, 402 credit failure, 429 bounded Retry-After, 503 unavailable, truncated/oversized output preserve their categories.
- [ ] Add failing `native_tools_and_lost_id_reconciliation`: tool-call finish reason maps to typed calls with IDs; continuation retains tools/results; missing generation ID and lookup miss return unresolved; no blind retry.
- [ ] Run `cargo test -p rom-openrouter --test loopback --test wire_contract`. Preserve the intended failures.
- [ ] Implement host-only HTTPS origin/key references, no redirects/proxy, `retry(reqwest::retry::never())`, deadline-bound transport and chunk limits. Use serde encoding directly. Parse exact decimal prices/cost and never expose raw error text.
- [ ] Run the same tests with local fake HTTP services and deterministic response fixtures. No live credentials.
- [ ] Commit Task 4 files after the coordinator reviews graph changes.

### Task 5: Typed reads, actions and cancellation

**Files:** Create `tools/{registry,read,action}.rs` and `crates/rom-ai/tests/tools.rs`.

**Interfaces:** Produce `ToolRegistry::new/read/action`, `ReadContext::read/query`, and `PreparedToolAction` exactly as the design declares. Consume FlowAuthority, flow checkpoint actions and Provider tool-call results.

- [ ] Add failing `read_context_cannot_execute_actions` external compile fixture: read context offers authorized read/query methods and has no Runtime mutation handle.
- [ ] Add failing `tool_schema_target_and_current_grant_are_enforced`: forged name/principal, malformed typed input, wrong kind, oversized input/result and revoked grants produce no mutation or success event.
- [ ] Add failing `prepared_action_replays_original_receipt`: lose acknowledgement, reopen database, replay expected revision/input/key/epoch, assert one Resource commit and event bundle; changed input fails identity matching.
- [ ] Add failing `cancel_before_and_after_dispatch`: queued cancellation gives zero dispatch; executing cancellation keeps cancel_requested, late evidence and reservation; uncertain outcome does not become confirmed failure.
- [ ] Add failing `serialized_calls_and_limits_are_finite`: duplicate call IDs, >8 calls per response, >16 calls per run or >32 ticks stop boundedly; tool results retain IDs and order.
- [ ] Run `cargo test -p rom-ai --test tools` and the new compile fixture through existing compile admission tooling. Retain intended failures.
- [ ] Implement typed registry wrappers with restricted read context, trusted current authority and frozen Resource commands. Execute one call per tick. Keep arbitrary HTTP/shell effects unsupported.
- [ ] Run the same tests on both adapters and public compile consumers. Assert redacted views after permission changes.
- [ ] Commit Task 5 files after authority and cancellation review.

### Task 6: Two public consumers and integration gate

**Files:** Create `examples/ai-flows/{Cargo.toml,src/lib.rs,src/publication.rs,src/triage.rs,tests/recovery.rs,README.md}`. Coordinator integrates package admission/verifier paths. Add producer docs with declared support limits.

**Interfaces:** Consumers use public `FlowHost`, `FlowClient`, ToolRegistry, Resources/actions and existing operator APIs. No private module imports, vendor aliases or schedulers.

- [ ] Add failing publication journey: prepare immutable revision then update head using frozen identities. Interrupt before/after each commit and acknowledgement. Assert explicit intermediate state, historical citation preservation and no duplicate publication.
- [ ] Add failing unrelated task-triage journey: read authorized tasks, propose validated classification and invoke one registered task action. Assert denial, restart, budget exhaustion and cancellation through the same facade.
- [ ] Run `cargo test -p rom-ai-flows-consumer`. Keep intended initial failures.
- [ ] Implement consumers without domain prompt templates inside rom-ai. Document exact declaration → bind → submit → observe → recover workflow and private field boundaries.
- [ ] Run both consumer journeys on SQLite and redb with deterministic provider fixtures. Compare receipts, events, reservations and Work states.
- [ ] Run `cargo fmt --all --check`, targeted Clippy/docs/MSRV commands from `scripts/check-rust`, affected package/compile admission and `./scripts/check` before integration. Record exact source and lock identity.
- [ ] Independently review supported versus proposed behavior and dependency direction. Do not broaden tests after passing unless new changes or unresolved risks justify it.
- [ ] Commit the reviewed consumer/docs/integration files. Leave this change incomplete if any required acceptance fails.

### Astral Plane acceptance traceability

The canonical intake is `docs/research/rom-0.1.0-usability-intake.json`.
Neutral routing tests do not establish these consumer expectations. Each acceptance below remains pending.

| Intake | AI subsystem acceptance and boundary | Acceptance owner |
| --- | --- | --- |
| AP-UX-007/008 | A committed submission appears as queued work. Reload preserves identity and actual queued/running/waiting/completed stages, attempts and retry time. No fabricated percentage. | Tasks 3 and 6; frontend stage presenter belongs to Studio. |
| AP-UX-013 | Every resumed action and result view checks current authority. Permission changes redact private context. Transport freshness never substitutes for authorization. | Tasks 3, 5 and 6; stale/visibility-aware frontend cache belongs to Studio. |
| AP-UX-014/015 | Consumer supplies domain prompts, topic policy and validated grounding. Concrete research gaps use explicit bounded work identities. Ordinary conversation stays usable while work runs. | Task 6 demonstrates the generic contract; consumer answer quality and topic policy remain application acceptance. |
| AP-UX-016 | Persist only evidence of tools actually executed. Authorized detail views retain call/result identity and omit private arguments. | Tasks 5 and 6; domain labels remain application-owned. |
| AP-UX-022/023 | Reopening saved results preserves immutable references and invokes no provider. History selection and source navigation preserve pending-work identity. | Task 6 verifies persistence/reference behavior; history layout, focus and safe link controls belong to Studio. |
| AP-UX-024 | Submit with a new identity creates fresh work. Exact replay or observing an existing identity returns retained work without recomputation. | Tasks 2, 3 and 6. |
| AP-UX-026 | Private observations are ordinary authorized Resources. Runs cannot invent permission to read or mutate them; revoked access removes private views. | Tasks 5 and 6; profile semantics and edit/delete policies remain consumer-owned. |
| AP-UX-027 | Free then bounded paid selection retains run-local cursor, deadlines and reservations. Provider changes preserve the same pending identity. | Tasks 1–4 and 6. Local CPU fallback is not implemented by these two crates; application composition needs separate acceptance. |
| AP-UX-029 | Scheduled daily work and explicit fresh execution use durable identities. Relationship and calculation settings remain consumer inputs and cache policy. | Generic delayed Work prerequisite and Task 6; daily scheduling, domain freshness and relationship answers remain consumer acceptance. |

Uncovered by neutral routing: frontend queue/history/navigation usability, useful grounded domain answers, private profile editing, daily freshness policy, and local CPU fallback.
The coordinator must keep these expectations assigned to Studio or consumer acceptance. Do not close them from Task 1 results.

## Plan self-review and handoff

The six tasks cover every added routing/flow requirement. Tasks 1–5 define the exact interfaces used by Task 6.
Dependency adoption remains conditional on graph review; initial code uses versions already in the workspace.
Finite offline tests do not prove live provider output quality, deployment recovery or human author usability.
The owner can review this design/plan while the authorized execution proceeds through the parent coordinator.
No code/build/dependency/deployment work was performed while writing this plan.

## Task 4 catalog and reconciliation candidate checkpoint

The current focused run passed 70 AI cases: 41 flow, 14 reservation and 15 routing tests.
The earlier 63-case inventory and coordinator review remain preserved as historical evidence.
Four new actual-database tests first failed pending reconciliation completion after settlement interruption.
Both SQLite and redb exercise failure before settlement commit and a lost acknowledgement after commit.
An authorized retry repairs only exact-attempt persisted known cost through the existing settlement contract.
It finishes the original Pending operation as Unresolved and returns a fresh authorized view.
Unknown-cost replay performs no lookup. Recovery creates no reservation, generation, or output publication.

The adapter run passed 26 cases: 15 loopback, three wire, two endpoint, five catalog/flow, and one TLS case.
Demand-aware discovery derives exact candidate IDs from the frozen policy when no host subset is configured.
An explicit host subset narrows discovery. Raw cache records are filtered again for each request and provider whitelist.
Actual HTTP flows over both databases reject paid-only or incapable endpoints and choose the eligible alternate model.
The TLS fixture verifies rejection of an untrusted signed certificate before HTTP credentials reach the server.
This does not establish production TLS, live provider or original-consumer acceptance.

Executed logs are `/var/tmp/rom-ai-task4-reconciliation-regressions-green.log` and `/var/tmp/rom-openrouter-catalog-initial-green.log`.
Four intended failures remain in `/var/tmp/rom-ai-task4-pending-settlement-red.log`.
Targeted Clippy exited zero in `/var/tmp/rom-ai-openrouter-task4-catalog-clippy.log`.
Source inventory: `/var/tmp/rom-ai-openrouter-task4-catalog-reconciliation-source-hashes.txt`.
The native lease was released after these terminal results.

Adapter-native observed metadata, optional content recovery, concurrency coverage above four configured IDs and further wire negatives remain pending.
Preflight lifecycle ordering remains under review; no new preflight transition or settlement semantics were implemented.
Task 5 typed tool execution and Task 6 external public consumers remain required.
AP-UX-007/008, 023/024 and 027 map to these generic recovery and routing cases.
Their original consumer reproduction, content requirements and acceptance remain open.

### Exact settlement selection correction

Both actual databases reproduced a pending replay selecting the successor after the original authorization proof.
The final regression pauses after `Runtime::establish_actor` returns, outside the backend authorization-read context.
Trusted public Resource actions reserve and prepare a successor while the original replay is paused.
The original reservation then remained Unknown instead of settling its persisted cost of three nano-USD.
The intended RED is `/var/tmp/rom-ai-task4-exact-settlement-selection-intended-red.log`.
Earlier authorization/storage blocking, fixture CAS failures and a masked characterization run remain preserved separately.
They do not count as intended product failures.

The repair passes the already authorized frozen RunRecord to the existing settlement helper.
It cannot select a newly active attempt, reservation, account, or cost.
Both regressions settle only the original entry and leave the successor Reserved at its exact ceiling.
They retain one generation and one lookup; the concurrent successor preparation issues no provider call.
The complete focused AI run passed 72 cases: 43 flow, 14 reservation and 15 routing tests.
Targeted Clippy exited zero in `/var/tmp/rom-ai-task4-exact-selection72-green-clippy.log`.
Frozen AI inventory: `/var/tmp/rom-ai-task4-exact-selection72-source-hashes.txt`.
This supersedes the earlier candidate source inventory without deleting its evidence.

The adapter's six-candidate discovery characterization passes with measured two-to-four concurrent refreshes in one catalog call.
Its separate observed metadata/cache RED executed 19 loopback cases: 16 passed and three failed for intended behavior.
Six catalog cases and two endpoint cases passed before Cargo stopped at the loopback failures.
TLS and wire targets did not execute in that RED run.
Native metadata enrichment and explicit response-cache disable are now source candidates awaiting their GREEN run.
Two additional native metadata flow tests over actual databases are also pending execution.

### Owner-visible queued failure and deadline requirements

AP-UX-007/008/027 remain incomplete when discovery or current budget authority fails before PREPARE.
Current source can stop channel Work permanently while the run still projects Queued without a failure.
The public reconcile API admits started or held attempts; it does not remedy this queued failure.
Operator inspection is available, but it does not establish an ergonomic owner-facing diagnosis or safe permitted retry.
The owner must receive an actionable failure and a safe retry route after current authority and policy checks.
Retry must preserve durable identity and receipts, or explicitly create a fresh run when the user requests fresh execution.
It must not fake HTTP429, paid permission, nonacceptance, or a weaker schema/tool policy.
A narrow queued-step0 failure/retry contract requires design review and actual negative recovery tests before implementation.

The frozen worker caps the prepared expiry at tick clock-now plus 18 seconds, below the immutable run expiry.
It computes that cap after initial reads and authority resolution.
The outer callback deadline starts earlier and uses a monotonic clock, so it can still cancel generation after accepted headers.
No explicit persistence grace currently separates provider completion from the outer supervisor.
The earlier description of the adapter receiving the full 300-second run expiry was incorrect for this worker.

New source-only supervisor tests use real HTTP headers, a stalled body, and both actual databases.
A bounded authority delay makes the existing deadline mismatch deterministic.
They retain the exact prepared attempt/account and verify conservative reservation, restart and absence of a second POST.
They require the known generation ID to survive the outer callback cancellation.
Both actual database cases failed for the intended missing generation ID after 18.16 seconds.
Evidence: `/var/tmp/rom-openrouter-task4-real-host-supervisor-red.log`.
The exact prepared attempt, reserved ceiling, one POST and restart assertions passed before the final identity assertion failed.
No deadline-context product change was made for this RED.

Proposed ordering: create the monotonic callback budget before initial reads; reserve finite time for persistence.
Pass a compatible opaque execution deadline to generation, separate from frozen attempt identity and policy expiry.
The real adapter must return observed header knowledge before the supervisor expires, then commit HOLD and conservative settlement.
Current owner, complete prepared/account proof, finite shared permits and no blind regeneration remain mandatory.
Manual metadata/content lookup must share its original two-second authorization/queue/HTTP budget.
The coordinator approved the additive deadline context after this RED. Queued failure semantics remain a separate review requirement.

### Approved execution deadline correction

Add opaque `ExecutionDeadline` with a finite monotonic end, redacted Debug and no Deserialize implementation.
Its constructor accepts positive durations up to 18 seconds. Cloning preserves the same end; shortening cannot extend it.
Compatible Provider defaults add `complete_observed_within` and `reconcile_observed_within`.
They reject an expired context before invoking an existing provider method.
The context supplies execution time only. It cannot authorize a provider call or change durable attempt identity.

Start the shared callback deadline before initial reads, authority and provider-permit acquisition.
Reserve two seconds of the original 18-second callback budget for persistence.
The worker bounds generation at the resulting 16-second end.
The OpenRouter adapter returns observed knowledge 250 milliseconds before that execution end.
Catalog discovery, authorization and permit waits consume the same original budget.
Keep the existing prepared UTC expiry, request, policy, reservation and idempotency key unchanged by this context.

Manual reconciliation shares its original two-second lookup budget across authorization, permit acquisition and HTTP.
Its adapter observation margin is 250 milliseconds; persistence remains within the existing 18-second operation total.
Persist exact-attempt knowledge before settlement, preserving current owner and budget authority checks on every CAS retry.
Do not turn cancellation, incomplete content, or a depleted execution context into proof of nonacceptance.
Legacy providers that ignore the new context cannot guarantee header recovery.
Process or storage stalls beyond the persistence grace remain unknown outcomes; this is not a real-time guarantee.

First execute the missing-API RED in `crates/rom-ai/tests/execution.rs` before adding its production module and defaults.
Then reproduce both supervisor cases with the adapter context and retain the original failure log.
Add exhausted-context zero-POST/GET, shared catalog/permit delay, and manual lookup deadline cases.
Repeat the prior 74 AI and 33 adapter cases, targeted Clippy, and the coordinator's full local verifier before integration.

### Executed execution deadline candidate

The additive missing-API RED failed only the absent deadline type and compatible provider methods.
All four neutral deadline contract cases subsequently passed.
Real HTTP execution RED passed zero-POST/GET exhaustion and failed generation and lookup budget reuse as intended.
The adapter overrides now return observed knowledge before the shared execution end without modifying the frozen prepared attempt.
Both actual supervisor cases passed in 15.94 seconds with retained header identity, conservative charge, restart and one POST.

Independent source review found a valid ToolCalls branch discarding known usage and generation identity.
Two declared-tool, paid-ceiling database cases first failed only the missing generation ID.
The corrected branch persists validated knowledge through the existing hold and exact settlement path.
It performs no tool execution or output publication; current revoked budget authority still blocks lookup after restart.
The malformed oversized/free tool fixture remains unchanged and tests a different rejection path.

The complete focused run passed 80 AI cases and 38 adapter cases, plus targeted all-target Clippy.
Evidence: `/var/tmp/rom-ai-openrouter-task4-execution80-native38-green-clippy.log`.
Frozen source inventory: `/var/tmp/rom-ai-openrouter-task4-execution80-native38-source-hashes.txt`.
The coordinator owns the full verifier and independent review. No release or original-consumer acceptance is claimed.
Dedicated catalog/permit shared-budget coverage, optional content recovery and owner-facing queued failure/retry remain open.
Tasks 5 and 6 remain mandatory; retained ToolCalls knowledge does not implement ergonomic tool execution.

### Next preparation failure and explicit fresh retry seam

Four existing-interface database tests required owner-visible failure after discovery ineligibility or current budget denial.
They first failed Queued versus Failed, then passed after the preparation action and coordinator correction.
The separate missing-action API RED failed only the absent public action/type imports.
Add `flow/preparation.rs`, `FailPreparation` and `FAIL_PREPARATION`; the coordinator owns facade exports and registration.
The pure action admits only an admitted Queued run at checkpoint zero, with no active attempt or consumed generation.
It records a sanitized failure and Failed state without altering request, policy, owner, counters, evidence, expiry or account money.
The coordinator must refresh current owner authority and the exact Resource revision before each bounded CAS retry.
Only failures before ReserveBudget enter this path; started or uncertain generation retains its existing held-state contract.

An account reservation can commit before the Run prepare commit.
Absence of an active attempt does not prove an empty account or authorize a release.
Add an orphan-reservation negative case that preserves the complete original entry, ceiling, evidence and charge.
Do not invent provider nonacceptance, an HTTP429, a zero-cost settlement or a weaker paid grant.

The fresh retry recipe uses the ordinary public submission boundary:

1. Read the current authorized RunView and show its sanitized failure.
2. Retain the original run identity and history for observation or receipt replay.
3. After an explicit fresh-run request, create a new run ID and a new idempotency key.
4. Build Submission from currently authorized application inputs and the selected bounded routing policy.
5. Call FlowClient::submit and observe the returned RunHandle through the ordinary authorized view.

Reusing the original submission identity returns its retained failed run; it must not recalculate.
Current eligibility and budget authority are checked before the new generation. A fresh run cannot release the old account hold.
Both database journeys must reopen, replay the original identity without generation, then complete only the explicit new identity after permission or eligibility changes.
The application owns source freshness and domain prompts; the recipe cannot infer new private context from a serialized RunView.

The complete preparation candidate passed 85 AI cases and 38 adapter cases, plus both crates' all-target Clippy.
Evidence: `/var/tmp/rom-ai-openrouter-task4-preparation85-native38-corrected-green-clippy.log`.
Frozen inventory: `/var/tmp/rom-ai-openrouter-task4-preparation85-native38-source-hashes.txt`.
The orphan test processes mapper materialization and its admission action as separate existing Work steps before reservation.
Its earlier one-step admission Conflict remains preserved as a fixture timing failure.
The reviewed legacy grant assertion now expects Failed and Denied, retaining its original zero-lookup/generation/account assertions.
Capacity, missing-account and post-Prepare denial diagnosis remain separate coverage; full-verifier and consumer acceptance remain open.

### Task 5 execution ledger

The coordinator reported full verifier session 37302 terminal exit zero for the preparation candidate.
Task 5 source changes intentionally supersede that frozen candidate. They have not passed a new full verifier.

The declaration API first failed with missing ReadContext and ToolRegistry exports.
Two declaration cases then passed, covering finite registry entries, aliases, typed native schema and redacted Debug.
Evidence: `/var/tmp/rom-ai-task5-registry-missing-api-red.log` and `/var/tmp/rom-ai-task5-registry-initial-green.log`.

The public tool-host seam failed with only missing tool_actor, action and tools methods.
Evidence: `/var/tmp/rom-ai-task5-tools-host-corrected-missing-contract-red.log`.
The PreparedToolAction root export separately failed with E0432.
Evidence: `/var/tmp/rom-ai-task5-prepared-tool-missing-export-red.log`.

Five maintained tests then ran: two passed and three failed for intended behavior.
Both actual database journeys remained AwaitingReconciliation instead of completing typed reads, an action and native continuation.
The decoded action contract also accepted an oversized actor identity.
Evidence: `/var/tmp/rom-ai-task5-tools-corrected-initial-behavior-red.log`.
Earlier fixture failures retain their separate logs: an incorrect derive attribute and a missing explicit field policy.
The identity fix delegates to the original OwnerIdentity validator and bounds the complete PreparedToolAction payload.
The identity regression passed in `/var/tmp/rom-ai-task5-decoded-action-green.log`. No persisted identity becomes an authority grant.

Accepted continuation invariants:

- Keep the original request, policy, owner and expiry immutable.
- Persist a bounded native transcript separately. Derive each effective generation request from completed earlier turns.
- Keep native call IDs unique across the complete run. Preserve call and result order.
- Freeze registry version, alias, exact input, target, revision, action identity and retry epoch before mutation.
- Re-establish current trusted authority and match the persisted actor before invocation, replay and result disclosure.
- Replay the original Runtime receipt without a current target revision precheck. Do not regenerate the command.
- Complete known provider cost settlement before tool dispatch. Keep unknown cost charged conservatively.
- Execute one tool per durable tick through the existing Work scheduler. Bound all calls, ticks and transcript bytes.
- Derive the active generation source checkpoint exactly from the persisted batch. Do not replace equality with an arbitrary lower-bound check.
- Retain pending uncertain mutations across cancellation and restart. Do not fabricate stopped success or compensate automatically.

The standalone `tests/ai-tools-compile` fixture uses only public read/query/actor interfaces.
Its negative binary requires exactly E0599 for execute at source line four.
The coordinator admitted its standalone pinned lock offline. The positive and negative compile checks passed in `/var/tmp/rom-ai-task5-external-read-context-green.log`.
Typed runtime integration, adversarial database cases and the two unrelated external consumers remain required.
No Task 5 or original-consumer acceptance is claimed.


Task 5 registry freeze and continuation source checkpoint (2026-10-08):

The registry freeze first failed on both actual adapters because the durable record omitted its admitted version.
Evidence: `/var/tmp/rom-ai-task5-registry-freeze-red.log` (zero passed, two failed).
The compatible default-empty field now freezes the trusted installed version at submission.
Authorized original receipt replay retains that version after host replacement; an explicit fresh identity captures the current version.
Both adapters also reject a forged version replacement without journal or Work changes.
Evidence: `/var/tmp/rom-ai-task5-registry-freeze-green.log` and `/var/tmp/rom-ai-task5-continuation-compatibility.log`.

The continuation source now has private transcript and checkpoint modules plus a tool coordinator.
Each turn references its original ReservationKey; its full PreparedAttempt remains bound in the existing account ledger.
The Resource validator derives source checkpoints exactly from completed earlier calls.
The transition validator reconstructs each permitted delta and compares the complete record against that reconstruction.
Read and action actors are persisted before tool invocation and checked again before result publication.
These source checks do not establish complete runtime acceptance.

The existing 85 AI cases passed against this partial continuation source.
Evidence: `/var/tmp/rom-ai-task5-continuation-compatibility.log` (four execution, fourteen reservations, fifteen routing, fifty-two flow cases).
The complete seven-test tools target still has five passes and two failed database journeys.
Those journeys remain Executing while root-owned private action registration is pending.
Evidence: `/var/tmp/rom-ai-task5-first-continuation-check.log`.
There is no Task 5 Clippy, full-verifier, consumer or release acceptance at this checkpoint.

Remaining requirements include original action receipt recovery, concurrent cancellation, revocation, finite overload, lost acknowledgement and transcript forgery regressions.
A pending uncertain mutation must remain held across cancellation; it cannot be reported as stopped success.
The current client reconciliation path must route tool uncertainty to the original action receipt, never to another provider POST.
Action route validation needs an additive public Action name accessor to avoid current codec decoding before receipt replay.
Real provider continuation also needs fresh eligible rates with the existing selected model, tier and rejected-model cursor preserved.
The existing catalog identity guard must not be weakened by resetting routing state.
Native argument escaping and complete effective wire bytes remain bounded; a definite preflight refusal must stay distinct from uncertain dispatch.
The two unrelated public consumers remain required after these runtime seams pass.

AP-UX-007/008 trace durable tool stages and actionable recovery; AP-UX-013 traces current authorization and private result disclosure.
AP-UX-015 traces bounded authorized grounding; AP-UX-023/024 trace retained context and explicit fresh identity versus receipt replay.
AP-UX-027 traces immutable routing and conservative cost settlement; application prompts and interpretation remain consumer-owned.
These implementation mappings do not close original-consumer acceptance.

The install-time action route accessor has an intended standalone E0599 failure at action_route.rs line three.
Evidence: `/var/tmp/rom-ai-task5-action-route-missing-api-red.log`.
The standalone fixture lock is unchanged (`a3ce26c0fa838f1b67846bd76b582683adf4206861fbaa7e2920c52e829f61f3`).
Root Cargo.lock remains `f6ac31a111d67e8c4d1cc8bc0438ac0cf02bd87e851c78c86d6e2756b1241bf3` for this check.
No Action accessor or root-owned registration was changed by the AI worker.

Task 5 durable recovery and current-grant checkpoint (2026-10-08):

The tools target passed 27 maintained cases on actual SQLite and redb adapters.
Evidence: `/var/tmp/rom-ai-task5-tools27-corrected-journal-green.log`.
The original 85 AI cases, all-target Clippy, public read-only compile checks and public Action route checks also passed.
Evidence: `/var/tmp/rom-ai-task5-tools112-compatibility-clippy-public-fixture.log`.
These 112 cases supersede the earlier partial runtime checkpoint. They do not establish Task 5 completion.

The cases cover original action receipt recovery after acknowledgement loss, restart, cancellation with uncertain effects, concurrent resume and immutable transcript forgeries.
Current tool grants are checked before successor generation and retained output disclosure.
An actual Resource row grant change denies retained output while the owner and tool role remain valid.
Completed JSON null uses a private result envelope. It remains distinct from a missing result across persistence and native continuation.
The null behavior first failed on both adapters: `/var/tmp/rom-ai-task5-nullable-result-red.log`.
Fixture timing, compile and journal-oracle failures remain in their original separate logs.

The native public authoring path installs a versioned ToolRegistry on FlowHost.
Typed reads receive ReadContext with authorized read/query methods. Typed mutations register existing public Action values.
Submission freezes registry version and requested descriptors. An explicit fresh run captures the current installed version.
FlowAuthority::tool_actor must check current domain/data grants through AuthorizationRead, in addition to matching the persisted actor.
The external compile fixture proves that ReadContext does not expose execute(). It does not prove both required external consumer journeys.

Two new maintained tests are staged for globally repeated model call IDs in a second generation.
They require rejected tool admission to retain valid generation ID and known usage, settle the exact original account entry, and perform no further effects.
These tests have not run. The current source inventory includes them and therefore is not the exact previously tested 112-case snapshot:
`/var/tmp/rom-ai-task5-tools112-with-repeated-id-red-source-hashes.txt`.
No product correction is justified until the behavioral RED is recorded.

Remaining scope includes real OpenRouter continuation with changing catalog snapshots, exact native wire-byte bounds, positive paid tool settlement interruption and finite failure paths.
The selected route, tier and rejected-model cursor must survive fresh endpoint discovery without resetting routing state.
The two unrelated external public consumer flows remain required. Current adapter tests and the full verifier must rerun after accepted source changes.

AP-UX-007/008 map to durable stages, actionable failure and safe receipt recovery.
AP-UX-013 maps to current tool and Resource-row grants before private result disclosure.
AP-UX-015 maps to bounded authorized grounding; AP-UX-023/024 map to retained context and explicit fresh execution versus receipt replay.
AP-UX-027 maps to routing state and conservative exact-attempt settlement.
Application-specific research selection, prompts and interpretation remain consumer-owned.
Each mapping still needs original-consumer acceptance; internal test success is insufficient.

Task 5 rejected repeated-call plan checkpoint (2026-10-08):

The staged repeated-ID tests ran on both actual adapters and failed as intended.
Both runs remained Executing after rejected second-generation admission, instead of retaining valid provider knowledge in a held state.
Evidence: container `/var/tmp/rom-ai-task5-repeated-native-id-red.log` (zero passed, two failed).
The worker now rereads the exact committed account/run proof after admission failure.
If that same attempt remains Executing, CancelRequested or AwaitingReconciliation, the existing observed HOLD path preserves validated evidence and usage.
Each bounded observed-HOLD CAS iteration rechecks current attempt authorization.
A committed ToolsPending admission remains intact after acknowledgement or settlement failure; this path does not erase its batch.

The complete 29-case tools target and all-target rom-ai Clippy passed.
Evidence: container `/var/tmp/rom-ai-task5-repeated-native-id-tools29-green.log`.
The two new cases prove generation ID retention, exact original zero-cost settlement, no third provider POST/lookup and no further action mutation.
They do not prove positive paid-cost interruption or a real OpenRouter continuation.
The previous 85 compatibility cases and adapter cases have not rerun after this narrow correction.
The current source inventory is `/var/tmp/rom-ai-task5-tools29-repeated-id-green-source-hashes.txt`.
Task 5/6, current full verification and original-consumer acceptance remain open.

Task 5 actual OpenRouter native continuation checkpoint (2026-10-08):

Two real HTTP/database journeys first failed because fresh catalog identity prevented the completed tool batch from continuing.
Evidence: container `/var/tmp/rom-ai-task5-real-openrouter-native-continuation-red.log` (zero passed, two failed).
The new neutral `RouteDecision::for_request(policy, previous, next)` first produced intended E0599 failures.
Evidence: container `/var/tmp/rom-ai-task5-continuation-route-api-red.log`.
It validates the original decision against the exact prior request, then recomputes only the frozen policy ceiling for the bounded effective request.
Model, tier, full catalog-session cursor and rejected IDs remain unchanged.
Forged original cost, substituted prior request and changed policy version fail the focused routing case.

The worker checks current selected-model capability, endpoint eligibility and prices in a fresh focused catalog before constructing the new reservation.
That check uses fresh metadata independently of the frozen cursor. It cannot grant budget authority or reset selection.
Both actual adapters now complete two real OpenRouter POSTs with ordered native calls/results and retained descriptors.
Both also fail visibly without a successor POST when the endpoint cannot accommodate enlarged authorized tool context.
The unschematized fixture originally rejected valid text because its output validator assumed an object.
That separate fixture failure remains in `/var/tmp/rom-ai-task5-native-route-green.log`.
The consumer validator now explicitly decodes and validates its JSON text.
Corrected focused evidence: `/var/tmp/rom-ai-task5-native-route-corrected-fixture-green.log` (four passed).

The current complete AI targets passed 115 cases: execution four, reservations fourteen, routing sixteen, tools twenty-nine and flows fifty-two.
The current adapter targets passed 42 cases, including four native continuation journeys and one local TLS case.
Both packages passed all-target Clippy with test-support enabled.
Evidence: container `/var/tmp/rom-ai-task5-native-tools115-adapter42-regressions-clippy.log`.
Source inventory: `/var/tmp/rom-ai-task5-tools115-native42-green-source-hashes.txt`.
Its SHA-256 is `03bd4bbbda43b1dac8dcf37b1993cd3539b409251adb363a3823d17f430d2820`.
Native lease was released after the terminal result. No full current verifier, release or original-consumer acceptance is claimed.

Remaining Task 5 requirements include exact escaped native wire bounds, positive paid settlement interruption, additional finite failure cases and review.
Both unrelated external public consumer flows remain Task 6 requirements.
AP-UX-007/008 and AP-UX-027 map to actual owner-visible failure, retained progress and frozen routing across native continuation.
AP-UX-013/015/023/024 retain their current-authority, bounded grounding, source context and fresh-execution acceptance requirements.

Task 5 deterministic native preflight checkpoint (2026-10-08):

The actual SQLite/redb escaped-argument cases first failed as intended: the known encoder refusal became AwaitingReconciliation after accepting a successor generation.
Evidence: container `/var/tmp/rom-ai-task5-escaped-native-preflight-red.log` (zero passed, two failed).
The additive Provider::preflight default validates only the exact PreparedAttempt.
The OpenRouter override performs bounded wire encoding without network I/O.
The worker applies the same remaining execution deadline and current grant checks before and after preflight, before reserving or starting a new attempt.
It does not widen any persisted ceiling, change the original request or manufacture provider nonacceptance evidence.
The final dispatch encoder remains authoritative.

Both cases now report owner-visible Failed/InvalidRequest, retain one original reservation and one generation, and perform no successor POST.
Focused evidence: container `/var/tmp/rom-ai-task5-escaped-native-preflight-green.log`.
The current 115 AI cases, 44 adapter cases and both all-target Clippy checks passed.
Evidence: `/var/tmp/rom-ai-task5-preflight115-native44-regressions-clippy.log`.
Frozen inventory: `/var/tmp/rom-ai-task5-preflight115-native44-green-source-hashes.txt`.
Its SHA-256 is `624e2e2d85916c2cfe3852e881a0db106da47af90f74230ba3288f46826b7309`.
A legacy default cannot certify protocol framing. Already started or uncertain restored work is not regenerated by this seam.
Existing Waiting failures retain their prior identity and account knowledge; this increment does not change their state semantics.

Task 6 initial standalone API checkpoint (2026-10-08):

The coordinator generated the standalone consumer lock offline with existing package versions.
The corrected initial compile attempt failed only on intended missing publication/triage modules.
Evidence: `/var/tmp/rom-ai-task6-public-consumers-corrected-api-red.log`.
The earlier attempt also lacked the required Provider::reconcile fixture method; that separate setup failure remains in its original log.
Domain implementation now exists as source-only work. Its declaration and complete tool journey tests have not passed yet.
The package implements a custom Classification Field through public interfaces, immutable captured editions, separate head publication and unrelated ticket classification.
No private AI module is imported. Source capture and semantic interpretation remain application responsibilities.

Current original-consumer feedback reports commit `86dca6f` and update marker `1791445000`.
This is consumer-reported/source evidence, not installed package or release acceptance.
Both external flows retain AP-UX-007/008/013/014/015/016/023/024/027 requirements.
Positive paid interruption, additional actionable tool-step failures, supervised calculation and consumer restart/cancellation remain required.
The coordinator owns the generic Runtime calculation seam so CPU work remains tracked through shutdown.
ReadContext delegation remains held until that seam has its own RED/GREEN evidence.

Task 5 supervised restricted CPU wrapper checkpoint (2026-10-08):

The coordinator implemented generic Runtime::calculate through supervised I/O and the exact existing Rayon pool.
The wrapper initially failed with intended E0599 only: `/var/tmp/rom-ai-task5-context-calculation-api-red.log`.
ReadContext::calculate now delegates through that API under its original execution deadline, matching the read/query result contract.
It creates no pool, semaphore or scheduler. It exposes no Runtime handle or write interface.
The trusted closure must finish joined child work before returning and must not block on nested Runtime calls.
Cancellation stops waiting; the Runtime admission and shutdown tracking remain owned until the CPU closure finishes.

Four wrapper tests each use both actual adapters.
They verify deadline and cancellation permit retention, shutdown drain, expired zero-submission and sanitized nonterminal panic behavior.
Focused evidence: `/var/tmp/rom-ai-task5-context-calculation-green.log`.
The complete current AI suite passed 119 cases, including these four unit cases; all-target Clippy passed.
Evidence: `/var/tmp/rom-ai-task5-context119-regressions-clippy.log`.
The adapter source did not change after its 44-case evidence. That target was not rerun for this wrapper-only increment.
Inventory: `/var/tmp/rom-ai-task5-context119-native44-source-hashes.txt`, SHA-256 `45cfc1c492b7ef818b5c1f884c0d3bcc3404a3bc316645ad87885e8a621e4f80`.
Current-tool-grant-before-publication still requires an actual flow regression with calculation.
These privileged wrapper fixtures do not establish serialized context authority or production sandboxing.

Task 6 initial public consumer GREEN checkpoint (2026-10-08):

Five public declaration/custom-codec cases and four complete public flow journeys passed in the standalone package.
The publication flow reads a current source, prepares a captured immutable edition, then publishes its head as a separate action.
The unrelated ticket flow reads an authorized ticket and commits its validated classification once.
Both run on actual SQLite/redb adapters without private AI imports or a second scheduler.
Evidence: `/var/tmp/rom-ai-task6-public-consumers-initial-green.log`.
All-target standalone Clippy passed: `/var/tmp/rom-ai-task6-public-consumers-initial-clippy.log`.
The standalone lock SHA-256 is `1742d2f3cdfa7b0601bad9d308c481045fac5a582de8a9023a62031ae8f224b7`.
The coordinator added its checks to the local Rust gate. No current combined full-verifier result is claimed here.
Consumer acknowledgement loss, restart, current grants, cancellation and budget cases remain required before Task 6 acceptance.

Task 5 current-grant and positive-cost tool recovery checkpoint (2026-10-08):

Two real-flow calculation tests revoke the tool grant while supervised CPU work runs.
Both databases retain ToolsPending, no result, zero completed tool calls and no successor generation.
Evidence: `/var/tmp/rom-ai-task5-calculation-current-grant.log`.
The current grant prevents private-result publication; an actionable failed-read projection remains required.

Four paid-tool tests inject known settlement noncommit or lost acknowledgement after the underlying commit.
The original generation ID and cost3 persist before tool I/O. The immutable reserved ceiling is10.
After reopening each database, the original reservation settles once before the application action and successor generation.
The two generations cost6 in total. Duplicate processing adds no charge, reservation or application action.
Evidence: `/var/tmp/rom-ai-task5-paid-tools-settlement-first.log`.
All35 tool integration cases and all-target Clippy passed: `/var/tmp/rom-ai-task5-tools35-paid-regressions.log`.
Two additional paid-grant revocation cases each exercise both fault positions; the exact account remains unchanged after restart.
No application action, new reservation or generation occurs while the current attempt grant is denied.
Evidence: `/var/tmp/rom-ai-task5-paid-grant-recovery-first.log`.
These checks characterize existing behavior. No behavioral RED or product correction is claimed for these additions.

Task 6 expanded public recovery investigation (2026-10-08):

The maintained external consumer suite adds lost acknowledgement, restart, current grants, cancellation and generation-limit cases.
Its first run passed4 and failed8: `/var/tmp/rom-ai-task6-external-recovery-first.log`.
Six failures use an incorrect journal-head oracle: unrelated AI facts also advance the global inspected-history position.
The corrected fixture compares bounded same-kind events and retains the original evidence.
Two generation-limit failures leave ToolsPending instead of actionable Failed after the committed domain stage.
Source inspection finds successor reservation before PREPARE_RUN rejects the exhausted immutable generation limit.
The strengthened regression requires one original reservation, no successor POST and sanitized BudgetExhausted.
A proposed early admission check remains subject to the corrected RED and coordinator review.

AP-UX-007/008 map to durable progress, interruption and actionable failure. AP-UX-013 maps to current private-data grants.
AP-UX-015/016 map to bounded actual tools and original receipt recovery, without application-specific research prompts.
AP-UX-023/024 map to immutable source publication and explicit fresh run identity. AP-UX-027 maps to routing and paid bounds.
Original consumer commit86dca6f remains source-reported evidence, not installed-release acceptance.

Task 5/6 actionable admission and public recovery GREEN checkpoint (2026-10-08):

The corrected external RED passed10 and failed2 only on a second orphan reservation despite generation limit1.
Evidence: `/var/tmp/rom-ai-task6-external-recovery-corrected-oracle-red.log`.
After current transcript authorization, plan_attempt now checks the immutable generation limit before discovery or reservation.
Existing pure failure checkpointing retains prior domain commits, evidence and charges while publishing sanitized BudgetExhausted.
No new reservation or successor generation is admitted.

Two definite read-tool failures initially remained ToolsPending: `/var/tmp/rom-ai-task5-definite-read-failure-red.log`.
The coordinator approved a positive whitelist: Denied, InvalidOutput, InvalidRequest, UnsupportedCapability and BudgetExhausted.
Only the first unresolved Read with no prepared invocation or unknown marker can fail through this path.
Fresh exact Prepared/account proof and current owner/attempt grants precede every bounded CAS attempt.
The existing clone-equality transition permits only state, failure and derived observation milestone changes.
Prior calls, actor identities, results, evidence, costs, counters and reservations remain immutable.
ProviderUnavailable, UnknownOutcome, Storage, Closed and DeadlineExceeded retain pending state in actual two-database tests.
Prepared mutating-action uncertainty retains its original recovery behavior. It cannot become a fabricated failure or cancellation success.

The complete focused checkpoint passed131 AI cases,3 default adapter wire cases and17 external consumer cases.
Production targets, AI fixtures and the standalone consumer passed all-target Clippy with warnings denied.
The adapter test-support feature was omitted; its HTTP/TLS fixtures and feature-enabled Clippy require a new run.
Evidence: `/var/tmp/rom-ai-task5-ai131-adapter44-external17-regressions.log`.
The external12 flow cases include original receipt recovery, reopen, current tool-grant denial, cancellation and finite generation budgets.
Every added recovery scenario exercises both publication and ticket-triage domains on the named database.
Publication tests also edit the current Draft and retain the original immutable Edition citation.
The five declaration/custom-Field cases remain included.

AP-UX-007/008 now have actionable read and exhausted-generation failure regressions in addition to recovery.
AP-UX-013/015/016 retain private-grant and actual-tool evidence. AP-UX-023/024 retain immutable citation and fresh-versus-replay recipes.
AP-UX-027 retains exact paid settlement and no-charge amplification tests.
Independent review, combined local verification, packaged installed consumers and original-application acknowledgement remain open.

Executed-count correction (2026-10-08): the command in `/var/tmp/rom-ai-task5-ai131-adapter44-external17-regressions.log` omitted adapter test-support.
It executed131 AI cases and17 external cases, but only3 adapter wire cases; gated HTTP/TLS targets ran zero cases.
Its filename overstates current adapter coverage. The previous44 adapter result remains historical evidence.
A test-support-enabled rerun is required before assigning44 cases to this new checkpoint.

Feature-corrected adapter checkpoint (2026-10-08):

The explicit `--features test-support` rerun executed44 adapter cases and passed matching all-target Clippy.
Counts are8 catalog,2 endpoint,3 execution,19 loopback,6 native-tool,2 supervisor,1 TLS and3 wire cases.
Evidence: `/var/tmp/rom-ai-task5-adapter-feature-corrected-regressions.log`.
AI131 and external17 evidence remains the preceding unchanged-source run.
The earlier default3 adapter log and its incorrect filename remain preserved with the scope correction above.
The coordinator added explicit feature-enabled adapter testing and Clippy to the mandatory Rust verifier.
No combined full-verifier result or original-consumer acceptance is claimed by these focused checks.

### 2026-10-08 restricted read recovery remains in progress

The completed candidate has 131 AI cases, 44 feature-enabled adapter cases and 17 external consumer cases. Root verifier65912 passed that candidate. These results do not cover the new public read recovery requirement.

Six maintained SQLite/redb read-resume cases first failed with `Conflict`: reopen, physical CPU ownership after timeout, and concurrent duplicate admission. The physical controls rejected a second Runtime while CPU work remained owned. They did not authorize takeover from a timeout alone. The concurrent fixture initially overflowed its stack; that setup failure remains separate from its corrected behavioral RED.

The proposed implementation adds persisted read ordinals and phases. A bounded weak lease registry retains physical ownership through synchronous calculation descendants. Existing operation stamps and Runtime receipts remain authoritative. Read recovery emits existing channel work through a pure checkpoint action. Original source, request, account, generation and action identities remain frozen.

The first implementation compiled but overflowed the default test stack before acceptance assertions. The bounded probe measured public resume at 27,848 bytes, the recovery fixture at 120,088 bytes, and `Runtime.process_work` at 96 bytes. These type sizes alone do not establish the overflow cause. Later fixture traces reached the started read callback. The grant-refusal experiment also overflowed before its expected public failure assertion. No stack limit was increased. No GREEN claim applies to this implementation.

Pre-start delivery recovery, queued deadlines, current grant refusal, expiry, duplicate replay and acknowledgement loss remain required. The read contract is trusted and side-effect-free; it is not a sandbox. Mutations require typed Actions. Calculation children must finish before their closure returns. Unknown Action recovery and cancellation retain their original receipt semantics.

AP-UX-007/008 require actionable public recovery. AP-UX-013/015/016 require current private grants and bounded research/tools. AP-UX-023/024/027 preserve source, run and payment identity. Domain prompts and original Astral Plane acceptance remain separate.

### 2026-10-08 read recovery boundary checks

Narrow allocations at the public resume and read-recovery seams removed the observed default-stack overflow. The measured public future is now176 bytes. The fixture future is28,624 bytes. No thread stack limit changed. SQLite physical CPU ownership, timeout, shutdown and takeover controls passed. Both adapters passed ten public reopen, duplicate, owner-change, private-grant refusal and expiry cases. Full regression verification remains pending.

Two queued pre-start cases passed after shutdown and reopening the actual databases. Concurrent original and recovery deliveries invoked the original read once. These cases do not establish recovery after a native delivery was already recorded as started.

Four admission fault cases passed on SQLite/redb. They distinguish confirmed noncommit from a lost acknowledgement after commit. The reopened caller reused its original operation key and expected revision. The tests observed no extra action, provider lookup, account change or read callback.

The separate native DeliveryStarted fault commits Unknown before invoking the callback. Both adapters then retained the exact Scheduled read attempt. Public resume failed with Conflict after reopening. Both adapters also reproduced retry-budget exhaustion returning BudgetExhausted without an actionable Failed view.

The approved correction adds an explicit bounded Wake for a current Scheduled read. It preserves the callback ordinal, actor and original operation key. It appends one Finished(Unresolved) reconciliation admission and one tick with an atomic existing-channel intention. A wake is an authorized admission request; it does not classify the original Work outcome. A physical callback lease prevents concurrent invocation. Stale revision, changed operation identity, active physical ownership and exhausted frozen limits remain rejection conditions.

This changes the former sequential different-key Scheduled rejection. A new explicit key at the current revision may request a bounded wake. Exact same-key replay returns the current authorized projection without another tick or intention. Concurrent deliveries must still produce one physical callback. The old rejection log remains historical evidence.

The first correction run passed18 of20 public cases. The two remaining failures assert that former Scheduled rejection. The coordinator approved updating those cases to test preserved transcript, attempt, original operation, account, callback ordinal and eventual single invocation. Corrected tests are staged; no complete GREEN result is claimed. Result-checkpoint acknowledgement loss, queued deadlines, all baseline regressions and full verification remain required.

AP-UX-007/008 map to actionable failed views and explicit durable recovery. AP-UX-013/015/016 map to current authorization and finite read retries. AP-UX-023/024/027 retain exact context, fresh-versus-replay identity and charges. These checks do not establish original Astral Plane application acceptance.

Focused read recovery checkpoint:20 public cases now pass. The complete64 tool cases pass. The other90 AI cases pass:4 unit,4 execution,14 reservation,16 routing and52 work-flow cases. Thus154 AI cases passed across the affected runs. The adapter explicitly enabled test-support and passed44 cases. Both external consumers passed17 cases. Positive public compile fixtures passed; the negative ReadContext.execute fixture produced only its intended E0599. Matching AI, adapter and external all-target Clippy passed.

The external locked test initially stopped before compilation because the core HMAC addition required its standalone lock refresh. The coordinator refreshed only the two approved standalone locks. Existing registry package identities and checksums remained unchanged. The subsequent locked/offline test passed. This setup failure remains preserved separately from behavioral RED.

Evidence files are `/var/tmp/rom-ai-task5-read-resume-public-wake-contract-green.log`, `/var/tmp/rom-ai-task5-read-resume-tools64-regressions.log`, `/var/tmp/rom-ai-task5-read-resume-ai90-affected.log`, `/var/tmp/rom-ai-task5-read-resume-adapter44-regressions.log`, `/var/tmp/rom-ai-task5-read-resume-external17-lock-corrected.log` and `/var/tmp/rom-ai-task5-read-resume-public-compile-fixture.log`. These results do not cover the pending result-checkpoint and queued-deadline cases or constitute a new complete verifier result.

Six additional read-recovery cases passed on the actual adapters. Four inject failure before read-result commit or acknowledgement loss after commit. Confirmed noncommit retained Started ordinal2 and no result. A new authorized pure-read retry recovered it. Lost acknowledgement retained Completed ordinal2 and its result, so recovery did not repeat the read. The original settled account entry remained exact; the domain Action committed once. Two authority-wait cases exhausted the shared2s resume budget without read admission, callback, account change or failure publication. After the owned authority job drained, the original key resumed successfully. The first compile failed on test-only fixture fields/accessor names; that setup failure is preserved. No product correction was needed for these six cases.

The previous154/44/17 inventory remains historical. The new six cases bring the tool target to70 maintained cases. A full70-case rerun and complete verifier are still pending.

The next proposed owner projection exposes a bounded read status and ordinal. The four states are Queued, Active, AwaitingRecovery and ActivityUnknown. It includes no call ID, alias, argument or result. Pure projection cannot observe physical callback ownership. Started, Unresolved and legacy reads therefore report ActivityUnknown there. Host projection may report Active only while a real physical lease survives. Host projection must check current owner, registry and transcript/tool grants before disclosure. This snapshot is advisory; resume rechecks current revision, authority and physical ownership. Completed and non-read runs expose no read progress. API RED and actual two-adapter/public-consumer acceptance remain required before this proposal becomes implemented behavior.

### 2026-10-08 read progress and complete affected checkpoint

The new public contract passed intended missing-API RED: E0432 for ReadProgress/ReadStatus and E0599 for read_progress. The pure projection then passed its finite wire contract. Four real-database cases failed as intended: the host still reported ActivityUnknown after reopening and disclosed progress after a referenced row grant was revoked.

The corrected host checks current owner, registry, settled transcript proof and pending tool grants under one2s deadline. It annotates physical Active only from the existing bounded weak lease. No callback, provider request, operator grant or scheduler is added. The pure projection retains ActivityUnknown for Started, Unresolved and legacy reads.

The first corrected run passed3 of5 progress cases. Two fixtures incorrectly expected a successful patch response after the committed patch revoked outcome disclosure. The fixture now expects Denied and checks the actual revision2/read_allowed=false row through bounded AuthorizationRead. No permission or product policy changed. A separate incidental fixture-edit compile failure remains preserved.

Five focused progress cases now pass. Both physical CPU cases also pass: host Active survives the caller timeout, pure projection stays ActivityUnknown, and recovery remains blocked until the physical lease drains. Owner, tool-role and referenced-row grant denial are tested. Reopened reads report AwaitingRecovery. Queued retry/wake keeps the ordinal. A stale snapshot cannot authorize another admission. Completed and non-read stages expose no progress.

The complete current rom-ai suite passed165 cases:4 unit,4 execution,14 reservation,16 routing,75 tools and52 work-flow cases. The feature-enabled OpenRouter suite passed44 cases. External consumers passed17 cases; both publication and triage now observe Queued ordinal1 through only public owner APIs. AI/adapter and external all-target Clippy passed. Public compile fixtures passed their positive APIs and exact E0599 negative. Logs use the `/var/tmp/rom-ai-task5-read-progress-` prefix.

Source body and tests are frozen at this checkpoint. A new combined full verifier, independent review and installed AI consumer/original Astral Plane acceptance remain separate gates. The original reported consumer commit86dca6f is source/application evidence; it does not adopt or approve this AI candidate. AP-UX-007/008 map to actual durable progress and recovery;013/014/015/016 to bounded private tools and research;023/024 to retained source versus fresh run;027 to frozen routing and money. Application prompts, answer quality and local-model fallback remain consumer-owned acceptance.
