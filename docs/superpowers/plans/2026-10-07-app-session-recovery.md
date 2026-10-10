# App Session Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve actual App drafts, selected context and exact unknown mutations across transient sessions and same-principal renewal.

**Architecture:** Use one existing R4 lane for the active target. Separate session request fences from editor CAS persistence and App rendering. Host-selected full profile supplies authority and stores.

**Tech Stack:** Existing ROM client/recovery/auth, TypeScript, Svelte and existing controls; SQLite/redb and actual Chromium/WebKit acceptance.

**Spec:** Reviewed narrow design below, derived from [UI composition admission](../../research/rom-0.1.0-ui-composition-admission.md).

## Global Constraints

- Root approved Controller/R4 then App/editor increments sequentially on 2026-10-07.
- Root owns main.ts, trusted bootstrap/configuration, store exports, shared manifests, Rust fixtures and native scheduling.
- Preserve public paths, legacy custom-client mode and existing auth modules unless a new coordinated RED justifies change.
- No implicit authority, namespace, global store or localStorage fallback.
- Editor CAS failures block unsafe navigation; invalid raw text is never submitted as wire input.
- Same-principal unchanged descriptors preserve form base revision and accepted wire identity.
- Restore requires explicit current owner and selected target; no automatic execution or previous-owner load.
- Seven actual installed App cases run in both actual engines for each adapter; unit tests do not close acceptance.
- Commands use bounded evidence directories; browser/native allocation requires coordination. No commits or deployment.

## Review Focus

- Same IDs under different principals must not reuse the prior editor subtree.
- A stale draft CAS must not enable navigation or permit a server dispatch.
- Fresh permission denial clears private projection without rewriting unknown commit knowledge.
- A later invalid draft must survive renewal without replacing the accepted operation.
- Hook/subscriber reentrancy must suppress old results, errors and finalizers.

## Execution tasks

### Task 1: Controller/R4 and editor store

- [x] Add a failing public-controller pause/rebind test at `studio/tests/unit/application-session-recovery.test.ts`.
- [x] Implement named session-types/session-binding modules and preserve controller API via reexports.
- [ ] Add RED cases for dropped acknowledgement, later draft, current generation, same-principal renewal and changed-principal quarantine.
- [x] Implement the one-target mutation-lane using existing createMutationRecovery only.
- [ ] Add RED editor CAS cases for invalid text, exact bigint codec, failure/stale writes, wrong owner/target and late principal change.
- [ ] Implement bounded editor-drafts with explicit host storage and current tickets.
- [ ] Run affected controller/legacy tests, full Studio unit suite and type check. Freeze hashes and hand off to root review.

### Task 2: Actual App and editor wiring

- [ ] Add RED cases for managed auth profile validation and controller session bridge.
- [ ] Wire app-session/App after Task 1 freeze; keep root bootstrap ownership separate.
- [ ] Add controlled form/FieldHost drafts and submitDisabled without changing uncontrolled usage.
- [ ] Add principal-aware editor keys and shared mutation/navigation projection.
- [ ] Add explicit owner/target restore and possible-commit discard UI.
- [ ] Run relevant units/type/component checks under allocated browsers. Preserve all failures and frozen source.

### Task 3: Installed real App acceptance

- [ ] Propose fixture package/lock graph to root before admission; use public source package imports.
- [ ] Add the seven reviewed cases below as failing actual App scenarios.
- [ ] Coordinate Rust HTTP fixture/native compile and frozen binary provenance through root.
- [ ] Run seven cases per engine per adapter, one worker, bounded commands, matching real WebKit.
- [ ] Record installed source, locks, binaries, engine paths, backend counts and limitations.
- [ ] Obtain independent review and full local verifier before integration. Update per-ID evidence without aggregate closure.

## Root approval and evidence ledger

Root approved the narrow profile, one R4 lane, separate editor CAS and pause/rebind design. Controller first, then App/editor, then real installed acceptance. At plan creation no implementation or new tests for this increment have passed. The preceding auth adapter candidate independently passed 45 targeted, 222 unit tests and installed authoring cases; it does not establish App recovery.

### Controller/editor candidate, 2026-10-07

The candidate has a managed fourth controller argument. The existing three-argument legacy mode remains supported. Named modules own session fences, one existing R4 lane and separate editor CAS. Managed snapshots are cloned; legacy snapshot behavior remains unchanged.

Sixteen targeted cases cover pause/rebind, paused dispatch, exact accepted-wire retry, later draft persistence, principal clearing, callback reentrancy and explicit restore tickets. Editor cases cover invalid text, bigint revisions, stale CAS, held and queued writes, malformed records and wrong owner/target. These are controller and store-boundary unit cases. They do not establish actual App behavior or browser persistence.

