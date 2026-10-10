# UI Composition Admission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Admit small public session, observation and UI compositions through an independent non-AI consumer.

**Architecture:** Separate transport lifecycle, observation ownership and interaction components. Extract existing Studio behavior where its guarantees match. Keep Resource policy, calculations, routes and document formats in the host.

**Tech Stack:** Existing Rust ROM, TypeScript, Svelte 5.57.1, Bits UI 2.19.5 and vendored shadcn-svelte controls. Existing SQLite/redb adapters and Playwright engines. No new dependencies selected.

**Spec:** [UI composition admission](../../research/rom-0.1.0-ui-composition-admission.md)

## Global Constraints

- Design approval precedes product source changes, new dependencies and builds.
- Package manifest, lock admission, source ownership and native leases require coordinator assignment.
- Preserve `rom-studio/client`, `/recovery`, `/controls`, `/styles` and the existing root API.
- Propose separate `/auth`, `/observe` and `/ui` subpaths; facades contain exports only.
- Host-selected persistence, clocks, locale catalogs, scope and authorization policy remain explicit.
- No process-global private cache, implicit localStorage, new mutation repository or new receipt store.
- Preserve exact ROM wire values, absence, null, revision, principal and unknown commit knowledge.
- Use existing primitives and bounded protocols. Do not create a component controller for each Resource kind.
- No commits, deployment, real secrets or consumer production edits are authorized by this plan.
- New acceptance commands use disposable output directories, 180-second and 8-MiB command budgets, and one browser worker.
- Ports and native allocation require coordinator scheduling; do not reuse another worker's component dist or server.

## Review Focus

- A transient authentication outage must not erase a still-valid view or extend its expiry.
- Identical record IDs under different principals must not retain labels, errors or old callbacks.
- Manual upward scrolling must survive late content growth, resize and pending microtasks.
- Focus must return safely when the opener disappears or a viewport changes while details are open.
- A selected date/revision changed during export must not change the already captured document identity.

---

This umbrella plan contains three separable implementation increments and one admission increment. Approve each independently. Task 1 source was subsequently authorized. Its candidate progress is marked below. Other tasks remain unchecked; their paths are proposed ownership.

### Task 1: Session lifecycle and public auth entry

**Files:** Create `studio/src/auth.ts`; `studio/src/lib/auth/{types,lifecycle,browser-driver}.ts`; `studio/tests/unit/auth-lifecycle.test.ts`. Adapt `studio/src/lib/application/auth.ts` through stable reexports or delegation. Coordinator owns package exports and related lock metadata.

**Interfaces:** Consume `DurablePrincipal` from `/recovery` and the existing validated browser auth protocol. Produce `SessionDriver`, `SessionCheck`, `SessionIdentity` and `createSessionLifecycle` exactly as specified. Add `SessionLifecycleState` with `{ status: 'checking' | 'authenticated' | 'anonymous' | 'denied' | 'transient' | 'disposed'; identity: SessionIdentity | null; transientCode: string | null }`. Snapshots contain no CSRF or provider credential. `onTransition` executes after state invalidation, before exposing new authority; callback exceptions do not restore old state.

- [x] Write tests for single-flight refresh, transient error retention/dismissal, expiry during outage, logout during refresh and current-principal rebind.
- [x] Add adversarial tests for callback-triggered logout/principal change, late check failures, cloned snapshots and absent trusted authority.
- [x] Run `cd studio && node --experimental-strip-types --test tests/unit/auth-lifecycle.test.ts`; retain the intended missing-interface failure.
- [x] Extract the validated browser driver. Keep credential access inside its closure. Define authoritative denial categories from the actual server contract before mapping HTTP errors.
- [x] Implement the ticket-guarded lifecycle and expiry clock.
- [x] Add the public R4 renewal/clear design recipe with actual public calls.
- [x] Implement the approved shared protocol/transport and thin legacy adapter; preserve anonymous generation and typed expiry/stale behavior. Frozen candidate evidence is `/var/tmp/rom-auth-lifecycle-evidence/legacy-evidence.json` (45 targeted, 222 full units, clean type check).
- [x] Independently review the extraction and candidate installed-source/browser baseline (coordinator-reported, authoring only).
- [ ] Review and implement the actual App managed-profile/controller/form design in `/var/tmp/rom-app-session-recovery-design.md`. Seven installed App scenarios map AP-UX-004/005/006; they are not executed. Root owns main.ts/config/manifests.
- [x] Run targeted tests and `npm run check`; record source/diff/log hashes and results.
- [ ] Extend an installed-source import fixture for `/auth` without producer aliases or application graph traversal. Obtain coordinator export admission before its green run.
- [ ] Request independent review of lifecycle and denial semantics before advancing.

