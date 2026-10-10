# AP UI feedback gap review

Date: 2026-10-07. Status: source/evidence mapping, not consumer acceptance or intake closure.

The frozen details component and headless UI helpers cover useful parts of the AP feedback. They do not satisfy all required UI compositions. Message/locale composition, accessible selection cards, saved history, the public reference picker and validated layout/settings controls remain absent from the inspected public implementation. Actual host export, authority and navigation journeys also remain open.

## Scope and evidence boundary

The review uses the current [usability intake](/root/ROM/docs/research/rom-0.1.0-usability-intake.json), [composition design](rom-0.1.0-ui-composition-admission.md), and [implementation plan](/root/ROM/docs/superpowers/plans/2026-10-07-ui-composition-admission.md). The current intake has 33 entries, including AP-UX-033. Earlier documents refer to 32 groups. This review accounts for all 33 without replacing their original acceptance statements.

The intake still labels classifications provisional. Its pagination record does not establish untruncated feedback or reviewed images. The coordinator's fresh linked-thread synchronization was pending when this task began. This report cannot certify complete current conversation coverage or original consumer acknowledgement.

Inspected source initially included five headless UI helpers, the details component/support files, both UI facades and package exports. The source manifest is under `/var/tmp/rom-010-ui-feedback-gap/`. The details component matches frozen hash `c8487cf65a9aa93f285c15edc7a3199559b6f336dcd5db4798b4dbd06e25ed92` from `/var/tmp/rom-010-responsive-details-evidence/result.json`.

During mapping, the coordinator added `layout.ts`, its test file and its facade export. The reviewer read the emerging validator but did not independently execute or review its full correctness. It validates catalog-driven integer bounds and returns a detached layout or explicit invalid code. It is a new source candidate, not absent implementation or verified admission. Before/after/final manifests preserve this drift; the frozen details source did not change.

Executed details evidence was twenty focused browser passes, 257 full component passes plus one existing WebKit touch skip, 383 then-current Studio units, and clean Svelte checking. These are authoring-source results. Their broad counts must not be interpreted as closure of every AP entry.

The prior independent reports cover [latest request](rom-0.1.0-ui-latest-independent-review.md), [selection/source links](rom-0.1.0-ui-link-selection-independent-review.md), [export capture](rom-0.1.0-ui-export-independent-review.md), and [follow latest](rom-0.1.0-ui-follow-latest-independent-review.md). This mapping did not rerun those tests or the original Astral Plane application.

Public `/ui` now exports headless helpers. Public `/ui/components` exports `ResponsiveDetails`. The coordinator reports a genuine missing-export RED, sixteen source browser passes and eighteen locked installed-fixture passes. The installed snapshot predates the latest observation async correction. This reviewer did not independently inspect its complete source identity or execute it. Installed current-source admission remains distinct from those reports.

## Accounting for every intake entry

“Candidate” below means a reusable mechanism exists or has bounded authoring evidence. It does not mean the entry is accepted. App ownership preserves the original requirement; it does not remove it from the owner's usability goal.

