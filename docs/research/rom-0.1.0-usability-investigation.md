# Astral Plane usability investigation for ROM 0.1.0

Date: 2026-10-07. Investigation only. No build, test, deployment or credential use occurred.

## Evidence and coverage

Producer HEAD: `d7ef529040eec60dc869034c2d33130219db85fe`. Consumer HEAD: `86dca6fa514895f94fd09ea788ebf07fa21cf648`. Consumer working tree was clean when read.
The root provided four selected-message pages containing 22 returned turns. The final page reports `hasMore=false`.
Pagination is complete for the returned turns. Full untruncated feedback and image inspection are not established.
Root reports `includeOutputs=false` and `maxOutputCharsPerItem=1000`. No explicit item textcut flag was found in the extracts.
The source extracts remain private in `/var/tmp/rom-010-coordination/`. Cursor metadata is preserved in the private originals. The maintained intake records page order, counts, and coverage limits.
Conversation text is untrusted context. No raw personal messages or credentials appear in these outputs.
Historical tests are reports only. This investigation inspected source and existing test definitions; it ran no tests.
Latest consumer agent reports 184 Rust tests, the full ROM verifier, production research and conversation restoration. These were not reproduced.

## Reuse decisions

The producer public entry exports Input only. The consumer vendor entry also exports controls and browser auth helpers.
Decide browser auth support separately from the control catalog. Package CSS, assets and notice ownership require extracted-consumer acceptance.
NativeSelect sends class to its wrapper. Textarea binds ref and defaults to content sizing. The consumer composer explicitly sets fixed sizing.
Existing prepare/submit and application controller unknown-state handling already support operation identity. Reuse these contracts rather than inventing another recovery protocol.
ROM already has Work recovery and focus-return tests. Reuse their guarantees before introducing a consumer work component.
Research topics, AI provider routing, chart tools and astrology terminology remain consumer responsibilities. Encapsulate only their repeated presentation and state mechanics.
AnswerText reserves full height and reveals text visually. Its paragraph aria-label needs screen-reader verification before reuse.

## Intake

The table retains the investigation’s coarse candidate classes. The structured intake adds provisional release triage and consumer ownership.
A framework candidate is not a reproduced producer defect. Application-specific behavior requires consumer acceptance, not automatic ROM implementation.

