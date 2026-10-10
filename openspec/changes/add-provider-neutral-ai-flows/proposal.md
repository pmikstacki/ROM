# Add provider-neutral AI routing and durable flows

## Why

Astral Plane duplicates provider routing and work coordination around ROM Resources.
Its deadline, schema and retry defects show concrete reuse opportunities.
Its prepared publication and search reservations supply useful recovery examples.

## What changes

Add optional `rom-ai` contracts and Resource-backed flow composition.
Add optional `rom-openrouter` HTTP/catalog/error adaptation.
Keep ROM core free from AI policy, providers, HTTP clients and database drivers.
Reuse existing actions, channels, receipts, Work supervision and operator controls.

Persist run-scoped policy, deadlines, routing continuation and reservations.
Provide typed reads and typed Resource action tools under current host authority.
Distinguish confirmed result, invalid output, failed execution and unknown external outcome.

## Compatibility and migration

Existing public paths and Resource/action/event semantics remain stable.
Existing applications need not depend on either new crate.
New Resources require explicit host registration. Automatic rate-limit recovery requires a generic delayed-channel scheduling prerequisite.
That prerequisite includes a separate enforced storage/archive reader compatibility decision; no AI-specific state enters core.
Applications retain domain prompts, citation validation and publication actions.
This proposal does not migrate Astral Plane or deploy a provider integration.

## Scope and evidence

Use deterministic providers and loopback HTTP fixtures first.
No live inference, paid requests or provider exactly-once guarantee forms part of initial acceptance.
See [design](design.md), [tasks](tasks.md), and [implementation plan](../../../docs/superpowers/plans/2026-10-07-ai-routing-and-flows.md).
The source investigation is [maintained research](../../../docs/research/rom-0.1.0-ai-investigation.md).
