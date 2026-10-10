# Public UI composition admission for ROM 0.1.0

Date: 2026-10-07. Status: proposed design; coordinator review required before implementation.

The release needs public compositions for session, observation and interaction behavior that consumers currently assemble themselves. Select small helpers and components. Reuse Studio's existing controls, reference lookup and responsive inspector. Keep application data, computations, routes and export formats in the consumer.

This document does not close feedback groups. The [32-group intake](rom-0.1.0-usability-intake.json) remains canonical. Its classifications are provisional triage, not reproduced defect claims. The linked consumer thread marker remains unchanged, as reported by the coordinator. Pagination reached its last returned page; truncated text and unavailable visual context limit coverage. A shared-file handoff does not establish consumer acknowledgement.

## Source investigation before research

Source inspection established this map before primary-document research. No test or build was run for the initial design. The separately authorized Task 1 candidate checks are recorded below.

| Boundary | Existing behavior and source | Missing public composition |
| --- | --- | --- |
| Session transport | `studio/src/lib/application/auth.ts` validates bounded session/provider responses, expiry and CSRF fields. Logout invalidates its epoch. | `refresh()` is not single-flight. Non-success responses do not distinguish authoritative denial from transient failure. The helper is private. |
| Consumer session lifecycle | `/root/astral-plane/web/src/App.svelte` coalesces refresh, preserves views on transient errors, clears on expiry, and restarts streams on session changes. | Repeated host policy; same-principal renewal also clears pending state. R4 recovery must retain accepted mutation identity. |
| Observation transport | `studio/src/lib/client/session.ts` caps observations; `stream.ts` bounds frames and rows, cancels readers, and checks client generation. | Visibility, fresh-snapshot reconnect, stale indication and principal-bound view ownership require host code. |
| Consumer observation | `App.svelte` retries two streams with bounded backoff. `research-client.ts` caches bounded public, credential-free results. | Distinguish explicitly public data from private data with no known principal. Never reinterpret unavailable private authority as public scope. |
| References | `renderers/ReferenceField.svelte` and `reference-lookup.ts` retain exact IDs, bound search/candidates, abort stale work and keep manual input. | Lookup context and component remain private. Existing contracts should be extracted, not replaced by a second picker. |
| Details | `application/ResponsiveInspector.svelte`, `preserve-editor.ts` and `InspectorToggle.svelte` preserve mounted drafts across resize and return mobile focus. | Public snippet/ref/label contract; locale injection; explicit modal policy and long-content geometry. |
| Following content | Consumer `ChartQuestions.svelte` has a 100px near-end threshold, `ResizeObserver`, explicit following state and disposal. | Public lifecycle helper; tests for late resize, manual scroll, content replacement and reduced motion. |
| History and sources | Consumer `ConversationSessions.svelte` formats localized timestamps/counts. `FeatureDialog.svelte` restores previous focus. | Generic history presentation and validated source-link policy. Wiki routes and citation meaning stay consumer-owned. |
| Selection and dated calculation | Consumer `UserDashboard.svelte` binds selection and aborts superseded transit requests. | Exact selection identity and a reusable latest-request guard. Calculation semantics remain consumer-owned. |
| Layout | Consumer `dashboard-layout.ts` validates fixed widget IDs, dimensions and positions. Settings provide move/resize buttons. | Parameterized catalog and injected persistence; no Astral widget IDs or default localStorage in ROM. |
| Export | Consumer `pdf-export.ts` creates domain documents and loads its own licensed fonts. | Snapshot identity and asynchronous busy/retry ownership, independent of PDF implementation. |
| Settings and fields | `docs/studio-presentation.md` and `docs/studio-fields.md` describe authorized settings, exact codecs and editor drafts. | Public recipes for semantic values, capability navigation and localized presentation. No new settings write API. |

Relevant existing tests include `auth-expiry.test.ts`, `reference-picker.test.ts`, `reference-picker.spec.ts`, `right-inspector.spec.ts`, `presentation-ux.spec.ts`, and `settings-presentation.spec.ts`. Test source is evidence of scenario intent. The coordinator reports 221 component passes and one skip, plus installed public App cases. This worker did not rerun those suites for this design.

R4 candidate evidence was executed separately: 38 targeted helper tests, 184 Studio unit tests, and five cases per engine per adapter. SQLite and redb each exercised real HTTP, restart, lost acknowledgement, IndexedDB and held responses. These authoring-source results do not establish final release-artifact admission or original consumer behavior.

## Selected boundaries and public interfaces

