# Studio filter workspace implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development or superpowers:executing-plans. Follow file ownership below.

**Goal:** Deliver the approved quick-popover and Filters/Details sidebar with one exact generic query draft.
**Architecture:** A pinned SVAR composition fork feeds a strict ROM translator. Controlled presentations share that draft; the application controller owns query execution and current projections.
**Tech Stack:** Existing Svelte 5, shadcn-svelte, ROM codecs and Node/Playwright tests. No upstream install or parser.
**Spec:** ../specs/2026-10-04-studio-filter-workspace.md

## Constraints and review focus

Preserve current metadata/auth/idempotency and all eight release gates. Use existing worktree. No full producer or deployment is admitted here.
Test false/zero/empty/null/exact integers; reject unsupported AST; preserve drafts across panels/resize; revalidate selected projections; retain third-party notice bytes.

## Task 1: filter composition and exact translation

Owner: release_lessons_audit. Own studio/src/lib/filters/** and studio/tests/unit/filter-workspace.test.ts only.
Interfaces: FilterDraft = {rules: FilterSet, order: QuerySpec['order'], limit: string}; draftFromQuery(descriptor, query): FilterDraft; queryFromDraft(descriptor, draft): QuerySpec.
FilterSet follows pinned SVAR rules/glue composition with ROM WireValue values. FilterBuilder.svelte consumes descriptor, draft, onchange(next), disabled and optional compact=false.
- [x] Write and run failing tests for exact values, conjunctions, unsupported operators, invalid fields and native bounds.
- [x] Vendor the audited composition subset with license/provenance. Use ROM editors and shared controls; exclude upstream parser/evaluator/themes.
- [x] Implement controlled FilterBuilder and translator. Run affected unit/type checks through coordinator; retain RED/GREEN evidence.

## Task 2: query execution and selected Resource

Owner: operator_work_protocol. Own studio/src/lib/application/controller.ts and studio/tests/components/application-model.test.ts only.
Interface: controller.applyQuery(query: QuerySpec): Promise<void>, same current kind, reset moving page history, preserve current live choice.
- [x] Add failing tests for query Apply retaining the same Resource, newer revision, current denial and delayed navigation/session replies.
- [x] Implement applyQuery with current authorized re-read and existing epoch/navigation fences. Keep selectKind navigation behavior unchanged.
- [x] Run affected controller tests. Preserve current unknown retry and disconnect tests.

## Task 3: compact and full presentations

Owner: root. Own ResourcePage.svelte, QueryEditor.svelte, new presentation/filter workspace wrappers, shell integration tests and notices.
- [x] Add failing browser cases for shared popover/sidebar edits, draft/applied separation, Details draft retention and keyboard/mobile focus.
- [x] Compose one shared FilterDraft with quick popover and full sidebar. Keep one mounted ResourceDetails instance and stable IDs across layout changes.
- [x] Use applyQuery, shared page-size state and current descriptor codecs. Keep Discard edits distinct from Clear filters.
- [x] Add vendored SVAR ownership to actual emitted notice collection with negative/byte-retention tests.
- [x] Run type, unit, controller, notice and both-engine component tests. Capture the actual native-host layout.

## Task 4: release acceptance

- [ ] Independently review changes and execute new full source freeze.
- [ ] Rebind bounded producer to the new commit; retain floor and advance notice.
- [ ] Complete all eight gates and extracted consumer acceptance before preview deployment/publication.