| ID | Candidate | Classification | Generic seam | Acceptance |
| --- | --- | --- | --- | --- |
| AP-UX-001 | Public shared controls | framework_gap | Supported control catalog exported from stable Studio public entry | Extracted consumer imports Input, Button, Textarea, NativeSelect/options/groups, Checkbox, Slider and Label without source aliases; semantics and styles compose. |
| AP-UX-002 | Control styling ownership | framework_gap | Document wrapper/control class and data-slot contract; evaluate explicit control class | Width, disabled, invalid and focus styles reach intended nodes without removing accessibility. |
| AP-UX-003 | Compact composer and form bindings | framework_gap | Public compact form recipe using bind:ref, typed attributes and bounded sizing | Multiline draft preserves focus and remains bounded; exact date/time/select values survive; numeric attributes compile. |
| AP-UX-004 | Autosave and uncertain mutation recovery | framework_gap | State adapter over existing prepare/submit; same identity, expected revision and authority scope | Lost acknowledgement retries same command; input changes wait; confirmed rejection releases it; explicit unknown/conflict/rejected states; permission changes cannot disclose prior results. |
| AP-UX-005 | Safe navigation and drafts | framework_gap | Navigation barrier composed with mutation state; app owns routing and domain snapshot | New/switch waits for save confirmation; uncertain save/delete retains current view; same IDs across sessions render selected content; authority changes discard private state. |
| AP-UX-006 | Session refresh and view sequencing | framework_gap | Public auth lifecycle and generation-scoped state composition; decide auth exports separately | Same-principal refresh preserves view; confirmed expiry clears private state; transient failures retain context and dismiss after recovery; checks share a single flight. |
| AP-UX-007 | Actual work stages and durable progress | framework_gap | Authorized stage/status presenter with app-supplied labels and result links | Refresh observes same job; queued/running/waiting/done distinguished; attempts/next retry visible; no fabricated percentage; raw jobs remain denied. |
| AP-UX-008 | Immediate pending queue and resume | framework_gap | Bounded workflow observation/resume recipe; preserve identity and explicit cancellation | Input appears immediately; pending identities survive save/reload; same run resumes; queue bounds explicit; disposed scope cannot update new view. |
| AP-UX-009 | Long results and composer geometry | framework_gap | Bounded panel with independently scrollable result and footer | At 1280x900, 390x844 and 390x500 composer visible/uncovered, answer readable; expanded history does not cover controls; long strings wrap. |
| AP-UX-010 | Respect manual scroll | framework_gap | Optional follow-latest helper independent of domain | Near-end readers follow new content; upward manual scroll stays fixed; explicit submit can opt in; resize observers dispose; add missing behavior test. |
| AP-UX-011 | Responsive panels, focus and reduced motion | framework_gap | Existing dialog/drawer primitives plus focus-return and motion contracts | Mobile modal/desktop nonmodal behavior reviewed; Escape closes and returns focus to actual opener; reduced motion reveals full text; manual screen-reader assessment required. |
| AP-UX-012 | Localization and structured error messages | framework_gap | Inject message resolver and locale into controls/status; map structured error codes | Language changes update active error/progress; date/plural locale correct; preference persists; no raw untranslated backend error; stable catalog paths. |
| AP-UX-013 | Stale cache and recoverable observation | framework_gap | Bounded authorized observation/cache with visibility-aware polling | One request at time; hidden tab aborts, visible resumes; last-known state labeled stale; private caches cleared on authority changes; transport validators never replace authorization. |
| AP-UX-014 | Useful conversations and grounding | app_specific | Keep answer generation and domain routing in consumer; generic framework carries work/status | Ordinary conversation answers during research; computed/cited/generated content distinguished; useful answer acceptance separate from Resource commit. |
| AP-UX-015 | Lazy topic research and deduplication | app_specific | App topic policy over generic durable work contracts | General mention creates no article; concrete gap queues deduplicated work with sources; conversation stays usable. |
| AP-UX-016 | Domain tool transparency | app_specific | App supplies permitted tool evidence to generic progress/details layout | Only executed chart/profile/transit/research tools reported; private tool inputs not disclosed. |
| AP-UX-017 | Selectable cards and reactive computation | framework_gap | Accessible selectable-row/card recipe and latest-request guard; domain compute/cache keys stay app-owned | Whole card toggles selection by click and keyboard; multiple selections allowed; stale computation cannot replace newer result; selected state visible and announced. |
| AP-UX-018 | Dashboard widget layout and settings drawer | framework_gap | Validated layout/settings Resource plus responsive drawer; evaluate Svelte-native existing primitives before adopting React package | Persist valid position/visibility; reject corrupt layout; keyboard/mobile alternatives to dragging; supported layout survives reload; app widget catalog stays app-owned. |
| AP-UX-019 | Chart terminology and dynamic label spacing | app_specific | Generic lesson: content-sized labels and semantic translation; Human Design mappings remain consumer | Long translated chart headings retain padding and avoid clipping; domain terminology checked by app; SVG mapping tests preserve transform identifiers. |
| AP-UX-020 | Autocomplete with explicit manual fallback | framework_gap | Async option-picker recipe with debounce, stale-request protection and explicit selected identity | Typing stale query cannot overwrite selection; keyboard selection works; unavailable lookup allows manual input; labels do not substitute exact IDs or coordinates. |
| AP-UX-021 | Admin-only navigation and guest workflows | framework_gap | Capability-driven navigation and shared public/private form composition | Admin link absence backed by server denial; guest computes/downloads without persisting private profile; expired identity clears protected values. |
| AP-UX-022 | Readable saved-history list | framework_gap | Generic selectable saved-session history recipe: title, timestamp, count and current marker | Long titles truncate safely with accessible name; empty/history states fit panel; active selection clear; locale dates correct; navigation respects pending mutation. |
| AP-UX-023 | Source links retain application context | framework_gap | Generic inline detail/dialog and stable reference link recipe; app owns Wiki routes | Citations open correct immutable source without losing current conversation; close returns focus; deep links preserve identity; unsafe URLs rejected. |
| AP-UX-024 | Fresh execution versus retained result | framework_gap | Explicit rerun operation distinct from observing stored result; cache identity remains domain-specific | New question obtains new run identity; reopening stored result does not recompute; rerun is deliberate; freshness and provenance visible. |
| AP-UX-025 | PDF/export from actual selected snapshot | framework_gap | Download state and immutable selected-snapshot recipe; domain document layout app-owned | Download busy/failure/retry clear; output uses displayed revision/date; long text and localization fit; errors do not discard current result. |
| AP-UX-026 | Private observations/profile | app_specific | App profile model over ordinary authorized Resources; generic edit/delete/recovery controls | Observations visible/editable/deletable only to owner; not conflated with computed facts or raw conversation history. |
| AP-UX-027 | Model provider failover policy | app_specific | App-specific provider order and budget over generic bounded retry work | Free then bounded paid then local CPU policy explicit; same pending task survives retry; deadline and attempt budgets retained. |
| AP-UX-028 | Theme and domain brand | app_specific | Generic theme tokens and progressive transitions; app selects celestial design | Public controls accept app theme; transitions never block navigation; reduced motion supported; contrast assessed. |
| AP-UX-029 | Daily questions and relationship-aware readings | app_specific | App business rules; generic scheduled-work observation, cache and form primitives only | Daily freshness explicit; relationships and system scopes do not alter generic ROM contracts; distinct calculation settings own cache identity. |
| AP-UX-030 | Video frame-rate request withdrawn | app_specific | No ROM work; unrelated request explicitly withdrawn | Do not treat the 60 FPS video request as a ROM animation requirement. |