Initial candidate evidence: 22 targeted tests and 206 full Studio unit tests passed. A subsequent logout review regression reproduced both throwing/rejected driver failures. The fixed frozen candidate passed 24 targeted and 208 full Studio unit tests; svelte-check reported zero errors/warnings. Logout captures its credential before local invalidation and hooks. Public import RED is retained. The coordinator reports installed-source auth type/build GREEN with browserfalse/admittedfalse. Final independent review, legacy delegation and real auth/browser integration remain pending. Hooks must settle and must not await the same refresh promise. No feedback group is closed.

### Task 2: Bounded scope-owned observation

**Files:** Create `studio/src/observe.ts`; `studio/src/lib/observe/{types,controller,reconnect}.ts`; `studio/tests/unit/observation-lifecycle.test.ts`. Keep `lib/client/stream.ts` as the single stream parser.

**Interfaces:** Consume the existing `Client.observe()` and client generation. Produce `ObservationScope`, `Observation<T>` and `createObservation<T>` as specified. `ObservationState<T>` contains `{ phase, rows: readonly T[], stale: boolean, code: string | null, scope }`. Expose only cloned data. Inject visibility rather than installing implicit global listeners.

- [ ] Write tests for a single active source, hidden abort, visible fresh resume, finite retries and explicit exhaustion/retry.
- [ ] Add rebind tests with identical IDs, public/private scope distinction, denial clearing, client-generation changes, ignored late yields and notification-triggered disposal.
- [ ] Run `cd studio && node --experimental-strip-types --test tests/unit/observation-lifecycle.test.ts`; retain the intended missing-interface failure.
- [ ] Implement state ownership and reconnect scheduling around the existing iterator. Always close/cancel the old source before starting another.
- [ ] Add a recipe that passes live/query bounds and current authority. Treat transport validators as data checks, not authorization evidence.
- [ ] Run targeted tests and `npm run check`; preserve source and failure/pass logs.
- [ ] Add actual HTTP visibility/revocation browser scenarios during Task 4; unit iterator mocks do not establish server policy.
- [ ] Request independent review of scope clearing and resource disposal.

### Task 3A: Interaction mechanics

**Files:** Create `studio/src/ui.ts`; `studio/src/lib/ui/{follow-latest,latest-request,selection,messages,source-link,export-snapshot}.ts`; `studio/tests/unit/ui-{latest,selection,messages,source-link,export}.test.ts`. Create `studio/tests/ui-composition/{scroll.spec,selection.spec,export.spec}.ts` in an isolated fixture.

**Interfaces:** Use the exact function names in the design. `attachFollowLatest` receives an actual HTMLElement and returns methods specified there. `createLatestRequest` state is `{ phase: 'idle' | 'pending' | 'ready' | 'error'; identity: string | null; value: T | null; code: string | null }`; use structured host error classification, not raw exception text. Capture principal, authority ticket, exact ID/revision/date/format/locale and clone the value before calling the renderer. Recheck authority before exposing bytes.

- [ ] Write failing unit cases for superseded success/error/finally, callback-triggered clear, removed selected IDs, hostile URL schemes/credentials and mutated export inputs.
- [ ] Write browser cases for manual upward scroll during late image/text growth, explicit resume, resized container, disconnected element and queued callback after disposal.
- [ ] Add rapid date/selection changes during held calculation/export responses. Confirm the old result cannot publish into the new selection. Test principal change while export is held; no bytes or manual download become available. Optional file-picker cancellation must not trigger a download.
- [ ] Run narrow unit/browser commands after a coordinator port grant; preserve RED logs and screenshots.
- [ ] Implement single-purpose helpers without persistence defaults. Use exact ROM codecs for wire-value snapshots.
- [ ] Run targeted units and both actual engines with one worker. Record executable paths and viewport geometry.
- [ ] Review the API for duplicated R4 mutation semantics; keep latest-request limited to disposable computation/display/export.

### Task 3B: Extract components and document host recipes