The corrected reentrant replacement case was verified with the post-clear ticket guard deliberately removed, then restored. The mutation run failed with `editor principal unavailable`. Initial failures from invalid test operation shape, missing fixture content type and reentrant fixture recursion remain in the evidence directory. They are not feature RED evidence.

Evidence directory: `/var/tmp/rom-app-session-evidence`. Final source, hashes, commands and terminal results are recorded in `controller-evidence.json`. App/form/bootstrap source remains unchanged at this candidate boundary. Tasks 2 and 3 remain open. AP-UX-004/005/006 retain their original actual-consumer acceptance; unit results do not close them. All other active groups retain their maintained intake dispositions. AP-UX-030/032 remain explicitly withdrawn.

### Independent boundary review corrections

Root reproduced a renewal classification defect in the first frozen candidate. Any query failure cleared selection and was followed by an active session publication. The original frozen source and its successful 40-test evidence remain intact. They do not establish transient renewal behavior.

The new RED log `renewal-review-red.log` has eight intended failures: discovery/query/read 503 or network failures erase context, and confirmed denied/missing responses retain private editor state. The corrected contract returns explicit `fresh`, `transient` or `denied` renewal outcomes. The public controller session status adds `denied`, as approved by root. Only fresh renewal enables mutation. Same-principal transient failure retains context and editor snapshots as stale. Successful renewal publishes discovery/query/read projections together.

Confirmed server categories `denied`/403 and `missing`/404 clear private projections. A private-view fence precedes callbacks and cancels old requests. It suppresses cached rows, selection, query values, work, editor text and recovery draft/result in later publications. Unknown commit knowledge and its original stored command remain unchanged. Local staging and retry remain disabled while denied. Fresh authorized renewal can reopen the explicit restore gate; it does not automatically load a persisted owner draft.

Additional intended RED probes found a replacement-lane race, outstanding-write restore, reentrant draft write ordering and stale restore publication. A selected lane and subscription now retain their captured ticket. Editor writes enter the serialized queue before callback publication. Restore rejects while any draft write remains outstanding, including after same-principal rebind. A restore read cannot publish over a newer staged draft.

`publication-review-red-2.log`, `editor-reentrancy-review-red.log` and `editor-rebind-review-red.log` retain these failures. The first publication probe waited for a restore before releasing its held write; that test harness cancellation is preserved in `publication-review-red.log` and is not counted as an intended feature RED.

Revised source and terminal checks are recorded separately in `controller-review-evidence.json`. Actual App, browser, database-adapter and consumer acceptance remain open. AP-UX-004/005/006 are still linked to the seven actual App cases below; these boundary corrections do not close their intake items.

The later snapshot probe reproduced loss of decimal lexical metadata through `structuredClone`. Its RED log is `snapshot-decimal-review-red.log`. Root approved the new `application/snapshot.ts` seam. Only explicit wire subtrees traverse the existing bounded ROM codec. Controller pages, limits and descriptor versions keep their TypeScript number types. Draft stage, restore and publication preserve `1.0`, `1e0`, `-0.0` and large integers; queued editor writes retain these categories after reopening.

The revised candidate also clears prior global error text while denied. `denial-error-review-red.log` retains the two intended failures. It introduces no new transport codec or storage default. Previous 53-targeted/253-unit results remain historical snapshots; the final corrected hashes and commands are in the revised evidence record.

The next metadata review reproduced obsolete descriptor disclosure after confirmed discovery or selected-read denial. `descriptor-denial-review-red.log` retains both intended failures. While denial persists, the controller now conservatively hides all descriptor metadata and nulls legacy pending state. Managed mutations already use the R4 lane and do not assign legacy pending results. The original unknown intent remains unchanged. Subsequent pause/publication cannot expose removed metadata. This candidate is recorded separately in `controller-metadata-evidence.json`; prior frozen evidence remains intact.

## Reviewed design and acceptance


Date: 2026-10-07. Status: reviewed and approved for sequential implementation; tasks/evidence above track progress.

Target: AP-UX-004, AP-UX-005 and AP-UX-006 through the actual installed public App. Shared auth tests establish infrastructure, not these workflows.

## Source facts and retained acceptance

App currently calls `controller.disconnect()` on every refresh exception and changed generation. Disconnect clears selection, query history and pending mutation. The controller revalidation callback bypasses App's `checking` flag. ResourceDetails is keyed only by Resource ID. Its ResourceForm is read-only while a mutation is unknown. Invalid field text resides in FieldHost/ValueEditor draft state; valid form intents reside in ResourceForm. A same-principal reconnect that unmounts these editors loses both.