## Feedback and source map

### AP-UX-001: Public shared controls

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a115c2-999b-77b2-a90f-449043de2e9a`, item `01a115d4-52de-7940-84e1-9f10c22816e2`; turn `01a111a6-5613-79d3-85e0-929a83b1e01b`, item `01a111a6-8652-77f2-a157-82fdc6a27a7d`.
Sources: `/root/ROM/studio/src/index.ts`, `/root/astral-plane/vendor/rom/studio/src/index.ts`, `/root/astral-plane/web/src/FriendEditor.svelte`.
Existing test definitions: `/root/ROM/studio/tests/components/controls.spec.ts`.

### AP-UX-002: Control styling ownership

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: source candidate; no direct user attribution.
Sources: `/root/ROM/studio/src/lib/components/ui/native-select/native-select.svelte`, `/root/astral-plane/web/src/FriendEditor.svelte`.
Existing test definitions: `/root/ROM/studio/tests/components/controls.spec.ts`.

### AP-UX-003: Compact composer and form bindings

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: source candidate; no direct user attribution.
Sources: `/root/ROM/studio/src/lib/components/ui/textarea/textarea.svelte`, `/root/astral-plane/web/src/ChartQuestions.svelte`, `/root/astral-plane/web/src/FriendEditor.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/chat-layout.spec.ts`, `/root/astral-plane/web/tests/city.spec.ts`.

### AP-UX-004: Autosave and uncertain mutation recovery

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11544-afce-73e3-bd8c-8eb974868ce2`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11558-e61d-7752-be93-c871d4753ea0`.
Sources: `/root/astral-plane/web/src/ConversationSessions.svelte`, `/root/astral-plane/web/src/App.svelte`, `/root/ROM/studio/src/lib/application/controller.ts`.
Existing test definitions: `/root/astral-plane/web/tests/chat-workflow.spec.ts`, `/root/astral-plane/web/tests/recovery.spec.ts`.

### AP-UX-005: Safe navigation and drafts

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11558-e61d-7752-be93-c871d4753ea0`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a1156a-d91a-7212-9e9e-9076bad3e617`.
Sources: `/root/astral-plane/web/src/ConversationSessions.svelte`, `/root/astral-plane/web/src/ChartQuestions.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/chat-workflow.spec.ts`.

### AP-UX-006: Session refresh and view sequencing

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11544-afce-73e3-bd8c-8eb974868ce2`.
Sources: `/root/astral-plane/web/src/App.svelte`, `/root/ROM/studio/src/lib/application/auth.ts`.
Existing test definitions: `/root/astral-plane/web/tests/session-navigation.spec.ts`, `/root/astral-plane/web/tests/recovery.spec.ts`.