Keep `rom-studio/client`, `/recovery`, `/controls`, `/styles` and the existing root API stable. Propose new `/auth`, `/observe` and `/ui` subpaths. The coordinator owns package export admission. Each facade contains exports only. `/auth` and `/observe` must compile without traversing App or Svelte components. `/ui` may contain Svelte components. Do not add dependencies or copy a second primitive library.

### Session lifecycle

Create `lib/auth/{types,lifecycle,browser-driver}.ts`. Move the validated browser endpoint behavior behind a driver. Keep credentials inside that driver's closure; observable state and persistence contain none.

```ts
interface SessionIdentity {
  principal: DurablePrincipal;
  generation: string;
  expiresAt: number;
}
type SessionCheck =
  | { status: 'authenticated'; identity: SessionIdentity }
  | { status: 'anonymous' }
  | { status: 'denied' }
  | { status: 'transient'; code: string };
interface SessionDriver {
  invalidate?(): void; // Clear local credential authority; no server mutation.
  check(signal: AbortSignal): Promise<SessionCheck>;
  logout(signal: AbortSignal): Promise<void>;
}
createSessionLifecycle({ driver, now, onTransition }): SessionLifecycle
```

`DurablePrincipal` is the existing durable principal type from `/recovery`. A host supplies trusted authority and principal kind. The browser driver supplies the validated user subject; generation and CSRF never become principal identity. The driver classifies only explicit protocol denial, validated anonymous responses or confirmed expiry as lost authority. Network failure, invalid response and a generic server error remain transient.

`SessionLifecycle` provides `refresh(): Promise<SessionCheck>`, `logout(): Promise<void>`, `dismissTransient(): void`, `subscribe(listener): () => void` and `dispose(): void`. Simultaneous refresh calls share one promise. Logout, disposal and authority changes invalidate tickets before notification. Snapshots are cloned and sanitized. The host clock returns UTC seconds and has no side effects. A custom expiry scheduler queues callbacks asynchronously and returns a nonthrowing cancellation function. Transient errors retain current authority only until its validated expiry. Dismissal does not extend expiry.

`onTransition` receives `renewed`, `changed`, `cleared` or `transient`. Same-principal renewal rebinds the current client and R4 controller without erasing an accepted intent. Changed/cleared authority quarantines R4 state before rendering or accepting another result. Transition hooks may await R4 rebinding, but must settle. A hook must not await the same lifecycle refresh promise. Logout/disposal can supersede a hook; every completion checks its ticket. Capability navigation consumes authorized discovery; it never grants permission. Guest compute/download is an explicit host capability, separate from private persistence.

### Observation ownership

The candidate uses `lib/observe/{types,controller,scope}.ts`. Wrap existing `Client.observe()` and bounded query transport; do not implement another stream parser.

```ts
type ObservationScope =
  | { kind: 'public'; key: string }
  | { kind: 'principal'; principal: DurablePrincipal; generation: number };
createObservation<T>({ scope, source, clone, measure, limits, retry, classifyError, onAuthorityLost, onCallbackError? }): Observation<T>
```

`source(signal)` returns an `AsyncIterable<readonly T[]>`. `clone` validates/copies host data; source limits remain enforced by the existing transport. `limits` requires positive `maxRows` and `maxBytes`. `retry` requires finite attempt and elapsed-time ceilings, delay bounds and a host clock. There is no infinite retry default.

Methods are `start()`, `setVisible(boolean)`, `rebind(scope, source)`, `retry()`, `subscribe(listener)` and `dispose()`. Each controller admits one current source owner and one retry timer. Cancellation is cooperative; an obsolete iterator can remain running if it ignores cancellation. States are `idle`, `connecting`, `fresh`, `stale`, `denied`, `exhausted` and `disposed`. Hidden state aborts work and cancels retry timers. Visibility resumes from a fresh snapshot. A changed scope or authority generation clears rows before starting new work. An exactly equal scope may retain labeled stale rows. A transient error may retain a labeled stale snapshot for the same scope. Denial clears it and invokes the authority callback. Every await, timer and notification checks the ticket and scope. ETags/cursors never establish authority. There is no process-global cache or implicit document listener.

The host must supply detached, bounded clones and exact encoded byte measurement. ROM wire values require their codec, not ordinary JSON cloning.
The classifier must return sanitized codes. It does not automatically redact host-supplied text.
Retry elapsed limits bound reconnect admission; they do not limit the lifetime of an active observation.
Source adapters must bound connection acquisition and honor cancellation independently. An invalid initial clock is a synchronous configuration error; explicit retry can recover after correction.

