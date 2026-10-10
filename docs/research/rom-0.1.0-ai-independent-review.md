# Independent optional AI and OpenRouter review

Date: 2026-10-07. Scope: current R13/R14 candidate and its public workflow boundaries.
This is a source and retained-evidence review. The reviewer ran no native build or provider request.

## Result

R13/R14 acceptance remains open. The review identifies three concrete defects and incomplete public acceptance seams.
The planned additive monotonic execution deadline is a proposal at this checkpoint, not verified behavior.
No clean-source integration, production-provider acceptance, or release admission is established.

The existing candidate preserves exact prepared attempts and original account reservations through uncertain recovery.
Current authority is checked before generation and explicit metadata lookup.
Known metadata cost can settle its original reservation without inventing completed output.
These safeguards do not complete typed tools, owner-visible preflight recovery, or unrelated public consumers.

## Source identity and evidence

The reviewer fenced 58 Rust and Cargo files under `crates/rom-ai` and `crates/rom-openrouter`.
Before and after inventories are `/var/tmp/rom-010-ai-independent-review-before.json` and `/var/tmp/rom-010-ai-independent-review-after.json`.
No file in this subset changed between those fences.
Every existing path in `/var/tmp/rom-ai-openrouter-task4-observation74-native33-source-hashes.txt` matched its recorded hash during the final comparison.
That worker inventory is broader than the 58-file reviewer fence; it is not a claim that every inventoried file was inspected.

The reviewer read `/var/tmp/rom-ai-openrouter-task4-observation74-native33-green-clippy.log`.
It records 14 reservation, 15 routing, and 45 flow passes: 74 AI cases.
It also records 8 catalog-flow, 2 loopback, 19 wire, 1 TLS, and 3 endpoint passes: 33 adapter cases.
These results were inspected, not independently executed by this reviewer.
They predate the new outer-supervisor regression and planned deadline correction.

The reviewer read `/var/tmp/rom-openrouter-task4-real-host-supervisor-red.log` and its actual two-database test source.
Both SQLite and redb cases failed the final durable generation-ID assertion after accepted headers and a stalled body.
The log records zero passes and two failures, with 18.16 seconds elapsed.
The source also checks original prepared identity, conservative accounting, restart, and one POST.
This failed execution is stronger evidence than a standalone adapter timeout test for the host cancellation boundary.

## P1: outer cancellation loses accepted generation evidence

Location: `/root/ROM/crates/rom-ai/src/flow/worker.rs:20`.
Related deadline construction: `/root/ROM/crates/rom-ai/src/flow/worker.rs:127`.
Related adapter: `/root/ROM/crates/rom-openrouter/src/transport.rs`.

The 18-second callback deadline starts before permit acquisition, Resource reads, and current-authority resolution.
The provider deadline is constructed later from tick time plus 18 seconds.
The outer timeout can therefore cancel `complete_observed()` after the adapter has received a valid generation header.
The adapter normally returns that header with a failed or timed-out body, but cancellation prevents that return.
The run keeps conservative unknown accounting while losing a usable generation lookup identity.

The two actual database regressions establish this defect in the reviewed source.
Minimal regression: accept one generation request; send a valid generation header; stall the body beyond callback expiry; reopen storage.
Retain that exact generation ID, the original attempt and reservation, no output, and one POST.
An explicit current-authority lookup must target the original observed generation without generation replay.

The approved design separates monotonic execution and observation deadlines from persistence grace.
It must include admission, authorization, credentials, discovery, transport, validation, and durable observation within the host budget.
Do not mutate the frozen attempt, reservation, policy, or original identity to implement this correction.
This report does not claim that the planned context methods exist or pass these regressions yet.

## P1: queued preflight failure has no owner-visible public recovery

Location: `/root/ROM/crates/rom-ai/src/flow/worker.rs:29`.
Related preflight paths: `/root/ROM/crates/rom-ai/src/flow/worker.rs:109` and `/root/ROM/crates/rom-ai/src/flow/worker.rs:133`.
Related public admission: `/root/ROM/crates/rom-ai/src/flow/operation.rs`.

Current-grant denial or discovery failure can occur before a prepared attempt and checkpoint exist.
The callback converts these errors to Permanent unless an existing settlement proves retryability.
No queued failure action records a public reason or a safe next intention.
The authorized owner can continue seeing Queued after Work stops.
Public `resume()` admits reconciliation only for an already started or held attempt; checkpoint-zero reconciliation is invalid.

This is the known open queued-step-zero seam, confirmed by source inspection.
It prevents useful durable progress and public pending recovery under AP-UX-007/008.
Raw operator Work control does not satisfy the consumer-facing requirement.