Keep original intake acceptance unchanged:

| ID | Original acceptance | Actual App gap |
| --- | --- | --- |
| AP-UX-004 | Lost acknowledgement retries same command; input changes wait; confirmed rejection releases it; explicit unknown/conflict/rejected states; permission changes cannot disclose prior results. | Pending state is client-bound and discarded by disconnect. Unknown state locks editing rather than retaining a later draft. |
| AP-UX-005 | New/switch waits for save confirmation; uncertain save/delete retains current view; same IDs across sessions render selected content; authority changes discard private state. | Disconnect removes current view/pending barrier. UI key lacks principal. Navigation must consume R4 knowledge and editor durability, not only busy state. |
| AP-UX-006 | Same-principal refresh preserves view; confirmed expiry clears private state; transient failures retain context and dismiss after recovery; checks share a single flight. | Auth transport is shared, but App still disconnects after transient failures/generation renewal. |

“Confirmed rejection releases it” applies to a first established not-committed attempt. A refusal after earlier unknown outcome must not erase uncertainty. R4 already implements that distinction.

## Managed App profile and compatibility

Add optional `authProfile` to App props. A supplied profile selects managed mode and must include recovery configuration. Missing pieces are a configuration error; do not silently enter legacy mode. Preserve the existing custom `client` prop and legacy mode for consumers that omit authProfile. Document that legacy mode does not provide managed durable session recovery. It cannot satisfy the full root-consumer gate.

```ts
interface ResourceTarget { kind: string; id: string }
interface StudioAuthProfile {
  authority: string;
  now(): number; // UTC seconds; pure clock
  recovery: {
    namespace: string;
    intentStore: PendingIntentStore;
    editorStore: PendingIntentStore;
    slot(principal: DurablePrincipal, target: ResourceTarget,
         record: 'intent' | 'editor'): string;
    newVersion(): string;
    newCommandKey(): string;
    retryEpoch(): bigint;
    maxBytes: number;
  };
}
```

Use existing public `/auth`, `/client` and `/recovery` types. Export StudioAuthProfile through the existing root facade after coordinator admission; do not broaden the transport-only entries. The profile is host composition, not a server credential container. Slot/namespace/version inputs exclude credentials and session generation. Slots distinguish principal and target. Store compareExchange success acknowledges host-selected durability. No global store, default localStorage, generated authority or URL-derived identity is added.

Use the supplied custom client when present. Managed mode calls its existing invalidateSession/prepare/submit interfaces; it never reaches into a producer client WeakMap. The supplied client must obey the documented generation/current-credential contract. Its network credentials remain host-owned. Without a custom client, App creates its normal client with the explicit browser driver's csrf callback.

The standard `main.ts` must select managed mode explicitly for full root admission. Proposed host-owned `studio-profile.json` contains non-secret authority, namespace, slot policy, retry epoch and an explicit IndexedDB storage selection. Existing rom-studio-host asset serving already accepts JSON files. A bounded bootstrap parser builds the function-bearing profile. Missing/invalid configuration shows a configuration error; it must not silently fall back. The host writes that file; ROM does not manufacture authority/database names. Package controls/client use remains independent of this bootstrap.

The coordinator owns main.ts, bootstrap configuration, shared manifests and deployment identity namespace selection. This proposal does not grant that ownership. Investigate the backend identity namespace before selecting a stable authority. If the coordinator chooses another host injection mechanism, preserve the same required profile and acceptance. An optional profile whose default bundled entry remains legacy is insufficient.

## Controller contracts and small modules

Preserve `createApplication(client, key?, revalidate?)`. Add an optional fourth configuration argument. Keep the existing facade/types path through reexports.

Proposed new `application/session-types.ts` defines:

```ts
interface ManagedApplicationOptions { recovery: StudioAuthProfile['recovery'] }
interface ApplicationBinding { client: RomClient; principal: DurablePrincipal | null }
interface ApplicationSessionState {
  status: 'legacy' | 'active' | 'transient' | 'renewing' | 'cleared';
  stale: boolean;
  mutationAllowed: boolean;
}
```

Add controller operations:

