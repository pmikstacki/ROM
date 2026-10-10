---
name: rom-ai-integration
description: Use when integrating AI providers, model routing, typed tools, durable AI flows, or conversation recovery in a ROM application.
---

# Integrate AI with ROM

Use this project guidance for the `0.1.0` source candidate. It is not an executable bundle workflow.
Read [release support](../../../docs/release-support.md) for acceptance limits.
Read [quality gates](../../../docs/quality.md) before implementation and [writing rules](../../../docs/writing.md) before documentation changes.
Start from the [public AI consumer](../../../examples/ai-flows/README.md) and its [complete flow composition](../../../examples/ai-flows/tests/flows.rs).

## Select the boundary

| Concern | Owner or public source |
| --- | --- |
| Prompts, domain truth, citations, and publication intent | Application |
| Provider request, output, and uncertain dispatch | [rom-ai public contracts](../../../crates/rom-ai/src/lib.rs) |
| Model catalog, routing cursor, attempt limits, and money accounting | [Routing API](../../../crates/rom-ai/src/routing/mod.rs) |
| Tools, current grants, and retained command recovery | [Tools API](../../../crates/rom-ai/src/tools/mod.rs) |
| Durable run identity, stages, resume, and cancellation | [Flow API](../../../crates/rom-ai/src/flow/mod.rs) |
| Transport and provider schema | [OpenRouter adapter](../../../crates/rom-openrouter/src/lib.rs) and the selected provider's current documentation |
| Chat layout and durable browser save | [Studio skill](../rom-studio/SKILL.md) and [application skill](../rom-application/SKILL.md) |

1. Define domain success and failure cases independently of provider text.
2. Register Resources, actions, and versioned typed tools through the public consumer pattern.
3. Define current owner, tool-data, and attempt-budget grants through the host authority contract.
4. Freeze the source capture, request identity, routing policy, and execution limits for each durable run.
5. Validate the actual serialized provider request and response against its selected schema.
6. Test route progression, exhausted budgets, rejection, timeout, and provider response loss.
7. Preserve run and command identity after an unknown effect. Resume through the public flow API.
8. Test restart, changed grants, historical citations, and explicit fresh execution on SQLite and redb.
9. Verify conversation visibility, pending state, persistence, and reload in Chromium and WebKit.
10. Test the selected real provider separately from deterministic fixtures.

## Decision rules

Use one finite execution budget across attempts. A resumed route must retain its cursor and consumed limits.
Observe retained read progress before recovery. A caller timeout does not prove that a physical callback stopped.
Same submission identity replays the original run. Fresh generation needs explicit new intent and current source selection.
Cancellation of an unknown effect retains reconciliation state. It does not undo committed domain actions.
Keep research triggers and source selection in application policy. Ordinary replies need not wait for unrelated research.
Validate domain calculations against independent reference cases. Separate calculation versions from saved AI interpretation versions.
Accept current domain context before displaying retained AI output. Preserve immutable source references for historical citations.
Keep provider credentials in host configuration. Use synthetic inputs for request-schema and transport diagnostics.

## Completion

Run the affected core and adapter checks with their required features and the standalone public consumer.
For framework structural changes, also run the full local verifier before integration.
Record source, lockfiles, provider/model identity, commands, and separate fixture, real-provider, and browser results.
Evaluate output usefulness separately from Resource durability, authorization, and schema validity.
Local fixtures do not establish production provider quality or complete application acceptance.