Throwing subscribers are removed. Other subscribers and authority-loss handling continue.
Optional `onCallbackError` receives only `ObservationSubscriber` or `ObservationAuthorityCallback`. Diagnostic exceptions and returned Promise rejections cannot interrupt lifecycle handling. Subscriber removal occurs when rejection settles; notifications are not coalesced while callbacks remain pending. Hosts must fence their own awaited effects.
Callback-driven rebind, hide, and disposal invalidate obsolete continuations before another source can start.

The corrective candidate has 20 focused tests and 390 Studio unit tests passing, with zero Svelte diagnostics.
The independent review and actual installed-consumer observation acceptance remain required. These internal results do not close consumer feedback.

### Interaction helpers

Create separate modules under `lib/ui/`; use callbacks and explicit handles instead of an all-purpose context.

| Public interface | Contract |
| --- | --- |
| `attachFollowLatest(element, { thresholdPx, reducedMotion, onFollowing })` | Returns `notifyContent()`, `resume()`, `snapshot()` and `dispose()`. Near-end follows new layout. Manual upward scrolling disables following. Resize does not move an unfollowed viewport. Explicit submit can call `resume()`. Dispose invalidates queued layout work and disconnects observers. |
| `createLatestRequest<T>({ clone, onState })` | `run(identity, task)`, `clear()` and `dispose()`. Abort previous work; guard late successes, errors and finalizers with a ticket. Identity is an immutable host-selected input token, not a label. No mutation retry or accepted-intent semantics. |
| `createSelection({ ids, initial, multiple })` | Exact opaque IDs; immutable selected snapshot; `toggle`, `replaceAvailable`, `clear`. Removed IDs disappear. A native-button card recipe supplies pressed/selected announcements and excludes nested interactive targets. |
| `MessageResolver` | `(key: UiMessageKey, values?: Readonly<Record<string, string | number>>) => string`. Components receive `messages` and `locale`; they recompute active statuses on locale change. Host owns catalogs/preference persistence. Use Intl for dates and plural selection. Unknown backend text is not a translation key. |
| `resolveSourceLink(raw, { base, internal, externalOrigins })` | Returns validated internal/external link or rejection. Parse URLs, reject credentials and non-HTTP(S) external schemes, require explicit allowed origins, and use a host validator for internal paths. Return normalized href and external flag; do not fetch it. |
| `captureExportSnapshot({ identity, value, clone })` | Copy once before async rendering. Identity contains durable principal, authority ticket, exact Resource ID, revision, selected date, format and locale. A latest-request controller owns busy/failure/retry. Retrying retains that captured snapshot; changing selection creates a new request. Recheck current authority before exposing bytes or writing a file. No document engine or object-URL ownership hidden in ROM. |

Do not serialize exact ROM values through ordinary JSON. Public recipes use `stringifyWire`/`parseWire` where persistence or exact cloning requires codecs. A host clone for ordinary display data must explicitly define its accepted shape.

The proposed headless signatures are:

```ts
interface LatestRequest<T> {
  run(identity: string, task: (signal: AbortSignal) => Promise<T>): Promise<void>;
  clear(): void;
  dispose(): void;
}
interface Selection {
  readonly selected: readonly string[];
  toggle(id: string): void;
  replaceAvailable(ids: readonly string[]): void;
  clear(): void;
}
interface FollowLatest {
  notifyContent(): void;
  resume(): void;
  snapshot(): { following: boolean };
  dispose(): void;
}
```

Require a finite nonnegative `thresholdPx`. Validate unique available selection IDs; an unavailable ID cannot be selected. `run` resolves after publishing or suppressing its outcome; current failures become sanitized state, not unhandled promises. The host supplies `classifyError(error: unknown): string`. A caller cancellation of disposable computation never changes R4 commit knowledge.

### Latest-request lifecycle refinement

The controller changes ownership before it aborts previous work. Abort listeners can synchronously start another request.
The controller checks ownership after task, clone, classifier, and notification callbacks.
Superseding, clearing, or disposing resolves the old wrapper immediately, even if its underlying task ignores cancellation.
This does not stop uncooperative work. The host must enforce execution bounds for its task.
Current task failures become public codes from the host classifier. Host callback exceptions reject the wrapper as programming errors.
Disposal does not emit another state. Calls after disposal have no effect.

The candidate implementation and public source entry passed ten focused unit cases on 2026-10-07.
Svelte checking reported zero errors and warnings. Installed-consumer and browser acceptance remain open.
Primary-source research: [latest-request research](rom-0.1.0-ui-latest-primary-research.md).
Evidence: /var/tmp/rom-010-ui-latest-public-green.log and /var/tmp/rom-010-ui-latest-check.log.