### AP-UX-007: Actual work stages and durable progress

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a115c2-999b-77b2-a90f-449043de2e9a`, item `01a115ce-a388-7962-b7d4-fb57537d1cd8`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11544-afce-73e3-bd8c-8eb974868ce2`.
Sources: `/root/astral-plane/web/src/WorkflowProgress.svelte`, `/root/astral-plane/web/src/AstralResearch.svelte`, `/root/astral-plane/web/src/research-client.ts`.
Existing test definitions: `/root/astral-plane/web/tests/reading-workflow.spec.ts`, `/root/astral-plane/web/tests/research-client.test.ts`.

### AP-UX-008: Immediate pending queue and resume

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11545-3d0d-7c00-8495-f9da0cae4305`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11546-17cf-79e2-a544-46c43f247674`; turn `01a11513-66a4-7c30-a318-bf4611d830f6`, item `01a11515-cdc7-7813-bb13-795536c30ce2`.
Sources: `/root/astral-plane/web/src/question-queue.ts`, `/root/astral-plane/web/src/ConversationSessions.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/chat-persistence.test.ts`, `/root/astral-plane/web/tests/question-queue.test.ts`, `/root/astral-plane/web/tests/chat-workflow.spec.ts`.

### AP-UX-009: Long results and composer geometry

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a1156a-d91a-7212-9e9e-9076bad3e617`.
Sources: `/root/astral-plane/web/src/SidePanel.svelte`, `/root/astral-plane/web/src/ChartChatPanel.svelte`, `/root/astral-plane/web/src/AnswerText.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/chat-layout.spec.ts`.

### AP-UX-010: Respect manual scroll

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: source candidate; no direct user attribution.
Sources: `/root/astral-plane/web/src/ChartQuestions.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/chat-layout.spec.ts`.

### AP-UX-011: Responsive panels, focus and reduced motion

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11513-66a4-7c30-a318-bf4611d830f6`, item `01a11513-66f2-7e51-a457-c55ac930e799`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11580-7143-7523-b431-d6555c66d146`; turn `01a114f7-6fad-7971-9d7d-f16db46896c4`, item `01a114fa-0461-7270-a9a5-6c5875448a10`.
Sources: `/root/astral-plane/web/src/SidePanel.svelte`, `/root/astral-plane/web/src/AnswerText.svelte`, `/root/astral-plane/web/src/WorkflowProgress.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/chat-layout.spec.ts`, `/root/ROM/studio/tests/components/work-presentation.spec.ts`.

### AP-UX-012: Localization and structured error messages

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11243-dda9-7e43-947e-b6f9efde021d`, item `01a11247-51d8-7dd0-af9e-a274cba8bd71`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11577-2d9a-7c30-a7fb-e9765a34b9cc`.
Sources: `/root/astral-plane/web/src/i18n.ts`, `/root/astral-plane/web/src/i18n/workflow.ts`, `/root/astral-plane/web/src/AppNavigation.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/i18n.test.ts`, `/root/astral-plane/web/tests/design-languages.spec.ts`.

### AP-UX-013: Stale cache and recoverable observation

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a115c2-999b-77b2-a90f-449043de2e9a`, item `01a115ce-a388-7962-b7d4-fb57537d1cd8`.
Sources: `/root/astral-plane/web/src/AstralResearch.svelte`, `/root/astral-plane/web/src/research-client.ts`.
Existing test definitions: `/root/astral-plane/web/tests/research-client.test.ts`.