- `pauseSession(reason: 'transient' | 'renewing'): void`: advance request/navigation/selection tickets; abort requests, reference lookups and live readers. Retain kind, query/history, selected identity, owner drafts and unresolved lane. Mark view stale. Do not call disconnect or abandon the accepted invocation.
- `rebindSession(binding: ApplicationBinding): Promise<void>`: invalidate old transport; update generation observation; rebind the existing R4 lane. Same durable principal retains accepted intent and owner drafts, clears stale result/error/reference labels, then rediscovers/requeries under current authority. Retain selected target and history only when still disclosed. Different/null principal quarantines the lane and clears all private view/editor state before notification. No automatic restore of an owner record.
- `restoreSelectedIntent(): Promise<void>`: explicit current-owner action after selection and authentication. Calls R4 restore and editor restoration; checks tickets/principal at every await. It does not execute a command automatically.
- `discardSelectedIntent({ acknowledgePossibleCommit }): Promise<void>`: delegates existing R4 discard; does not bypass its possible-commit acknowledgement.

Implement these in `application/session-binding.ts`, not a second all-purpose controller. It owns lifecycle/request fences and calls existing query/selection operations through narrow callbacks. It contains no persistence or invocation codec implementation.

`application/mutation-lane.ts` owns one active-target R4 controller. Creating a different target requires the current lane to permit navigation. The lane uses createMutationRecovery with the explicit host slot/store and current principal. It owns no receipt repository or process-global target map.

Managed `mutate(id, expected, operation)` keeps its existing call signature. Stage the operation, allocate a key only for a new permitted command, call begin with exact expected/key/retryEpoch, then retry through the helper. `retry()` calls recovery.retry without generating a key. The helper uses exact ROM wire encoding and prepares the original invocation under the current client. Mutation results update the view only after current authority/tickets are checked.

Legacy `state.pending` remains unchanged for legacy mode. Add managed `state.recovery: { target: ResourceTarget; state: MutationRecoveryState } | null`; do not fabricate a PendingMutation or maintain a parallel accepted-invocation copy. App/ResourcePage consume a shared notice/navigation projection from this state. Render explicit knowledge: unknown is not rejection, storage_error is not confirmed dispatch, conflict is distinct from permission refusal.

New `presentation/mutation-navigation.ts` projects the shared guard. Busy draft persistence, unresolved accepted intent and storage uncertainty block New/Switch/delete navigation. Confirmed first rejection can release the lane. Deleting with a lost response keeps the selected view until current receipt recovery. Navigation attempts retain focus and draft; no silently discarded form.

## Editor identity and persistence

Keep editors mounted during same-principal transient failure/renewal. Keep App page and ResourcePage kind identity stable. Update read-only/current-value projections without resetting the form's original revision or intents. A stale descriptor/revision disables new command submission but does not erase edits. Same-principal refresh does not increment an editor reset key.

Key owner-sensitive ResourceDetails/Settings editors by durable principal + Resource kind + ID. Changed/null principal resets the mount identity before exposing another owner. Identical IDs must never reuse the previous principal's editor subtree.

Separate field editing from dispatch. Add `submitDisabled` to ResourceForm/ActionForm, while readonly means authority/descriptor forbids editing. During accepted mutation uncertainty, allow later edits but disable a fresh submit. A valid whole-form candidate calls `stageDraft(operation)` on the existing lane; it must not mutate the accepted wire. Invalid raw text is never a wire operation.

Add optional controlled editor draft callbacks to ResourceForm/ActionForm and FieldHost, preserving their existing uncontrolled API. Reuse existing EditorDraft/renderer draft callbacks. New `application/editor-drafts.ts` owns bounded, exact ROM-codec snapshots for intents, raw editor text, private row identities, validation state, base revision and descriptor identity. It stores editor snapshots through the explicitly supplied editorStore; it is not a mutation store.

Reject unsupported custom renderer state rather than truncate/JSON-convert it. Report a structured draft persistence failure and block unsafe navigation. Persisted entries carry principal/target/descriptor identity and exclude credentials and resolved reference labels. Clear private display before principal switch. Restoration is explicit and current-owner checked. A changed descriptor preserves raw text as a quarantined draft; do not resubmit against a new codec without review.

A valid later operation is also R4 staged draft wire. Invalid edits remain in the separate editor record. A failed editor/store CAS must not unlock navigation or claim durability. A server receipt/result is independent of the latest editor snapshot.

## App sequencing

New `application/app-session.ts` connects the public lifecycle to the controller. Startup, periodic Check, explicit Check and controller revalidation share its refresh promise. Current check publication clears structured transient notices after recovery. Transient failures retain valid context only until the original expiry; a deadline never becomes an extension.

On same principal/new generation, fence old network work and pause, then rebind without disconnecting/remounting the editor. On changed/null principal, clear/quarantine before provider/render callbacks. Provider UI and redirects have App view tickets; destroyed or superseded provider requests cannot redirect or restore login UI. Hooks may await R4 rebinding but must settle and must not await their own lifecycle refresh.