### Components and settings layout

Create `lib/ui/components/{ResponsiveDetails,HistoryList,ReferencePicker,SelectionCard,LayoutControls}.svelte` with focused contracts.

`ResponsiveDetails` accepts `open`, `onOpenChange`, `title`, `description`, `closeLabel`, `opener`, `breakpoint` and `children: Snippet`. It reuses existing responsive inspector/preserve-editor behavior and vendored Sheet/Dialog primitives. Desktop is an inline nonmodal region; mobile is modal. Escape closes mobile details and returns focus to a connected, enabled opener. If the opener disappeared, the host supplies `fallbackFocus`. Content remains mounted across breakpoint changes. Reduced motion displays complete content immediately. No animated answer reveal is a required dependency.

`HistoryList` accepts immutable entries `{ id, title, instant, count? }`, `selectedId`, `onSelect`, `messages`, `locale` and `empty`. Visible titles wrap or truncate with a full accessible name. IDs are not parsed for titles. Selection invokes the host's R4 navigation barrier. The component does not save, query or infer conversations.

`ReferencePicker` extracts existing bounded reference behavior. Pass lookup explicitly rather than requiring a private context alias. Candidates are exact `{ id, title }`; selected titles are authority-bound disclosure. Preserve manual exact ID input on lookup failure. Keep the existing 20-candidate, 65536-byte response and 1024-byte search bounds. Keep 250ms nonempty debounce and eight lookup attempts per opened flow. An authority token change clears labels and invalidates all awaits. Place coordinates remain an application-specific adapter with explicit value validation.

`SelectionCard` renders a native button with a host snippet. Nested action controls remain siblings. It does not turn a container with arbitrary interactive children into a second click target.

`lib/ui/layout.ts` validates `{ id, x, y, width, height, visible }` against an explicit host catalog and integer bounds. Catalog entries declare min/max width and height; options require finite positive columns, maxRows and maxItems. Reject unknown/duplicate IDs and corrupt entries. Return an explicit invalid result; the host selects reset/fallback. `LayoutControls` exposes move/resize/show commands with keyboard buttons. Store callbacks are host-selected and principal-bound; no localStorage default. Use existing Settings Resources for shared durable settings. Domain widget catalogs and grid algorithms remain consumer-owned until a second independent consumer proves a shared guarantee.

## Acceptance mapping for every feedback group

“Proposed” means required future acceptance, not verified consumer behavior. Evidence for one row does not close another row.