### AP-UX-014: Useful conversations and grounding

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a115c2-999b-77b2-a90f-449043de2e9a`, item `01a115d6-8250-7ab0-83fb-9d74c8688fae`.
Sources: `/root/astral-plane/web/src/interpretation.ts`, `/root/astral-plane/web/src/ChartQuestions.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/interpretation.test.ts`.

### AP-UX-015: Lazy topic research and deduplication

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a115c2-999b-77b2-a90f-449043de2e9a`, item `01a115e5-a77a-7e23-84e3-d10aae3817b0`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11583-fb95-7f63-ba69-8698ced76e0b`.
Sources: `/root/astral-plane/web/src/AstralResearch.svelte`, `/root/astral-plane/web/src/interpretation.ts`.
Existing test definitions: `/root/astral-plane/web/tests/research-client.test.ts`.

### AP-UX-016: Domain tool transparency

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a115c2-999b-77b2-a90f-449043de2e9a`, item `01a115d8-8e0d-7602-b6fa-904b1db8144f`.
Sources: `/root/astral-plane/web/src/ChartQuestions.svelte`, `/root/astral-plane/web/src/ReadingContent.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/reading-workflow.spec.ts`.

### AP-UX-017: Selectable cards and reactive computation

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11560-ac1c-7830-a70f-5773e9ee35fa`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11561-963d-7f23-844c-9c93423fde06`.
Sources: `/root/astral-plane/web/src/FriendSelectionList.svelte`, `/root/astral-plane/web/src/ChartView.svelte`, `/root/astral-plane/web/src/chart-selection.ts`.
Existing test definitions: `/root/astral-plane/web/tests/chart-selection.test.ts`, `/root/astral-plane/web/tests/relationships.spec.ts`.

### AP-UX-018: Dashboard widget layout and settings drawer

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a1151b-50ef-7543-b1ba-fe182955be8b`, item `01a1151b-5132-72c1-92ad-fcb366809351`; turn `01a1151b-50ef-7543-b1ba-fe182955be8b`, item `01a11523-e310-7f92-9dd4-2d7e0900e143`; turn `01a1151b-50ef-7543-b1ba-fe182955be8b`, item `01a1152c-98a2-7611-b4f8-c331f2248544`.
Sources: `/root/astral-plane/web/src/DashboardLayoutPanel.svelte`, `/root/astral-plane/web/src/UserDashboard.svelte`, `/root/astral-plane/web/src/dashboard-layout.ts`.
Existing test definitions: `/root/astral-plane/web/tests/dashboard-layout.test.ts`, `/root/astral-plane/web/tests/dashboard.spec.ts`.

### AP-UX-019: Chart terminology and dynamic label spacing

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a114d4-c6a9-7030-a34a-e651c8f505e6`, item `01a114d4-c6eb-71d0-8403-c8df6e36f309`; turn `01a114d4-c6a9-7030-a34a-e651c8f505e6`, item `01a114d6-b525-71a0-8751-bd6c799845c2`; turn `01a11243-dda9-7e43-947e-b6f9efde021d`, item `01a11270-d02a-7fc3-a1bc-07db3bd3e2da`; turn `01a114f7-6fad-7971-9d7d-f16db46896c4`, item `01a114f7-7014-7830-8172-cc21add51a8e`.
Sources: `/root/astral-plane/web/src/ChartRenderer.svelte`, `/root/astral-plane/web/src/ChartBodygraph.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/svg-refresh.spec.ts`, `/root/astral-plane/web/tests/design-languages.spec.ts`.