**Files:** Create `studio/src/lib/ui/components/{ResponsiveDetails,HistoryList,ReferencePicker,SelectionCard,LayoutControls}.svelte`; `studio/src/lib/ui/layout.ts`; `studio/tests/unit/ui-layout.test.ts`; `studio/tests/ui-composition/{details,reference,history,layout,locale}.spec.ts`. Adapt existing `application/ResponsiveInspector.svelte`, `InspectorToggle.svelte` and `renderers/ReferenceField.svelte` only after ownership is assigned. Add recipes in NEW `docs/studio-compositions.md` after documentation ownership is assigned.

**Interfaces:** Consume Task 3A messages/selection/link helpers and existing controls. Use component props in the design. Extract `ReferenceLookup` and exact bounds from `reference-lookup.ts`; retain compatibility for existing private-context users while admitting explicit public props. Layout validation returns `{ valid: true; layout } | { valid: false; code }`; caller selects fallback and persistence.

- [ ] Write public-import compile failures before adding component exports.
- [ ] Add browser tests at 1280x900, 390x844 and 390x500 with long answers, history overlays, unbroken strings and a visible composer.
- [ ] Test mobile modal/desktop region, focused draft across resize, Escape, missing/disabled opener fallback and reduced-motion complete content.
- [ ] Test exact reference IDs, 250ms debounce, eight attempts, stale labels after authority change, denied lookup and manual fallback. Assert candidate/byte/search limits.
- [ ] Test history accessible full titles, current marker, empty state, date/count localization and uncertain-save navigation barrier.
- [ ] Test selection keyboard/native button semantics and nested sibling actions. Test corrupt layouts, unknown/duplicate IDs, mobile controls and host-store failure.
- [ ] Test locale changes during active pending/error/detail states. Keep host preference persistence and domain catalogs outside components.
- [ ] Extract existing implementations into narrow components; do not copy a second inspector or reference implementation.
- [ ] Run targeted units, check and both actual engines. Record styles/font/animation asset loading from the installed package.
- [ ] Review source compatibility and documentation. Admit package exports through the coordinator.

### Task 4: Independent maintenance portal and release admission

**Files:** Proposed NEW `examples/maintenance-portal/{Cargo.toml,src/resources.rs,src/policy.rs,src/main.rs}`; NEW `studio/tests/ui-composition/consumer/`; NEW `studio/tests/ui-composition/{verify.mjs,http.mjs,playwright.config.ts}`; NEW `docs/research/rom-0.1.0-ui-authoring-review.md`. Coordinator owns workspace and lock additions. Reuse R4 fixture protocol patterns without copying mutation controller logic.

**Interfaces:** Declare Equipment, Inspection, WorkOrder and PortalSettings through public ROM APIs. Svelte consumes only supported public package subpaths. Generic helpers receive host policy, catalogs, exact input identities and store callbacks. No AI dependency or Astral private source.

- [ ] Write the fixture from a frozen source archive. Use a locked npm install; do not copy producer node_modules or rely on symlinks for acceptance.
- [ ] Preserve intended public-entry RED compile/browser failures before minimal implementation admission.
- [ ] Schedule a focused native build with the coordinator. Record source, Cargo.lock, compiler and immutable binary hashes.
- [ ] Run actual HTTP and browser cases against SQLite and redb. Exercise lost acknowledgement, held old-principal responses, server refusal, visibility and restart.
- [ ] Prove identical IDs under different principals clear history, reference labels, drafts and results. Prove transient valid authority retains a stale-labeled view until confirmed expiry.
- [ ] Exercise a technician's complete date/selection/detail/edit/history/export path and a dispatcher's keyboard layout/settings path.
- [ ] Exercise public guest compute/export while the server rejects private writes. Navigation hiding alone cannot pass this scenario.
- [ ] Record final installed-source public import, package CSS/font/animation and both actual-engine results. Fail admission when compiled provenance differs from the frozen artifact.
- [ ] Obtain a held-out human authoring review and separate keyboard/screen-reader/mobile/contrast review. Preserve unresolved findings; agent simulation is insufficient.
- [ ] Run affected tests, full Studio units/type checks, complete affected browser suite and `./scripts/check` under coordinator allocation.
- [ ] Request independent standards/spec review. Update each intake row with its own acceptance evidence and limits; do not close groups from aggregate totals.

#### Task 4 source increment, 2026-10-08