| ID | Existing evidence or ownership | Proposed admission or explicit exclusion |
| --- | --- | --- |
| 001 | R2 controls installed-source engine baseline; final package admission separate. | Public entries compile, style and load assets without private aliases. |
| 002 | Primitive wrapper/ref/class tests. | Document styled node, width, invalid/disabled and focus; manual accessible-name review. |
| 003 | Compact Textarea baseline. | Exact date/time/select/number attribute recipe; 1280x900, 390x844 and 390x500 composer remains visible. |
| 004 | R4 two-adapter unknown-outcome candidate. | Compose autosave without new mutations while accepted intent is unknown; later valid/invalid edits remain drafts. |
| 005 | R4 guarded navigation and retained drafts. | Identical IDs under different principals, New/Switch/delete routing and authority clearing through public recipes. |
| 006 | Private auth transport; consumer single-flight. | Concurrent refresh, transient dismissal, expiry, same-principal renewal and late old-principal callbacks. |
| 007 | Core/AI work projection belongs to coordinator's R5 work. | UI uses actual durable stages and provenance; no invented progress percentages. |
| 008 | R4 pending lane and AI queue work are separate. | Immediate queued/running/unknown display; resume uses existing accepted identity, not a fresh execution. |
| 009 | Control sizing only. | Long answers, unbroken strings, history/detail overlays and short viewport keep composer usable. |
| 010 | Consumer scroll implementation inspected. | Near-end follows; upward scroll stays fixed; explicit resume; observer/tick disposal. |
| 011 | Private inspector focus tests inspected. | Mobile modal/desktop region, resize preserves editor, Escape/opener fallback, reduced-motion full text. |
| 012 | Consumer locale stores inspected. | Active error/progress/date/plural updates; host preference; structured category resolver, no raw backend catalog keys. |
| 013 | Bounded transport inspected. | One active request, hidden abort/visible fresh resume, finite reconnect, stale labeling, principal cache clearing. |
| 014 | Application-specific conversation quality. | Consumer owns content/grounding. Generic long-content presentation remains covered by 009. |
| 015 | Application-specific lazy research. | Consumer owns research policy; latest-request guards stale disclosure and duplicate in-flight UI work. |
| 016 | Application-specific domain tool transparency. | Render disclosed provenance through work projection; no fabricated tooling labels. |
| 017 | Checkbox primitive; consumer selection/calculation. | Whole-card keyboard selection and announcements; rapid selection/date change cannot publish old results. |
| 018 | Consumer validation and keyboard controls inspected. | Catalog-bound layout, reload, corrupt input, visibility and mobile/keyboard alternatives. |
| 019 | Domain chart terminology and SVG identity. | Consumer owns chart regression. Generic translated headings use content-sized layout; no numeric DOM IDs inferred. |
| 020 | Existing bounded private picker tests inspected. | Public explicit lookup adapter, exact selection, delayed stale result, keyboard, denial and manual fallback. |
| 021 | Authorized Settings/discovery contracts exist. | Capability navigation, real server refusal, guest compute/export without private persistence, confirmed expiry clears values. |
| 022 | Consumer history inspected. | Full accessible title, timestamp/count/current marker, empty state, locale and navigation barrier. |
| 023 | Consumer dialog/source behavior inspected. | Immutable source identity; unsafe links rejected; inline detail preserves view; Escape returns focus; exact deep-link token. |
| 024 | Core/AI execution identity owns freshness. | Selection/latest-request never confuses retained result with fresh execution; display provenance from actual response. |
| 025 | Consumer PDF implementation owns format. | Export exact revision/date/locale captured before await; busy/failure/retry and long localized text. |
| 026 | Application-specific private profile/observations. | Consumer owns model and disclosure policy; generic scope clearing tested through 013. |
| 027 | Application-specific provider policy; AI routing work separate. | No UI provider failover policy or new provider dependency. |
| 028 | Domain brand stays consumer-owned. | Public tokens, readable contrast, nonblocking transitions and reduced-motion acceptance; human visual review. |
| 029 | Application-specific daily/relationship readings. | No ROM reading generator. Latest/date identity covered by 017/031. |
| 030 | Explicitly withdrawn video frame-rate request. | Excluded; no video work. |
| 031 | Application-specific timeline and temporal calculation. | Explicit initial date, keyboard/touch changes select a new snapshot, exact selected date exported; computations stay consumer-owned. |
| 032 | Explicitly withdrawn music/sound request. | Excluded; no audio work. |

## Independent non-AI authoring consumer

Add a maintenance portal fixture only after review. Declare `Equipment`, `Inspection`, `WorkOrder` and `PortalSettings` through public Rust ROM APIs. Use authorized discovery, semantic date/decimal/multiline fields, reference IDs, actions and presentation settings. Use no AI, astrology, chat queue or consumer-private code.

A technician selects equipment cards, changes an inspection date, opens inline history/details, edits a work order and exports a captured inspection snapshot. A dispatcher owns layout settings. A guest can inspect explicitly public information and export it without private writes. Use the same public helpers with different data and navigation. Distinct users may have identical Resource IDs; view, reference labels, draft and history remain principal-bound.

Actual HTTP fixtures must run SQLite and redb. Drop acknowledgements after commit and hold old-principal responses. Use real session refusal and visibility interruption. The installed Svelte fixture imports supported package subpaths only. Install from a frozen release-source archive with npm ci; record source, lock, native binary, browser executable and actual adapter identities. Authoring copies are troubleshooting evidence, not release admission.

## Evidence and human review boundary

Unit tests establish ticket/state/codec behavior. Actual browsers establish focus, scrolling, geometry and IndexedDB behavior. Real adapters establish commit/receipt/restart behavior. Automated accessibility checks establish only their checked rules.

Recruit a human who did not implement these fixtures. Ask them to declare a custom field, compose a public reference lookup, observe authorized records, recover an uncertain save, and diagnose a refused operation. Record task completion, wrong turns, documentation searches, time and unresolved confusion. Separately review keyboard, screen-reader reading order, mobile touch targets, long translated labels, theme contrast and reduced motion. Record browser, assistive technology and reviewer role. An agent-created example or simulated reviewer cannot satisfy this gate.

No new source ownership, dependencies or build allocation is granted by this document. The [implementation plan](../superpowers/plans/2026-10-07-ui-composition-admission.md) separates independently reviewable deliveries. Final integration requires affected checks, full local verifier, independent installed-source admission and human evidence or an explicit recorded limitation.

## Current primary-source research

