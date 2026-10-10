# Astral Plane: production consumer feedback for ROM 0.1.0

This report follows the production-readiness priorities in the “Design ROM resource architecture” thread, read on 2026-10-07.
The consumer repository is `/root/astral-plane`. The deployed service is `https://yggdrasil.cybernomad.it`.
The application uses a vendored ROM source tree and a local Studio source alias. This is not packaged-release acceptance.
New conversation generation, public research, and control changes have local evidence below. Their latest versions are not claimed as deployed in this report.

## Observed strengths

Friends, charts, conversations, personal observations, Wiki publications, and enrichment jobs use typed Resources.
Expected revisions and idempotency identities support recovery without an application SQL repository for each entity.
Real production tests exercised Authentik OIDC login, ROM sessions, private friend/chart mutations, SVG retrieval, and CSRF-protected logout.
Browser tests exercised ambiguous save outcomes, conversation restoration, pending-turn resumption, and switching between saved conversations.

Immutable Wiki revision IDs let saved conversations retain their original citations.
A prepared publication is stored before execution. Restart tests replay the same publication command instead of creating a second article.
These are useful consumer examples for the generic commit and receipt contracts.

## Concrete API friction

| Boundary | Consumer evidence | Suggested 0.1.0 action |
|---|---|---|
| Transition validation | Citation validation needs a runtime-specific catalog. The original validator accepted a function pointer, which cannot capture that catalog. | Accept a thread-safe captured validator without changing existing function-pointer callers. Review the consumer patch and runtime-isolation test. |
| Public Studio controls | The source entry exported only Input and Button although Textarea, NativeSelect, Checkbox, Slider, and Label already existed. | Export the supported controls consistently and test them from an external consumer entry point. |
| Control styling | NativeSelect applies class to its wrapper. Consumer styles for the select element need a data-slot selector. | Document wrapper/control boundaries; consider an explicit control-class contract. |
| Runtime license notices | Studio shared its `node_modules` through a symlink. Vite emitted resolved paths, but the notice collector accepted only lexical Studio paths. | Resolve the configured installation before ownership checks. Preserve package identity checks, complete license text, and portable module IDs. |
| Form migration | Textarea uses bind:ref; numeric properties require numeric values. Its content-sizing default can expand a bounded chat composer. | Include a compact composer and date/time/select examples in consumer documentation. |
| Unknown outcomes | Conversation autosave needs pending command identity, expected revision, retry state, save confirmation, and safe switching. | Provide a documented reusable mutation workflow, or show how existing public controllers compose these states. |
| Durable work | The application currently implements an enrichment coordinator over Resources and polling. | Treat this as a discoverability/composition gap to investigate, not proof that ROM lacks work capabilities. Build the same workflow with documented ROM work/chain APIs. |
| Public projections | Anonymous chart/Wiki/research endpoints need safe projections without exposing raw host-only jobs. | Provide an external example covering public projection, authorization, bounded query, cache revision, and redaction. |

The validator patch is documented in `/root/astral-plane/docs/rom-catalog-validator-patch.md`.
Its changes are limited to the vendored Resource definition implementation and tests.
The consumer's full vendored ROM verifier passed before the new Studio exports.
After the export and notice changes, Studio checks, unit tests, and component builds passed locally.
These targeted results do not establish a complete producer verifier run on the final combined revision.

## Separate framework evidence from application defects

Several failures were application bugs: view sequencing during session refresh, clipped chat scrolling, a stale PDF test, and eager reuse of generic question text.
A live AI defect came from dividing a deadline across every discovered provider, leaving about one second for a request that needed several seconds.
Another defect treated personal experience as an encyclopedia lookup. These are not ROM storage failures.
The revised conversation path generates practical responses and follow-up questions from bounded context. It is no longer restricted to selecting Wiki prose.
Chart interpretations require an explicit connection in the current question. Research can continue in the background without blocking an ordinary practical reply.
Validated tools read chart facts, available observations, transits, and Wiki sources, or request a public research topic.
Business evidence uses `practical-life/evidence`, separate from Human Design.
A generic business topic can combine two to eight allowed public words, rather than requiring a fixed article registry entry.
The topic becomes a validated Resource key; duplicate requests share the same durable job. Raw private messages are not used as search queries.

The app can report a successful Resource commit while presenting poor AI content. Framework acceptance must test both independently.
Do not use a model-generated answer or a screenshot as proof of mutation durability or authorization.

## Priorities for the next framework increment

1. Use an external consumer fixture for save → lost acknowledgement → retry → reload. Include permission changes between those steps.
2. Exercise the same workflow through published Rust and Studio entry points, without internal source aliases.
3. Make work state, attempts, receipts, events, and recovery actions discoverable to application authors.
4. Add a runtime-scoped validator example without a process-global registry.
5. Restore a complete deployed application from backup on a clean host. Measure RPO and RTO.
6. Publish the tested single-instance deployment limits and migration/rollback procedure.

## Evidence and limits

Consumer evidence lives in `/root/astral-plane/docs/evidence/` and its Rust/browser test suites.
A production conversation test confirmed automatic persistence, reload restoration, visible content, and cleanup of its temporary conversation.
Wiki tests cover duplicate detection, prepared-publication replay, source provenance, and immutable historical citations.
Research projection tests cover guest access, raw-resource denial, redaction, conditional caching, and restart stability.

The current deployment is single-instance with SQLite. Multi-writer operation, full backup restoration, prolonged overload, and host-loss recovery are not established here.
The research panel and form migrations passed local browser tests. Production login, chart creation, conversation persistence, and the public research projection were checked on 2026-10-07. Individual AI workflow evidence is recorded separately in the consumer repository.

### Shared-install notice regression

The failure occurred in `build:components`: `unclassified runtime notice external module`.
Studio's `node_modules` linked to the consumer's installed dependencies. This was an installation-path defect, not a missing license caused by new control exports.
The collector now resolves that configured installation and retains logical `node_modules/...` identifiers in the inventory.
It still checks package identity and requires complete license files. Unknown external modules and missing licenses still fail the build.

The regression failed before the fix and passed afterward. Local checks recorded on 2026-10-07:

| Check | Result | Evidence |
|---|---|---|
| Studio `npm run check` | 0 errors, 0 warnings | `/var/tmp/astral-rom-studio-check.log` |
| Studio `npm run test:unit` | 146 passed | `/var/tmp/astral-rom-studio-unit.log` |
| Notice and asset-admission tests | 15 passed | `/var/tmp/astral-rom-notice-admission.log` |
| Studio `npm run build:components` | Passed | `/var/tmp/astral-rom-components-fixed.log` |
| Affected consumer forms, Chromium and WebKit | 32 passed | `/var/tmp/astral-fields-browser.log` |

The consumer-side patch changes `vendor/rom/studio/build/notices/ownership.mjs` and its existing notice test file.
The source fix and regression test are candidates for upstream review. The runtime license gate was not disabled.
Evidence paths are local run artifacts; they are not published release certificates.
This file is a handoff for the ROM architecture agent; no cross-thread message-delivery tool is available in this session.