The external Rust application now declares all four Resources through public APIs.
Typed references and validated dates passed two field tests. Embedded journeys passed on SQLite and redb.
Two actual HTTP journeys passed on both databases, including verified handle release and database reopening.
Settings replacement persisted. Receipt replay preserved revision two and exactly two WorkOrder events.
Authenticated ownership denial and fixture credential revocation were tested separately.
All six application tests and Clippy passed.
See `docs/research/rom-0.1.0-maintenance-http-2026-10-08.md` for source evidence and limits.

The Task 4 checkboxes remain open because they include stronger artifact, browser, interruption, and human acceptance requirements.
This increment does not provide an installed Svelte portal or production identity acceptance.

Proposed verifier interface: `node studio/tests/ui-composition/verify.mjs NEW_OUTPUT --source-archive ARCHIVE --host-binary BINARY --host-provenance JSON --adapter sqlite|redb --port COORDINATED_PORT --admit`. It must enforce 180-second/8-MiB command budgets and one browser worker. Browser configuration must read explicit Chromium and actual WebKit executable paths. This command is a planned interface, not an existing runnable verifier.

## Task 3A partial implementation evidence, 2026-10-07

The public `/ui` source entry now exposes `createLatestRequest` and `createSelection`.
The controllers do not perform mutations, infer permissions, or store application data.
Latest-request ownership changes before synchronous abort listeners run. Superseded wrapper promises settle without waiting for uncooperative tasks.
Host tasks still require execution bounds. Host callback defects reject `run`; callers must handle that promise.
An exception from the idle notification propagates synchronously from `clear`.
Selection preserves exact opaque IDs. Catalog replacement validates before it removes unavailable selections.

The new cases first failed because their implementation or public entry was absent.
A separate notification-defect regression failed for unintended classification before the correction.
Ten latest-request/public-entry cases and five selection cases now pass.
The complete Studio unit suite passed 348 cases. Svelte checking reported zero errors and warnings.
Independent latest-request review passed the ten candidate cases and twelve additional reentrancy and cancellation probes.
These counts do not establish installed-consumer, browser, or original-consumer acceptance.

The full verifier passed OpenSpec and 197 Node tests, then stopped at Rust formatting in the demo facade.
The coordinator corrected module ordering after the production worker confirmed that the demo was outside its compiled fixture fence.
A complete rerun is live. Its result is not established by this entry.
Remaining Task 3A helpers, Task 3B components, observation ownership, and independent public consumer admission remain open.
Evidence: /var/tmp/rom-010-ui-composition-candidate-evidence.json.
Review: docs/research/rom-0.1.0-ui-latest-independent-review.md.

## Source-link boundary and review correction

The `/ui` entry now exposes `resolveSourceLink` with an explicit base and permitted external origins.
Internal validation receives the normalized pathname, query, and fragment as an immutable string.
The helper rejects non-HTTP(S) schemes and credentials before calling that validator.
Input has a fixed 8192-byte UTF-8 limit. Invalid configuration throws; an invalid input link returns `null`.
A successful resolution does not authorize or fetch the destination.

Independent review reproduced a JavaScript async validator that returned `false` but approved the link through truthiness.
The maintained regression failed before the correction. Approval now requires the exact boolean `true`.
A second regression reproduced an unhandled rejection from an invalid async callback.
The helper consumes that invalid result's rejection without awaiting or approving it.
Eight source-link cases pass. The complete Studio unit suite passed 357 cases.
Browser and installed-consumer admission remain open.

The full verifier reached the independently configured `rom-auth` build after earlier stages passed.
It found an unconditional export from a JWT-gated module in the no-default-features configuration.
The production identity worker owns the matching export guard and independent feature-matrix verification.
No complete-verifier success is established by that failed run.
Evidence: /var/tmp/rom-010-ui-source-link-corrected-green.log and /var/tmp/rom-010-ui-composition-full-verifier-corrected.log.

## Export snapshot candidate

`captureExportSnapshot` is now exposed through `/ui`.
It copies only the declared identity fields before invoking the host clone callback.
The captured identity is immutable. Each renderer or retry reads a separate clone of the retained value.
The clone contract requires detached, bounded host data. Exact ROM values use the wire codec, not ordinary JSON or structured cloning.
A null principal records explicitly public host context. It does not grant disclosure authority.
The helper does not render, authorize, or write files.
The host must check current authority and latest-request ownership before exposing rendered bytes and again before manual save.

Five tests passed after the missing-module regression. They cover exact integers, member float tokens, retained selection, callback reentrancy, and retry isolation.
Svelte checking reported zero errors and warnings. Independent review and consumer flow acceptance remain open.
Evidence: /var/tmp/rom-010-ui-export-snapshot-red.log and /var/tmp/rom-010-ui-export-snapshot-green.log.