The coordinator assigned a separate read-only researcher after this source map. Official live documentation was read on 2026-10-07. Resolved checkout versions are Svelte 5.57.1, Bits UI 2.19.5 and shadcn-svelte 1.7.0. These are not claims about latest registry versions. The live pages are generally unversioned.

Svelte effects return cleanup on rerun and destruction. An async onMount callback cannot return the ordinary cleanup function. getAbortSignal requires an active effect or derived owner. Public headless helpers therefore own explicit disposal; Svelte wrappers connect that disposal synchronously. [Effects](https://svelte.dev/docs/svelte/$effect), [lifecycle](https://svelte.dev/docs/svelte/lifecycle-hooks), [abort signal](https://svelte.dev/docs/svelte/svelte#getAbortSignal).

HTTP 401 and 403 have different meanings. A 403 can concern authorization rather than credentials. The proposed single-flight lifecycle is a ROM design inference, not a platform feature. Define server-specific authoritative denial before retry policy. Fetch abort does not establish mutation rollback; R4 knowledge remains authoritative. [HTTP authentication and denial](https://www.rfc-editor.org/rfc/rfc9110.html#section-15.5.2), [403](https://www.rfc-editor.org/rfc/rfc9110.html#section-15.5.4), [retry semantics](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.2.2), [AbortController](https://developer.mozilla.org/en-US/docs/Web/API/AbortController/abort).

Background timers can be throttled. Visibility adapters must explicitly pause work and resume from current authority. ResizeObserver needs disposal; resize-driven layout changes can repeat callbacks. Svelte's chat example measures near-bottom state before insertion and scrolls after tick. Its threshold is an example, not a standard. Schedule at most one layout update; cancel it on disposal. Preserve the manual reading anchor and page scroll. [Visibility](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API), [ResizeObserver](https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserver), [Svelte chat example](https://svelte.dev/docs/svelte/lifecycle-hooks#Deprecated-beforeUpdate-afterUpdate-Chat-window-example).

Sheet position does not define modality. Bits Dialog and Popover document focus trapping; popovers cannot be assumed nonmodal. Explicitly configure behavior and test actual wrappers. Modal details contain tab focus, expose a title and restore focus to the invoker or a logical successor. Use a static focused heading for long structured content where appropriate. [shadcn Sheet](https://shadcn-svelte.com/docs/components/sheet), [Bits Dialog](https://bits-ui.com/docs/components/dialog), [Bits Popover](https://bits-ui.com/docs/components/popover), [W3C dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/), [disclosure pattern](https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/).

Prefer CSS for visual reduced-motion behavior. Svelte MediaQuery values can differ during hydration. Imperative scrolling may read motion preference, but immediate state and focus changes must remain available. [Reduced motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion), [Svelte MediaQuery](https://svelte.dev/docs/svelte/svelte-reactivity#MediaQuery).

Intl date defaults depend on the environment; recipes pass locale and time zone explicitly. Display strings never become durable dates or IDs. URL parsing does not restrict schemes: link policy checks protocol, origin and credentials. Keyboard focus and selection are separate concepts. Ordinary history links/buttons avoid imposing listbox semantics on embedded actions. [Intl.DateTimeFormat](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat), [URL parsing](https://developer.mozilla.org/en-US/docs/Web/API/URL/URL), [W3C listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/), [combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/).

Reference lookup is distinct from browser file pickers. If a consumer uses a save-file picker, call it from a user gesture and handle limited availability explicitly. Cancellation must not trigger an automatic download. Preserve the captured export identity, recheck authority and offer a separate manual action. Object URLs require explicit lifetime cleanup. No file-picker dependency is selected here. [Save picker](https://developer.mozilla.org/en-US/docs/Web/API/Window/showSaveFilePicker), [object URLs](https://developer.mozilla.org/en-US/docs/Web/API/URL/createObjectURL_static), [revocation](https://developer.mozilla.org/en-US/docs/Web/API/URL/revokeObjectURL_static).

These sources support platform constraints. They do not establish ROM acceptance, current server authorization, durable recovery, browser compatibility or human usability.

## Task 1 implementation candidate

The coordinator authorized only Task 1 after reviewing this design. New source is confined to `studio/src/auth.ts` and `studio/src/lib/auth/`. At this initial checkpoint, the application auth helper was unchanged. The later approved legacy extraction candidate is recorded below. Tasks 2–4 are not implemented.

Source inspection of `crates/rom-studio-host/src/router.rs` shows validated anonymous sessions for missing, expired or denied cookies. `authentication.rs` emits JSON `denied` with 401, `overloaded`/`closed` with 503, and `internal` with 500. The browser driver recognizes those categories. A malformed 401 response is not credential denial. Malformed responses cannot establish or extend identity; an existing validated identity expires at its original deadline.

The browser driver has a fixed 65536-byte response bound and a 10000ms default acquisition/logout deadline. It captures its host configuration. Browser identity is human, with trusted host authority and validated user subject. Its explicit `csrf()` transport accessor is absent from lifecycle state and transition snapshots. Local `invalidate()` clears credentials without sending a mutation.

Locally executed candidate checks: 22 session lifecycle/driver tests; full Studio unit suite 206 passed; svelte-check reported zero errors and zero warnings. Test-first failure/pass logs and source identity are under `/var/tmp/rom-auth-lifecycle-evidence/`. Scenarios include single-flight through async rebinding, original expiry, ignored abort, malformed replies, held browser responses, logout/notification reentrancy, hook failure, invalid clocks, bounded bytes and sanitized/cloned snapshots.

The initial public import failed with ERR_PACKAGE_PATH_NOT_EXPORTED. The coordinator owns `./auth` manifest admission. No browser, native build, real authentication journey, packaged release admission or human review was executed for this candidate. AP-UX-006/012/021 remain open until their specific acceptance is complete.

Independent review found that a custom driver could retain local credentials when logout threw or rejected. The lifecycle now invokes logout first, then calls local invalidate before hooks or publication. This order preserves the browser driver’s captured POST credential. Ticket checks suppress outdated invalidation and callbacks. Both failures were reproduced before the fix; final review candidate checks passed 24 targeted tests and 208 Studio unit tests, with zero type errors/warnings. Evidence: `/var/tmp/rom-auth-lifecycle-evidence/review-evidence.json`. The earlier 22/206 evidence remains historical.

The coordinator reports public `/auth` import GREEN and locked installed-source type/build preparation at `/var/tmp/rom-auth-installed-root-20261007/result.json`. That result has browserfalse and admittedfalse. This worker did not rerun that fixture. Legacy helper integration and actual auth/browser acceptance remain pending.

## Legacy integration design boundary

A source-only follow-up reviewed `App.svelte`, legacy `application/auth.ts`, application controller disconnect and existing auth tests. Shared protocol extraction must preserve anonymous generation, SessionExpiredError, stale Session changed errors, providers and login URL. It must not clone the parser or invent anonymous generation. The proposed private transport retains validated protocol metadata; public lifecycle state remains sanitized.

Thin delegation cannot establish App sequencing: current App disconnects on every refresh error or changed generation, and controller disconnect clears pending mutations. A trusted host profile and separately reviewed pause/rebind or R4 adoption are prerequisites for same-principal recovery. The detailed proposal and regression matrix are `/var/tmp/rom-auth-legacy-integration-plan.md`. That design-only review changed no legacy/App/controller source.

The coordinator corrected its installed snapshot chronology: actual hashes match all corrected frozen auth implementation files. It reports separate 12-case Chromium/WebKit runs using actual WebKit 2359 and 2364. These are authoring-source fabricated-driver cases, not real session-server or release admission. See [independent auth review](rom-0.1.0-auth-lifecycle-review.md).

## Public R4 session binding recipe (design)

This host recipe uses actual admitted public functions. It is a design example, not a newly executed browser journey. The host supplies authority, target, pending-intent store, version generator and UI clearing. It must also guard its own async view callbacks.

```ts
import { createBrowserSessionDriver, createSessionLifecycle } from 'rom-studio/auth';
import { createClient } from 'rom-studio/client';
import { createMutationRecovery, type PendingIntentStore } from 'rom-studio/recovery';

interface SessionRecoveryHost {
  studioBase: string; apiBase: string; authority: string;
  namespace: string; slot: string; target: { kind: string; id: string };
  pendingIntentStore: PendingIntentStore; fetch?: typeof fetch;
  now(): number; newVersion(): string;
  clearPrivateDisclosureAndCancelCallbacks(kind: 'renewed' | 'changed' | 'cleared'): void;
}
function composeSessionRecovery(host: SessionRecoveryHost) {
  const driver = createBrowserSessionDriver({
    base: host.studioBase,
    authority: host.authority,
    now: host.now,
    fetch: host.fetch,
  });
  const clientOptions = { base: host.apiBase, csrf: driver.csrf, fetch: host.fetch };
  let client = createClient(clientOptions);
  let viewEpoch = 0;
  const recovery = createMutationRecovery({
    namespace: host.namespace,
    slot: host.slot,
    target: host.target,
    binding: { client, principal: null },
    store: host.pendingIntentStore,
    newVersion: host.newVersion,
  });
  const lifecycle = createSessionLifecycle({
    driver,
    now: host.now,
    onTransition: async ({ kind, next }) => {
      if (kind === 'transient') return; // UI subscribes to structured lifecycle state.
      const ticket = ++viewEpoch;
      client.invalidateSession();
      host.clearPrivateDisclosureAndCancelCallbacks(kind);
      if (ticket !== viewEpoch) return;
      client = createClient(clientOptions);
      await recovery.rebind({ client, principal: next?.principal ?? null });
      if (ticket !== viewEpoch) return;
      // Resume current-authority views separately; retain exact accepted mutation intent.
    },
  });
  return { driver, lifecycle, recovery, currentClient: () => client };
}
```

The example clears private result/label disclosure on renewal and changes, while R4 retains same-principal drafts and accepted wire identity. The host clearing callback preserves same-owner invalid editor drafts on renewed; it clears them on changed/cleared. A consumer may preserve same-principal layout and selection without preserving unauthorized labels. Initial authenticated binding does not automatically load storage. Offer an explicit owner restore action that verifies current lifecycle identity and invokes `recovery.restore()`. On principal change, quarantine; do not restore a previous owner. A transient status keeps only still-valid authority until its original expiry. The lifecycle's expiry transition rebinds to null.

Use `recovery.stage(operation)` for later valid candidates. Keep invalid editor text in a separate host draft store. `recovery.retry()` uses the retained exact invocation; do not generate another idempotency key while its outcome is unknown. A store CAS success must acknowledge host-selected durability. Never store cookies, CSRF or generation. The host hook must settle and must not await its own lifecycle refresh promise.

## Shared legacy transport candidate

The coordinator approved only protocol/transport extraction and thin legacy delegation. `application/auth.ts` is now a facade. A private browser protocol retains exact anonymous generation and expired/changed outcomes. Public driver and legacy adapter use one parser and request implementation. Providers also use the shared bounded request and validation. The public constructors/types and 24 lifecycle tests remain unchanged. App/controller/profile are unchanged.

Shared acquisition cancellation is per waiter. Cancelling one waiter leaves other callers active. Cancelling all waiters aborts the driver; the deadline also bounds an ignored signal. Each consumer receives a copied session. Logout clears the shared flight before starting replacement checks. Both adapters guard the await boundary so an intervening logout cannot publish a staged authenticated result. The old helper now checks original expiry when CSRF is read, rather than exposing an expired token until another refresh.

Executed frozen candidate results: 14 new legacy/transport cases, 24 public lifecycle cases and seven existing legacy cases all passed (45 targeted). Full Studio units passed 222 tests; svelte-check reported zero errors and warnings. Exact source/lock hashes and RED/GREEN logs are `/var/tmp/rom-auth-lifecycle-evidence/legacy-evidence.json`. The first shared-flight attempt broke held-response request timing; its failed log remains preserved. Later adapter-boundary races were reproduced before their epoch guards.

Independent review and installed-source/browser acceptance for this changed candidate remain pending. Earlier installed hashes cover the prior candidate, not this extraction. No App behavior, real server auth journey, human usability or intake closure follows from these unit results.

## Selected actual App integration design

The coordinator independently passed the frozen legacy candidate: 45 targeted tests, 222 full units and clean type checks. A new installed authoring consumer passed 12 actual Chromium/WebKit 2359 cases. Those cases still exercise the adapter/fabricated lifecycle rather than App session recovery. Root evidence is `/var/tmp/rom-auth-legacy-root-review/result.json` and `/var/tmp/rom-auth-legacy-installed-root-20261007/result.json`; this worker did not rerun them.

The proposed next increment is `/var/tmp/rom-app-session-recovery-design.md`. It preserves original AP-UX-004/005/006 acceptance and lists seven actual installed App scenarios. Managed mode requires trusted authority and explicit intent/editor stores, namespace, principal-target slots and version/command/epoch factories. It adds controller pause/rebind fences, one existing R4 active-target lane, separate exact editor draft persistence, principal-aware mount identity and editing distinct from dispatch. No second receipt repository or implicit persistence is selected.

App/controller/form source remains unchanged pending review. Root owns main.ts, trusted identity/bootstrap and shared manifests. A legacy default that silently bypasses managed profile cannot satisfy the full root-consumer gate. All other feedback groups, including application-specific and withdrawn items, retain their explicit mapping and remaining acceptance.