Work and attachment pages have separate pending operations. Pause/invalidation must stop their private network callbacks; it must not label a cancelled server mutation as rolled back. Their durable command contracts stay unchanged. Scope their pending/navigation state to the same principal reset key. Dedicated work/upload recovery remains a separate gate if those paths cannot reuse the existing guarantee.

## Exact proposed ownership

No listed edit is granted until review. Split source ownership before execution:

| Increment | Proposed files | Responsibility |
| --- | --- | --- |
| Controller/R4 worker | `application/controller.ts`, NEW `session-types.ts`, `session-binding.ts`, `mutation-lane.ts`, `editor-drafts.ts`; NEW `tests/unit/application-session-recovery.test.ts` | Generic request fences, one R4 lane, exact editor persistence and knowledge projection. |
| App/editor worker | `src/App.svelte`, NEW `application/app-session.ts`; `ResourcePage.svelte`, `ResourceDetails.svelte`, `SettingsPage.svelte`, `resources/ResourceForm.svelte`, `ActionForm.svelte`, `renderers/FieldHost.svelte`; NEW `presentation/mutation-navigation.ts` and unit cases | Actual profile/lifecycle wiring, mounted editor identity and navigation/focus behavior. |
| Coordinator-owned bootstrap/store | `src/main.ts`, proposed NEW `application/bootstrap-profile.ts`, any proposed public IndexedDB adapter/export after separate approval | Trusted deployment configuration and explicit store selection. No storage default in helpers; first App fixture supplies host stores. |
| Installed fixture worker | NEW `studio/tests/app-session-recovery/**`; narrow NEW recovery-host authority/proxy endpoints if required | Real App, HTTP/session authority, lost/held responses and backend counts. No private source aliases. |
| Coordinator | Package export/root-index additions, fixture manifest/lock/bootstrap asset admission; Rust fixture/native scheduling | Shared ownership and exact release-source provenance. |

The two source increments share no file edits. App worker reads controller interface after its reviewed freeze. Existing auth modules remain frozen unless new evidence requires a coordinated correction. Use module facades and existing helpers; do not put this work in one large controller.

## Seven actual installed App acceptance cases

Import App from the package root, controls/styles/client/auth/recovery through supported public paths. Configure explicit managed profile/store. Serve real HTTP session endpoints with disposable fixture authority and the current-authority ROM API. No production OIDC/credential certification is inferred. No fabricated SessionDriver-only assertion can substitute.

Run all seven in Chromium and matching actual WebKit, on SQLite and redb. Reserve disposable ports/output/native allocation through the coordinator. Workers one; each command has 180-second/8-MiB budgets. Record exact installed/compiled source, lock, binary and engine versions. Authoring copies remain candidate evidence.

| Case | Setup and injected fault | Required visible and durable result | IDs |
| --- | --- | --- | --- |
| 1. Concurrent check | Open actual App; hold real auth session response while startup/Check/controller revalidation request it. | One acquisition; current session connects once. Provider failure cannot replace authenticated page. | 006 |
| 2. Transient outage | Select a Resource, page/filter history and a valid plus invalid form draft. Return real temporary 503/network failure before original expiry; recover. | Same selection/history/editor text/focus; stale/dismissible status; no disconnect/new mutation; status clears on recovery. Expiry never extends. | 004,005,006 |
| 3. Lost acknowledgement + renewal | Commit save and drop every acknowledgement for its key. Make later edits; renew same principal with new generation/CSRF/current authority stamp. | Unknown notice and navigation barrier survive; original wire/expected/key remain identical on retry; later draft is distinct; one receipt/event/effect; current result displayed only under current authority. | 004,005,006 |
| 4. Uncertain delete/navigation | Commit delete and suppress ACK; click New, another row, Settings and page/back navigation. Restore ACK and retry. | Selected view, draft and opener focus remain until confirmation; same delete command; no second delete effect. Confirmed not-committed versus earlier unknown are distinct. | 004,005 |
| 5. Principal/permission switch | Hold old read/reference/mutation response; switch to another principal with identical kind/ID, or revoke disclosure. | Private values/labels/errors disappear before new-owner render; lane quarantines; late result cannot populate it; no automatic old-owner restore. | 004,005,006,020,021 |
| 6. Reload and explicit restore | Reload actual installed App after lost save acknowledgement with later valid/invalid edits persisted through host stores. Authenticate current owner; choose explicit restore. | Restored selected target and raw drafts; accepted original command recovered, no fresh key; retry under current session returns one committed receipt. Wrong principal cannot restore it. | 004,005,006 |
| 7. Expiry/logout/destruction | Let validated expiry pass during a transient outage; separately hold Check/provider/logout responses while logout/replacement/unmount occurs. | Private UI/editor/result clears at expiry; captured logout token posts before local invalidation; stale provider cannot redirect; late errors/results do not overwrite current authority. | 005,006,021 |