Minimal regression: submit a valid run; deny only its attempt grant or fail catalog discovery before prepare; process Work; reopen storage.
An inspection-authorized owner must see a bounded failure or waiting reason and the actual committed stage.
Provide a public, currently authorized recovery path when safe retry is supported.
It must retain the same run, original limits, budget authority, and receipt identity.
Unknown started effects must remain distinct and must not enter this preflight retry path.

No new native queued-failure regression was executed by this reviewer.

## P2: valid tool-call results discard observed identity and cost

Location: `/root/ROM/crates/rom-ai/src/flow/worker.rs:323`.
Related validation: `/root/ROM/crates/rom-ai/src/outcome.rs`.

The worker validates the completed outcome against the exact prepared attempt and its cost and token ceilings.
It then validates the completion structure.
For `Completion::ToolCalls`, it calls `hold_unknown()` without the completion's evidence or usage.
A valid observed generation ID and known in-ceiling cost are consequently replaced by empty observation and unknown accounting.
This loses a usable lookup identity and prevents exact settlement of already observed cost.

The existing `unsupported_tool_completion_is_held_without_executing_or_trusting_usage` test does not cover this valid branch.
Its adapter supplies input count 999 and cost 999 against a free attempt.
`DispatchOutcome::validate_for()` rejects that response before the ToolCalls branch.
The test correctly rejects untrusted out-of-ceiling evidence, but cannot prove retention of a valid tool result.

Minimal regression: return a valid requested tool call, exact attempt identity, valid generation ID, and known in-ceiling usage.
Hold the unsupported tool effect without executing it or publishing output.
Persist the validated observation against the original attempt, then settle only its exact reservation.
After restart, current-authority reconciliation must retain the original ID and issue no repeated POST.
Malformed, foreign-attempt, over-ceiling, and conflicting established evidence must remain rejected.

The reviewer traced the real validation and persistence branches, without a new native executable reproduction.
No Node substitute was presented as evidence for Rust runtime behavior.
The finding was sent directly to the AI implementation owner for a two-database failing regression.
Retaining valid financial observations does not establish that tools are supported or grant authority to execute them.

## Exact settlement and authority assessment

`committed_attempt()` checks both the run's active prepared attempt and its exact committed account entry.
`settle_record()` finds that same key and prepared payload, bounds actual cost, and preserves previously settled cost.
Unknown usage keeps the conservative reservation. Missing cost is not interpreted as zero.
`record_held_observation()` commits immutable attempt knowledge before financial settlement.
The public operation replay can repair already known settlement without repeating external lookup.

Generation and lookup use current host resolution and attempt authorization, not serialized owner strings or stored grant claims.
Authorized views establish the current actor and run inspection permission.
The view excludes prompts, credentials, raw tool arguments, and provider error bodies.
Completed output is hidden for cancelled or unresolved runs.
Financial facts about an already committed external request remain distinct from authority to perform another request.

These are inspected safeguards and scoped historical test evidence, not proof of all cancellation or revocation interleavings.
The reviewed deadline loss and valid-tool observation loss remain exceptions to durable evidence retention.

## Public usability and R13/R14 gaps

The public facade currently exposes submission, view, cancellation, and reconciliation.
Its builder installs Resources and existing channels and attaches Runtime before returning handles.
No competing scheduler was found in this reviewed composition.
Delayed confirmed-nonacceptance recovery uses committed existing Work intentions.

The planned typed `ToolRegistry`, restricted `ReadContext`, and frozen Resource-action wrappers are absent.
The worker deliberately holds every tool-call completion rather than executing an authorized typed tool.
Task 5 requires actual read/action boundaries, original receipt replay, finite call counters, current grants, and cancellation tests.
Native OpenRouter tool serialization is not evidence for this missing R14 executor.

The planned source-publication and unrelated triage consumers under `examples/ai-flows` are absent.
Task 6 requires public-interface recovery on SQLite and redb, including explicit partial multi-Resource publication.
Crate-internal development fixtures cannot establish those consumer seams or original application usability acceptance.

Pure `choose()` supports free then bounded-paid candidate selection with per-run continuation.
This alone does not establish complete user-facing failover across provider failures.
Local CPU fallback is expressly outside these two crates in the accepted plan and still needs application composition acceptance.
Do not relabel neutral-provider abstractions or held tool calls as completed local fallback or ergonomic agent flows.

Public progress projects committed milestones and bounded counters.
Discovery and authorization before prepare do not produce their own committed stage in this candidate.
The queued-failure seam therefore needs a designed public status and recovery contract, rather than generated narration or operator-only diagnostics.

## Integration limits

All inspected tests use offline simulated services or loopback transport; no live credentials or production provider calls were made.
No native build was launched by this reviewer while the AI worker held its build allocation.
The source fence applies to the recorded interval only. Subsequent implementation requires a new comparison and current evidence.
Full local verifier, clean packaged consumers, actual typed tools, original application acceptance, and production identity remain separate gates.