The renewed full verifier stopped at formatting in the newly added AI preparation regression tests.
Those tests remain visible. Their behavior is deliberately unimplemented; a complete green verifier is not established.

## Follow-latest candidate and actual browser regression

`attachFollowLatest` is now exposed through `/ui`.
The controller owns one queued animation frame and a viewport ResizeObserver.
Content updates coalesce through `notifyContent`. The host calls it after changing list content.
Manual upward movement stops following outside the configured end threshold. `resume` restores following explicitly.
Disposal cancels the frame and removes the observer and scroll listener.
Automatic scrolling is instant in both motion modes. No smooth animation can override a later manual movement.

Five unit cases passed after the missing-module regression.
The first actual browser run passed five cases and failed the viewport resize case in WebKit.
A maintained unit regression reproduced that resize event without upward movement.
The correction tracks the previous scroll position before changing following state.
Six unit cases and six actual browser cases then passed across Chromium and WebKit 2359.
The browser fixture transpiles the actual source helper. It does not prove installed-package or original-consumer acceptance.
Evidence: /var/tmp/rom-010-ui-follow-latest-browser.log and /var/tmp/rom-010-ui-follow-latest-browser-corrected.log.

## Follow-latest manual-intent refinement

Independent review found that a small upward movement inside the threshold left following enabled.
A maintained regression reproduced that behavior before correction.
Any upward movement now stops following, except a viewport clamp to its new end position.
Returning to the end by scrolling does not resume following. The user must call `resume` explicitly.
The initial threshold only determines whether the attached view starts following.
Eight focused unit cases and ten actual browser cases passed after this correction.
The browser matrix covers both engines, small upward movement, explicit resume, disposal, viewport shrinking, and viewport enlargement.
These are authoring checks. Installed-consumer and original-consumer acceptance remain open.
Evidence: /var/tmp/rom-010-ui-follow-latest-intent-red.log and /var/tmp/rom-010-ui-follow-latest-intent-browser.log.

## Observation scope boundary candidate

The internal observation scope module now requires an explicit public key or an exact principal and authority generation.
A missing principal does not become a public observation.
The module copies and freezes the scope before later source execution.
It compares identity fields directly, without delimiter concatenation or lossy serialization.
Inherited fields cannot establish a scope. Keys and identity strings have explicit UTF-8 bounds.
Five focused cases passed after missing-module and inherited-field regressions.
This boundary does not grant access. The observation controller, bounded retry lifecycle, public entry, and consumer acceptance remain unimplemented or open.
Evidence: /var/tmp/rom-010-observation-scope-inherited-green.log.

The production worker reported a passing actual HTTPS lifecycle matrix across both adapters and browser engines.
The coordinator inspected the result artifact and confirmed four passed case records.
That evidence remains authoring-only, with production dependency, packaged deployment, and original-consumer acceptance explicitly open.
Evidence: /var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-45fe16fe6cae02a424412229/result.json.

## Public composition and observation correction candidate

The ResponsiveDetails harness now imports `rom-studio/ui/components`. Missing package export produced the intended build failure before admission.
The corrected public-import harness passed 16 browser cases across Chromium and WebKit.
The headless `rom-studio/ui` entry remains loadable by Node without Svelte imports.

A locked installed checkout-source consumer passed type checks, build, and 18 browser cases.
Evidence: `/var/tmp/rom-010-installed-ui-async-corrected-20261007/result.json`.
Its equipment workspace exercises details draft/focus, preview ownership, selection, and synthetic authority-generation clearing.
This does not establish clean release provenance, actual server policy, or original-consumer acceptance.

Observation regressions first reproduced three review defects. A further async callback regression detected an unhandled rejection.
After correction, 20 focused cases passed. The independent review passed eight additional adversarial cases.
See `docs/research/rom-0.1.0-observation-corrective-review.md`.

The layout validator passed six behavior cases and three headless public-entry cases.
A stored getter originally widened host grid bounds; a maintained regression now checks captured configuration.
Current Studio units passed 397 cases, with zero Svelte errors or warnings.
Evidence: `/var/tmp/rom-010-ui-composition-public-candidate-evidence.json`.
The installed snapshot precedes the layout export. LayoutControls, other components, actual HTTP portal flows, and human acceptance remain open.
Recipes and remaining acceptance limits are in `docs/studio-compositions.md`.
