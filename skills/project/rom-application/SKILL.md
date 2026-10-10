---
name: rom-application
description: Use when building an application on ROM, composing sessions and queries, or adding durable browser saves and public projections.
---

# Compose a ROM application

Use this project guidance for the `0.1.0` source candidate. It is not an executable bundle workflow.
Read [release support](../../../docs/release-support.md) before selecting source or package versions.
Read [quality gates](../../../docs/quality.md) before implementation and [writing rules](../../../docs/writing.md) before documentation changes.

## Select public boundaries

| Task | Contract or example |
| --- | --- |
| Resources, ownership, actions, and bounded queries | [Maintenance portal](../../../examples/maintenance-portal/README.md) and [Resource skill](../../rom/rom-resource/SKILL.md) |
| Session lifecycle and UI composition | [Public composition guide](../../../docs/studio-compositions.md), [auth entry](../../../studio/src/auth.ts), and [package exports](../../../studio/package.json) |
| Durable browser mutation | [Recovery entry](../../../studio/src/recovery.ts) and [application recovery tests](../../../tests/application-recovery) |
| Authorized live observation | [Observation entry](../../../studio/src/observe.ts) and the composition guide |
| AI runs and tools | [AI integration skill](../rom-ai-integration/SKILL.md) |
| Controls and semantic editors | [Studio skill](../rom-studio/SKILL.md) |

1. Select the source revision or package archive and matching lockfiles.
2. Declare Resources and actions through public APIs.
3. Define current actor, Resource, field, and public-projection policies before exposing data.
4. Compose public session, recovery, and observation APIs. Keep domain orchestration in the application.
5. Bind private state and pending intent to the exact principal and authority generation.
6. Preserve the original command, expected revision, and idempotency identity after an unknown outcome.
7. Test save → lost acknowledgement → exact retry → reload through the application's public entry points.
8. Test authority changes, stale revisions, rejected input, and switching records while work is pending.
9. Verify the installed package consumer and the actual backend journey separately.

## Decision rules

Use `rom-studio/recovery` for durable mutations. Use latest-request helpers for disposable previews.
Stored intent bytes do not grant server authority. Recheck authority before replay and disclosure.
Confirmed denial clears private views. A transient transport failure does not establish denial or rollback.
A new command key creates new intent; it does not resolve an earlier unknown outcome.
Public projections expose explicitly authorized fields, bounded queries, and revision-aware cache identities.
Test direct raw-resource denial separately from projection success.
Use public package imports and one Svelte runtime. Internal source aliases do not establish packaged acceptance.

## Completion

Run the consumer's affected Rust, frontend, and actual browser checks with its declared tools.
For framework structural changes, also run the full local verifier before integration.
Record source/archive identity, lockfiles, executed commands, failures, and remaining deployment limits.
Fixture actors do not establish production authentication. A successful commit does not establish domain or AI output quality.