Tests must inspect the actual App and compare independent backend receipt/event/effect counts. A mock projection or isolated checkbox test does not pass these cases. Exercise missing/null/false/zero and an integer above Number.MAX_SAFE_INTEGER in accepted save wire. Preserve failed artifacts.

## Every feedback group remains traceable

| Groups | This increment's relationship and remaining consumer acceptance |
| --- | --- |
| 001–003 | Existing public controls baseline; full package assets, exact form recipes and composer geometry remain separate. |
| 004–006 | Seven cases above are selected; not yet implemented or executed. |
| 007–008 | Work/queue progress belongs to durable work acceptance; session fencing cannot fabricate stages or restart accepted work. |
| 009–011 | Long results/composer, follow/manual scroll and panels/focus/reduced motion remain selected UI composition acceptance. This increment's focus retention is only a narrow intersection. |
| 012–013 | Locale/error resolver and bounded observation remain separate. Auth categories and stale status alone do not establish them. |
| 014–016 | Domain conversation/grounding/research/tool meaning remains consumer-owned; generic provenance/latest guards require their own tests. |
| 017–018 | Whole-card selection/latest calculation and widget layout/drawer remain UI composition work. |
| 019 | Domain terminology/SVG IDs stay consumer-owned; generic translated spacing remains untested. |
| 020–023 | Reference picker disclosure, capability nav, history and source/detail routing remain specific acceptance. Cases 5/7 cover only authority clearing. |
| 024–025 | Fresh execution provenance and exact selected snapshot/export remain separate work. Recovered mutation identity is not fresh calculation or PDF output. |
| 026–029 | Profile/provider policy/brand/daily relationships stay domain-owned; generic scope, motion and temporal request guards remain visible candidates. |
| 030 | Explicitly withdrawn video request; excluded, not resolved by App work. |
| 031 | Consumer date/timeline calculation stays domain-owned. Generic exact selected-date/latest-result/export identity still needs acceptance. |
| 032 | Explicitly withdrawn music/sound; excluded, not resolved by App work. |

Source thread coverage remains 32 canonical groups. The coordinator reports 36 user references and an unchanged thread marker. Neither shared-file handoff nor this report establishes consumer acknowledgement. Source text truncation/visual limits remain unchanged.

Prior reviewed legacy evidence (45 targeted, 222 units, clean type check and root-reported installed 12 engine cases) concerns the adapter candidate. It does not pass any of these actual App workflows. No feedback status changes here.

### App contract admission in progress

The first clean installed source fixture uses the public App, `StudioAuthProfile`, `rom-studio/recovery` and package styles. Its frozen consumer dependency graph matches the reviewed public-controls fixture except for its package name. A locked npm ci installed physical source files inside the consumer; no producer source aliases or linked checkout are used.

`/var/tmp/rom-app-installed-contract-red-WjjXlu/type-check-corrected.log` records the intended single failure: App rejects `authProfile`. The initial log also exposed a missing fixture CSS ambient declaration; that fixture issue was corrected before the intended RED. A wrong-working-directory check was preserved separately and is not feature evidence.

The new `application/app-session.ts` bridge validates explicit host configuration and composes the existing browser driver, lifecycle and managed controller. Two bridge unit cases first failed for the missing module and then passed with the prior 33 controller cases. They cover shared refresh and transient/same-owner renewal transport behavior. They do not establish actual App draft behavior.

App now accepts the reviewed profile and preserves the legacy custom-client path. The installed preparation at `/var/tmp/rom-app-installed-contract-green-20261007/result.json` passed locked installation, type checking and bundling. It is an authoring checkout with browser execution false and admission false. Controlled forms, full private subtree boundaries and seven runtime scenarios remain in progress.

The initial actual App form RED uses the retained historical host binary and its original compile provenance only. Root authorized this bounded authoring experiment; it cannot establish current full-source compilation or release acceptance. Final acceptance requires a fresh candidate host and seven cases in Chromium and locked WebKit 2359 for SQLite and redb. The synthetic loopback auth proxy is not production provider identity admission.

The corrected actual App persistence RED is `/var/tmp/rom-app-form-runtime-red-3-20261007/browser.log`. Both Chromium and WebKit 2359 reached a real committed patch, independently confirmed its receipt, lost its acknowledgement, edited invalid count text, reloaded and selected the explicit owner target. Restore showed the stored count instead of the invalid text. The source and fixture fingerprints and fresh narrow-metadata host's authoring provenance remain retained. This is an intended feature RED, not release acceptance.