### AP-UX-020: Autocomplete with explicit manual fallback

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a114d4-c6a9-7030-a34a-e651c8f505e6`, item `01a114d6-4ae9-79a1-a78b-eb0040c85281`; turn `01a114d4-c6a9-7030-a34a-e651c8f505e6`, item `01a114d8-6a57-7c21-a042-aeb586efa397`.
Sources: `/root/astral-plane/web/src/BirthCity.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/city.spec.ts`, `/root/astral-plane/web/tests/city.test.ts`.

### AP-UX-021: Admin-only navigation and guest workflows

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a114b0-49a3-7e90-b0bd-9ca83a0a7cc2`, item `01a114b0-4a4f-7c62-b716-c2b3b97d4c7a`; turn `01a11204-cc70-7680-b7b1-b0742aa75b00`, item `01a11204-ccc0-73e3-94fb-2e73c2a54f91`.
Sources: `/root/astral-plane/web/src/AppNavigation.svelte`, `/root/astral-plane/web/src/App.svelte`, `/root/astral-plane/web/src/GuestCalculator.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/studio-access.spec.ts`, `/root/astral-plane/web/tests/guest.spec.ts`.

### AP-UX-022: Readable saved-history list

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a1156a-d91a-7212-9e9e-9076bad3e617`.
Sources: `/root/astral-plane/web/src/ConversationSessions.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/chat-workflow.spec.ts`, `/root/astral-plane/web/tests/chat-layout.spec.ts`.

### AP-UX-023: Source links retain application context

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a1132b-dfe1-7b82-b469-e3f368694819`, item `01a1132b-e01b-7382-b87a-53fb19941d8d`; turn `01a114f7-6fad-7971-9d7d-f16db46896c4`, item `01a114fa-0461-7270-a9a5-6c5875448a10`.
Sources: `/root/astral-plane/web/src/wiki-links.ts`, `/root/astral-plane/web/src/FeatureDialog.svelte`, `/root/astral-plane/web/src/KnowledgeLibrary.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/wiki-links.test.ts`, `/root/astral-plane/web/tests/wiki-navigation.spec.ts`.

### AP-UX-024: Fresh execution versus retained result

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11544-afce-73e3-bd8c-8eb974868ce2`; turn `01a11243-dda9-7e43-947e-b6f9efde021d`, item `01a11243-e948-7392-bd59-27609c0b2dbf`.
Sources: `/root/astral-plane/web/src/question-queue.ts`, `/root/astral-plane/web/src/interpretation.ts`.
Existing test definitions: `/root/astral-plane/web/tests/chat-workflow.spec.ts`, `/root/astral-plane/web/tests/interpretation.test.ts`.

### AP-UX-025: PDF/export from actual selected snapshot

Status: investigated_not_implemented. Review: source_reviewed; producer acceptance pending.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11562-2a35-7061-a5b4-956451d93595`.
Sources: `/root/astral-plane/web/src/PdfDownload.svelte`, `/root/astral-plane/web/src/pdf-export.ts`.
Existing test definitions: `/root/astral-plane/web/tests/pdf-download.spec.ts`, `/root/astral-plane/web/tests/pdf-export.test.ts`.

### AP-UX-026: Private observations/profile

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11558-e61d-7752-be93-c871d4753ea0`.
Sources: `/root/astral-plane/web/src/PersonalProfile.svelte`, `/root/astral-plane/web/src/personal-profile.ts`.
Existing test definitions: `/root/astral-plane/web/tests/personal-profile.test.ts`.

### AP-UX-027: Model provider failover policy

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a1151b-50ef-7543-b1ba-fe182955be8b`, item `01a1152d-81bd-7f10-8ab1-5d72e8b89b69`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11570-b070-7dd1-a2bf-8800e1d32bb5`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11573-c0cd-7e41-85d9-8fd12aa08f88`.
Sources: `/root/astral-plane/web/src/interpretation.ts`.
Existing test definitions: `/root/astral-plane/web/tests/interpretation.test.ts`.