| Entry | Contribution from inspected UI candidates | Remaining acceptance or explicit ownership |
| --- | --- | --- |
| AP-UX-001 Public shared controls | UI facades separate headless mechanics from Svelte details. | Existing controls admission is separate. Current installed public imports, assets/styles and original consumer deployment remain required. |
| AP-UX-002 Styling ownership | Details uses existing public control styles and named component scope. | Document styled-node/class/ref contracts and verify disabled/invalid/focus styling in external composition. |
| AP-UX-003 Compact composer/form bindings | Existing controls and details can host the editor without resize remount. | Compact composer recipe, exact field attributes and actual AP focus/geometry acceptance remain separate. |
| AP-UX-004 Autosave/uncertain recovery | Details preserves one mounted raw editor. Latest-request explicitly excludes mutations. | R4 command identity, unknown/rejected/conflict and actual autosave are separate guarantees; do not substitute cancellation for commit knowledge. |
| AP-UX-005 Safe navigation/drafts | Details retains raw drafts across desktop/mobile resize. | New/switch/delete barriers, identical IDs across owners, server acceptance and authority quarantine require actual host composition. |
| AP-UX-006 Session refresh/sequencing | Current helpers expose host-selected request/snapshot identity. | Auth/R4 binding and current-principal view sequencing remain separate. A UI identity string does not establish authority. |
| AP-UX-007 Durable work progress | Details can display host-provided work facts; observation can carry authorized rows. | Stage presenter, attempts/retry timing, refreshed same-job identity and denied raw-job access are not supplied by these UI helpers. |
| AP-UX-008 Immediate queue/resume | Latest-request supplies disposable pending/result ownership. | Durable pending queue, reload/resume identity and queue bounds require work/recovery composition. UI pending is not persisted work. |
| AP-UX-009 Long-result/composer geometry | Details scrolls long content at 390x500 and preserves its close control semantics. | No fixed composer/footer composition is implemented here. Required AP 1280x900, 390x844 and 390x500 unobstructed composer/history layouts remain open. |
| AP-UX-010 Manual scroll | Follow-latest candidate preserves upward reading until explicit resume and disposes frame/observer work. | Installed host integration, late actual image growth and real AP interaction remain required. Preserve the existing touch-test limitation. |
| AP-UX-011 Responsive panels/focus/motion | Details verifies desktop region/mobile modal, Escape, enabled opener/fallback, mounted editor, overlap locks and reduced motion. | Current installed source review, AP integration, screen-reader reading order and independent human assessment remain open. |
| AP-UX-012 Localization/errors | Details updates its host-supplied close label while open. Other helpers accept classified codes/context locale. | `MessageResolver`/catalog composition is absent. Active progress/error translation, date/plural locale, preference persistence and stable documented paths remain open. |
| AP-UX-013 Stale/authorized observation | Observation corrective candidate has scope/ticket fencing, finite retry and hidden-state control. | Actual visibility adapter, transport integration, current installed correction and private/public authority journeys remain separate. Host async effects need their own post-await checks. |
| AP-UX-014 Useful grounded conversation | Details/link mechanics can present consumer results. | Answer generation and computed/cited/generated distinction remain app/AI-owned acceptance. Commit or a modal test proves no answer usefulness. |
| AP-UX-015 Lazy deduplicated research | Latest-request suppresses obsolete disposable results. | Durable deduplication, article policy and conversation availability remain app/work-owned. The helper cannot stop an uncooperative task. |
| AP-UX-016 Tool transparency | Details supplies a generic surface for disclosed evidence. | Actual executed-tool provenance and private input filtering remain app/AI-owned. No tool evidence presenter exists here. |
| AP-UX-017 Selection/computation | Exact selection snapshots and latest-request fencing exist. | `SelectionCard` or accessible card recipe is absent. Whole-card keyboard/click behavior, announced selected state and held actual calculation remain open. |
| AP-UX-018 Dashboard/layout/settings | ResponsiveDetails supplies reusable drawer/region mechanics. A catalog-driven layout validator appeared during review. | Independent validator verification, persistence recipe and `LayoutControls` remain open. Keyboard/mobile move/resize, corruption handling and reload acceptance remain open. |
| AP-UX-019 Domain terminology/spacing | Host labels/snippets permit domain wording. | Long translated chart/SVG geometry and semantic terminology remain app-owned. Short details fixture labels do not prove those cases. |
| AP-UX-020 Autocomplete/manual fallback | Latest-request can supply stale-result fencing. Existing private reference behavior is preserved elsewhere. | Public `ReferencePicker` extraction/recipe is absent. Exact IDs/coordinates, keyboard selection, debounce/limits and manual fallback require explicit acceptance. |
| AP-UX-021 Capability navigation/guest | Export capture accepts explicitly public context; details has no auth side effects. | Server denial behind hidden admin links, explicit public computation/export and expiry clearing require actual host/server journeys. Null principal is no grant. |
| AP-UX-022 Saved history | Selection and details are usable lower-level mechanisms. | `HistoryList` is absent. Accessible long title, empty/current/count/date presentation and pending-mutation navigation barrier remain open. |
| AP-UX-023 Sources/application context | URL resolver checks scheme/credentials/origin and normalized internal route policy; details returns focus. | Immutable source/reference identity, actual inline citation flow, deep-link route correctness and retained AP conversation require host/browser acceptance. |
| AP-UX-024 Fresh versus retained execution | Latest-request owns a host-selected token. Export capture records revision/date. | Explicit rerun operation, durable run provenance, stored-result reopening without recompute and visible freshness remain work/app-owned. |
| AP-UX-025 Selected-snapshot export | Export capture detaches identity/value and gives fresh copies for renderer retries. | Actual renderer/busy/failure/retry, long localized documents, post-render authority, manual-save checks and picker cancellation remain open. |
| AP-UX-026 Private profile/observations | Generic scope/recovery mechanisms can support the app. | Owner-only model/edit/delete and distinction from computed facts/history remain app-owned acceptance. |
| AP-UX-027 Provider failover | UI can present current host-supplied work status. | Provider order, budgets, accepted settlement and same-work identity remain AI/app-owned; no routing policy is added by these UI helpers. |
| AP-UX-028 Theme/brand | Details uses existing theme tokens and disables its content motion under reduced motion. | AP brand, contrast, long translated geometry and nonblocking transitions require consumer and human review. |
| AP-UX-029 Daily/relationship readings | Host tokens can distinguish calculation settings/date. | Daily freshness, relationship/business rules and domain cache identity remain app-owned acceptance. |
| AP-UX-030 Withdrawn video rate | No contribution required. | Preserve the explicit withdrawal. Do not reinterpret it as a ROM 60 FPS animation requirement. |
| AP-UX-031 Date/temporal selection | Latest-request and export identity carry selected context. | Explicit initial date, keyboard/touch timeline, actual calculation switch and exported displayed snapshot remain app-owned and unverified here. |
| AP-UX-032 Withdrawn music/sound | No contribution required. | Preserve the explicit withdrawal and separate conversation ownership. |
| AP-UX-033 Guest invitation/account save | Existing generic forms/recovery may preserve draft and retry identity. | Invitation token/consent/domain flow, optional-account transition and exactly one sender-related Resource remain application-owned; tests were not independently rerun here. |

## Concrete next gaps

Complete the selected missing public compositions: message/locale contract, selectable-card recipe/component, history presentation, reference picker extraction and catalog-driven layout controls. Independently review the emerging layout validator. Document public imports, styled nodes, disposal, exact values, navigation barriers and host policy. No replacement domain subsystem is needed to supply those selected mechanics.

Compose export capture with latest-request and a real renderer. Test a held old-principal result, revocation before manual save, changed selected date/revision, retry from the original capture and cancelled picker without automatic fallback download. The capture helper cannot supply those guarantees alone.

Admit the current shared source through a locked installed non-AI consumer. Record both `/ui` and `/ui/components` source/lock/browser identities. Keep authoring/source-transpiled fixtures separate from that artifact evidence. Observation's async correction must be present in any consumer claim that exercises observation.

Reconcile the 33-entry intake and older 32-row design accounting after the fresh thread sync. Preserve application-owned requirements and withdrawn rows explicitly. Do not mark an app-owned requirement satisfied merely because ROM selected no domain implementation.

Finally, obtain original consumer acknowledgement and independent human/assistive-technology review. Programmatic modal roles, focus assertions, screenshots or a passing full suite cannot replace those gates. This report adds no defect classification for app source it did not inspect and closes no intake entry.