Earlier attempts retain distinct limits: the historical host hid descriptors; the first fresh-host test selected an ARIA field group rather than its textbox; an epoch-one fixture profile was rejected because the disposable database was at epoch zero. None is counted as persistence RED. The corrected profile explicitly chooses epoch zero for this fixture; production epoch configuration remains host-owned.

Root approved an additive controller `createDraftWriter` seam. Its closure captures the current durable owner, kind and selected target. It checks the captured context before editor CAS, after CAS, and after staging in the existing R4 lane. Six intended missing-API RED cases then passed with the prior 35 managed/bridge cases. These include same-owner offline/renewed editing, changed principal, null principal, confirmed denial, target navigation and a principal switch triggered by the saved-editor publication.

The controlled form candidate exposed two actual Svelte integration failures before reaching GREEN. Binding undefined entries to a fallback bindable caused `props_invalid_value`; owned field drafts now initialize explicitly. A persistence effect tracked cloned descriptor objects, producing repeated writes; normalization uses the immutable mounted definition without making controller publications its write trigger. Browser error attachments preserve these failures. A further focused RED records the safe-integer revision type loss: `copyEditorSnapshot` turned typed `1n` into number `1`, disabling Save on an unchanged resource. Its correction is under root review.

Root approved bounded frame-set composition inside the existing EditorSnapshot contract. It retains each simultaneously visible resource/action draft under one editor CAS record, with global byte, frame, child and depth bounds. Exact accepted commands remain in the separate R4 lane. Pure merge/identity/bounds tests are in progress; ActionForm and simultaneous callback acceptance remain open.

The actual fixture now authors all seven original cases, linked explicitly to AP-UX-004/005/006 and AP-UX-021. Diagnostic `--scenario` execution runs a single case per engine and leaves completion false. Runtime budgets currently cap at 600000 milliseconds per adapter, matching the disposable host. `--admit` fails closed until a root-reviewed complete native source witness is integrated. Full seven-case SQLite/redb runs, final installed source admission, main/bootstrap deployment wiring and original consumer reproduction remain open. AP-UX-018 dashboard/settings/mobile/keyboard/reload acceptance and all other active intake groups remain visible in the maintained UI composition plan; withdrawn AP-UX-030/032 stay withdrawn.

The safe-integer revision correction passed its intended RED and affected checks. Typed editor `baseRevision` remains bigint; explicit wire subtrees retain decimal lexical categories. Nested frame parsing validates and normalizes its own unsigned revision fields without changing user field values. The bounded frame contract now has 12 pure tests, including simultaneous resource/action retention, definition/target/base mismatch, extra frames, global bounds, large integers, decimal categories and malformed floating revision tokens.

Three updater cases first failed, then passed: merge against the current editor cache during concurrent callbacks; reject an updater that changes principal synchronously; retain a navigation barrier when composition fails. Together with controller/session/bridge and pure form cases, the affected suite passed 56 tests. The current full Studio unit suite passed 287 tests at `/var/tmp/rom-app-session-evidence/app-forms-unit-current.log`; Svelte checking passed at `frame-forms-check-3.log`. These checks establish authoring behavior only.

The original installed reload case then passed in Chromium and WebKit 2359 on SQLite at `/var/tmp/rom-app-form-runtime-green-5-20261007/result.json`. This diagnostic executed one case per engine. Its real patch receipt, dropped acknowledgement, invalid resource input, reload and explicit restore evidence remains separate from the full seven-case matrix. Completion and release admission remain false.

The App candidate now validates the full bounded frame set before signalling explicit restore. Resource and action forms merge their own leaf through the captured controller draft writer's synchronous updater; neither uses a stale aggregate closure. Same-principal renewal retains the mounted form and original revision. Changed principal clears the private subtree. Invalid editor text stays separate from the accepted command in the existing R4 lane.

The installed fixture now consumes public `createStudioBootstrap` with explicit authority, namespace, epoch and separate intent/editor database names. A separately named maintained-main startup diagnostic bundles the physical installed `src/main.ts` entry without private aliases or vendor patches. It checks missing, duplicate, wrong-type and invalid JSON profile scripts, and unavailable IndexedDB, for a generic error before App mount. This preview-entry check is distinct from public package API admission. The original seven cases remain unchanged in purpose; complete runtime execution expects seven recovery cases plus this diagnostic per engine and adapter.