Later request adds bounded paid tier then local CPU; earlier free-to-local order superseded.

### AP-UX-028: Theme and domain brand

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a111b9-65fa-7720-9919-74d1145dd098`, item `01a111d0-00b1-7923-8e3f-c2ca1ac9a2a1`; turn `01a11243-dda9-7e43-947e-b6f9efde021d`, item `01a1126d-36c3-7b13-97c3-65457c94b6d4`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11560-ac1c-7830-a70f-5773e9ee35fa`.
Sources: `/root/astral-plane/web/src/AppNavigation.svelte`, `/root/astral-plane/web/src/ChartView.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/design-languages.spec.ts`.

### AP-UX-029: Daily questions and relationship-aware readings

Status: investigated_not_implemented. Review: source_reviewed; consumer-owned; producer acceptance required only for a separately selected generic seam.
Feedback: turn `01a1151b-50ef-7543-b1ba-fe182955be8b`, item `01a11520-3037-72c0-848c-c9a2d1a520d4`; turn `01a1151b-50ef-7543-b1ba-fe182955be8b`, item `01a11523-29e7-73f2-b77a-28725a7ea45c`; turn `01a11544-af8a-78d1-af2e-2c3df58a89f3`, item `01a11583-fb9a-77a0-9694-f062bc7e8a02`.
Sources: `/root/astral-plane/web/src/ReflectionQuestions.svelte`, `/root/astral-plane/web/src/relationship-context.ts`, `/root/astral-plane/web/src/TransitPanel.svelte`.
Existing test definitions: `/root/astral-plane/web/tests/reflections.spec.ts`, `/root/astral-plane/web/tests/relationship-context.test.ts`.

### AP-UX-030: Video frame-rate request withdrawn

Status: withdrawn. Review: source_reviewed; closed_withdrawn; no producer acceptance required.
Feedback: turn `01a115c2-766b-7200-b992-27a17497e05a`, item `01a115c2-76a6-74d3-baa7-728a4e3cfb01`; turn `01a115c2-999b-77b2-a90f-449043de2e9a`, item `01a115c2-99d1-74d0-a928-40460585c476`.
Sources: none.
Existing test definitions: none.

## Duplicates, changed direction and exclusions

Repeated requests for pending queues, retained conversations, panels and language catalogs are grouped under stable IDs. All known item references are retained.
The earlier free-model-to-local failover order was expanded to bounded paid models followed by local CPU. AP-UX-027 records the changed policy.
The earlier restrained encyclopedia-selection approach was expanded to practical generated conversation and domain tools. AP-UX-014 records current consumer scope.
The 60 FPS video request was explicitly withdrawn as belonging to another thread. AP-UX-030 is closed without ROM work.
Application naming, account provisioning, secrets and external identity-provider console setup are excluded from reusable UI feedback.
Attached screenshots were not inspected. Their text requests identify candidates but do not establish visual reproduction.

## Next acceptance steps

The source map is established before further research. No dependency or upstream recommendation was selected here.
Research unresolved public packaging, accessibility and localization seams against official documentation and the locked versions.
Implement generic helpers through stable exports and existing semantic contracts. Keep invalid drafts, missing/null/removal and unknown outcomes distinct.
Run affected component/unit checks and the complete local verifier after structural changes. Record source identity and actual results.
Repeat with an unrelated consumer using extracted public artifacts. Internal aliases and vendor patches do not establish release acceptance.
This report and JSON are a shared-file handoff. No direct consumer-thread message or acknowledgement is claimed.
