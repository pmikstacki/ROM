# ROM 0.1.0 release coordination

Date: 2026-10-07. Status: active goal; investigation in progress. This is not release acceptance.

## Owner-authorized scope

Complete the [production-consumer research plan](rom-0.1.0-consumer-research-plan.md) and the accepted generic Resource contract.
Investigate source before research, selection, and implementation. Include reusable consumer usability improvements.
Add optional smart OpenRouter routing and ergonomic agent flows through existing Resource, action, event, and durable work contracts.
The core must remain independent of model providers, transports, and database drivers.

## Consumer coordination

Consumer: `/root/astral-plane`.
The [consumer thread](thread://01a111a5-617a-7800-b94f-eb82b11da39d?hostId=remote-ssh-codex-managed%3A7455bafd-bcdf-4924-b743-87f13bd5b771) was read on 2026-10-07.
The read covered the latest turn and three older pages. The last response reported `hasMore=false`.
Pagination completion does not guarantee that provider-limited messages are untruncated.
Private extracts retain turn/message identities and page metadata. Do not publish raw personal messages or credentials.

There is no callable cross-thread message-delivery tool in this session.
A shared-file handoff is placed at `/root/astral-plane/docs/rom-0.1.0-coordination.md`.
The existing [consumer feedback](astral-plane-production-feedback-2026-10-07.md) remains unchanged.
No acknowledgement from the original consumer agent is claimed.
No deployed consumer, vendor source, provider settings, or live data has been changed by this coordination.

## Feedback intake contract

Each usability observation receives a stable ID, source turn/message reference, and classification.
Classifications: shared ROM defect, public-interface friction, documentation/discoverability gap, application-specific behavior, or unresolved evidence.
Record duplicates, superseded observations, withdrawn requests, and conflicts instead of silently discarding them.
A domain-specific preference does not automatically become a core feature.

Each accepted shared item must identify its maintained source location, proposed generic contract, regression, and external-consumer acceptance.
Track states: collected, investigated, researched, selected, implemented, independently reviewed, verified, deferred, or rejected with reason.
A passing isolated fixture does not close an item requiring deployed or packaged-consumer acceptance.

## Maintained investigation and intake

The [usability intake](rom-0.1.0-usability-intake.json) records 30 stable groups from the returned conversation and inspected source.
The [usability investigation](rom-0.1.0-usability-investigation.md) maps each group to a generic seam and acceptance scenario.
The [core investigation](rom-0.1.0-core-investigation.md) identifies production composition and recovery gaps.
The [AI investigation](rom-0.1.0-ai-investigation.md) maps routing and agent flows to existing durable work.

These are source investigations, not executed acceptance tests. Existing consumer test reports remain reported evidence.
Framework candidates require selection and implementation. Application-specific requests remain visible in the intake.
The withdrawn frame-rate request requires no ROM implementation.

## Investigation ownership

| Investigation | Exclusive report ownership | Source access |
| --- | --- | --- |
| Consumer usability and shared Studio behavior | `/var/tmp/rom-010-ui-investigation.md`, `/var/tmp/rom-010-usability-intake.json` | Read-only ROM/consumer source and thread extracts |
| AI routing and agent flows | `/var/tmp/rom-010-ai-investigation.md` | Read-only consumer execution and existing ROM work contracts |
| Production core and other-project lessons | `/var/tmp/rom-010-core-investigation.md` | Read-only core, conformance, deployment, and bounded project comparisons |
| Coordination, research plan, release specification | Maintained ROM documents assigned to the root agent | Integrate reports without modifying their evidence |

No investigation agent owns product code, live data, shared build targets, or deployment configuration.
Implementation file ownership will be assigned after the interfaces and investigation results are reconciled.

## Additional AI research scope

Investigate a provider-neutral invocation contract and a separate OpenRouter adapter.
Separate model eligibility, provider routing, fallback order, and an aggregate run budget.
The deadline must cover discovery, waiting, each attempt, validation, and fallback.
Retries must retain run identity and progress rather than restart a failed catalog traversal.
Track actual cost and usage separately from reservations and model-price estimates.
Do not promise a strict spend ceiling when the provider's accounting cannot establish one.

Agent flows need typed tools, authorized Resource access, validated outputs, durable step state, cancellation, and restart recovery.
Tool mutations use normal actions and stable idempotency identities. A model output does not grant authority.
Separate ephemeral token streaming from durable result and work events.
Private context disclosure must be explicit. Credentials do not belong in Resources, prompts, logs, or public progress projections.

Domain prompts, Human Design interpretation, and business knowledge remain in the consumer.
The framework should supply execution and composition machinery, not domain answers.

## Completion boundary

Investigation reports and research recommendations do not complete the active release goal.
Complete the selected implementations, native/browser checks, external-consumer acceptance, and production-profile recovery/load/upgrade trials.
Run the full release producer and independent artifact verification against clean source.
Publish the matching source and tag only after the accepted deployment and support limits are recorded.
GitHub remains source hosting; no registry publication or Actions activation is implied.

## Release triage

The intake preserves the investigation classification and adds provisional release triage.
A candidate labeled `shared_rom_defect` still requires a producer reproduction before it becomes a confirmed defect.
Model failover preferences inform the generic routing policy; application content and brand remain in the consumer.
Every candidate remains open until selection and the required regression and external acceptance are complete.

## Intake verification

The [independent intake review](rom-0.1.0-usability-intake-review.md) checked all 30 groups. Its three consistency findings were corrected.
Local checks confirmed 66 source references, 47 test-file references, and 13 local document links before the review record was added.
Whitespace checks passed for the imported documents and consumer handoff. No product tests were executed for this documentation-only intake.
The checked file references identify proposed acceptance locations; their existence does not establish passing tests.