Simultaneous resource/action reload browser verification is in progress. `/var/tmp/rom-app-frames-bootstrap-runtime-20261007` failed because the fixture attempted a native-select command on a Bits combobox; this is not counted as a feature RED. A corrected click/option run is retained separately. Full SQLite/redb execution, complete frozen-source native provenance, release admission and original Astral Plane reproduction remain open. Creation-form draft persistence and future composition groups remain explicit follow-up gaps; selected-resource recovery does not establish those paths.

The first complete SQLite authoring matrix passed 14 executions and failed both expiry/relogin executions. Evidence is `/var/tmp/rom-app-full-sqlite-authoring-20261007/browser.log`. After a draft-only edit, owner clearing left the volatile lane quarantined. Its navigation guard prevented fresh discovery from completing. Four new unit cases first failed at the prior-owner recovery projection. The intended RED is `owner-reacquisition-red-2.log`; the earlier log without the new tests is not a RED.

Root approved a narrow owner-scoped quarantine correction. The mutation lane records its original durable owner. Quarantined subscriptions hide the old target and recovery projection. Quarantine does not prevent current-owner discovery or selection. A same-target lane can be reused only for its original owner. Explicit same-owner restore can reopen the original slot. A different owner constructs only that owner's slot. Current live unresolved intent and storage errors still block destructive navigation. Durable records, accepted identity and pending CAS tickets remain unchanged.

The revised affected suite passed 61 tests. This includes the four reacquisition cases and a held-CAS probe. The latter confirms that a completed old-owner write cannot republish private text after another owner's acquisition. Store read spies confirm that another owner's explicit restore reads only that owner's slots. The full unit suite passed 292 tests and Svelte checking reported zero errors. Frozen source hashes are retained in `/var/tmp/rom-app-session-evidence/app-forms-owner-reacquisition-frozen.json`.

The next SQLite run passed expiry/relogin and accepted-unknown logout recovery in both engines. Two failures came from the strengthened fixture comparing global database counts before and after its explicit Bob setup mutation. The corrected fixture retains the unchanged-count assertion before setup. It then checks the new baseline, original receipt/event identity and absence of old-owner replay. This fixture failure is preserved at `/var/tmp/rom-app-full-sqlite-authoring-2-20261007/browser.log`.

The complete corrected SQLite matrix passed 16 executions at `/var/tmp/rom-app-full-sqlite-authoring-3-20261007/result.json`. Each engine ran the original seven recovery cases and the separately named maintained-main startup diagnostic. The source was physically installed with locked npm dependencies. Chromium and WebKit 2359 executed the tests. Public bootstrap creation, simultaneous invalid resource/action drafts, explicit reload restore, exact accepted replay, principal separation and expiry/logout recovery passed under the synthetic fixture authority. Admission remains false and source mode is `authoring_checkout`. Complete redb execution is in progress.

The matching redb matrix passed 16 executions at `/var/tmp/rom-app-full-redb-authoring-20261007/result.json`. The two adapter runs used identical installed producer source, fixture source, consumer lock and native binary. The comparison is retained in `/var/tmp/rom-app-session-evidence/app-matrix-authoring.json`. Total executed acceptance is 28 original recovery cases and four maintained-main diagnostics. Both reports retain admission false. The declared actual paths were Chromium `/root/.nix-profile/bin/chromium` and locked WebKit `/var/tmp/rom-studio-webkit-2359/pw_run.sh`. Per-session browser version attachments are not yet recorded by this fixture.

The owned App/controller/form candidate remains frozen for independent review. The full local verifier, complete release-source/native witness, extracted release-source runtime admission and original Astral Plane reproduction remain coordinator gates. Creation-form persistence and the remaining UI composition candidates are not implemented by these selected-resource tests.

The supported authoring command is:

```sh
ROM_WEBKIT_EXECUTABLE=/var/tmp/rom-studio-webkit-2359/pw_run.sh \
ROM_CHROMIUM_PATH=/root/.nix-profile/bin/chromium \
node studio/tests/app-session-recovery/verify.mjs \
  --output /var/tmp/rom-app-review-unique \
  --host /absolute/path/to/rom-recovery-host \
  --host-provenance /absolute/path/to/authoring-provenance.json \
  --adapter sqlite --port 43282 --budget-ms 600000
```

Select `redb` and port `43283` for the other adapter. Use a unique output directory. `--source-dir` and `--source-archive` select supplied producer source; they are mutually exclusive. `--prepare-only` skips runtime. `--scenario` is diagnostic and requires one case per engine; completion remains false. `--admit` fails closed until complete native-source admission is reviewed. Install/check/build commands have 180-second and 8-MiB output bounds. Browser runtime is bounded by `--budget-ms`, at most 600000 per adapter, with one worker and 60-second test deadlines. The generated fixture resides entirely below the reserved output directory. Failed evidence remains retained.
