# Independent Studio creation review

Review date: 2026-10-07. This review covers the creation candidate in `/root/ROM`.
The reviewer owns this report only. The release coordinator owns product changes and integration.

## Result

The review found two reproducible P2 defects: descriptor-mismatch discard and matching-draft cleanup after restart.
The coordinator corrected the descriptor defect during review. Matching-draft cleanup remains open.
Final checks must cover the changed source.
No release admission is established by this review.

The subsequent proof correction passed 27 independently rerun focused tests.
It corrects matching cleanup after restart, but review exposed one nested-editor association defect described below.

The reviewed command path freezes the target, expected revision, command key, retry epoch, and accepted wire before dispatch.
Explicit restoration does not dispatch a request. Explicit retry uses the accepted command and the current binding.
Later invalid drafts have separate storage and survive recovery of the earlier command.

## Actionable finding and correction

**P2: descriptor mismatch prevents explicit local discard.**

Initial location: `/root/ROM/studio/src/lib/recovery/record-storage.ts:31`.
Related caller: `/root/ROM/studio/src/lib/application/creation-drafts.ts:230`.
Initial storage SHA-256: `958ae920122e0736053da7ad34426eb143fe25f0f25412994722047695a1fa67`.
Initial draft SHA-256: `36ca364fa041bb740d3a1bf910acf39100cb9a6a01d6d691087cd77391438cbd`.

`read()` adopted the CAS version only after successful payload decoding.
A changed descriptor caused restoration to fail before version adoption.
Explicit discard then used a null expected version against an existing stored record.
The CAS failed. Draft storage remained in error and navigation stayed blocked.
The operator could not resolve this condition through the form's local discard control.

Minimal regression: save a draft under descriptor `v1`; create a fresh controller under `v2`; request restore; request explicit discard.
Restore must reject the incompatible draft. Explicit discard must remove only the observed, correctly scoped record and release navigation.
A concurrent replacement or foreign-owner envelope must remain protected.

The reviewer executed this sequence with an in-memory CAS slot against the initial source.
Observed output was `Form draft identity changed.`, then `recovery storage conflict`.
Navigation produced `Resolve creation draft storage before navigation.` The stored record remained present.

The coordinator's correction adds `createRecordStorage.remove()` and validates the creation envelope before explicit deletion.
Conditional receipt cleanup still uses the previously observed CAS version.
This preserves the distinction between intentional local discard and automatic matching-draft cleanup.
The correction was inspected. The coordinator reports 12 focused passes, including concurrent replacement and owner change during read.
The reviewer did not independently rerun that corrected regression set.
Coordinator-owned complete affected checks remain the integration evidence.

## Open finding

**P2: successful receipt recovery cannot clear its unchanged draft after restart.**

Location: `/root/ROM/studio/src/lib/application/creation-workflow.ts:139`.
Related UI: `/root/ROM/studio/src/lib/application/CreationForm.svelte:88`.

`acceptedDraft` exists only in memory. After reload, successful retry conservatively retains the restored editor draft.
This also retains an unchanged original draft, because the workflow cannot prove its relationship to the accepted command.
The reviewer reproduced phase `succeeded` with the original editor snapshot still present.
The form remains open because its completion callback requires an empty editor snapshot.

This preservation prevents accidental loss of later raw edits, but leaves the completed original form available for another submission.
Same-process receipt recovery clears its matching draft. Restart recovery must provide the same behavior.
The full ergonomics goal therefore requires a correction before completion.

Minimal regression: save draft A; submit A with acknowledgement loss; restart; restore; recover the original command successfully.
The matching draft must clear and the form must complete.
Repeat with later invalid draft B before restart. B must survive the recovered receipt for A.

Persist a bounded accepted-editor association in the shared recovery contract before dispatch.
Do not infer equivalence from canonical operation equality, which omits raw editor text and intent distinctions.
Test unchanged-draft cleanup and later-draft preservation together.

## Standards and contract assessment

The creation modules have distinct responsibilities: raw draft storage, workflow composition, application integration, and presentation.
The shared CAS helper removes duplicate serialization behavior without changing public Resource-target slot paths.
Bootstrap creation slots have a distinct tuple shape and retain namespace, owner, kind, and record-role scope.

Snapshot copies use the wire codec for explicit wire trees. Exact integers retain bigint representations.
Creation draft restore checks descriptor identity and rejects incompatible snapshots.
Draft snapshots and accepted operations remain separate.

The draft controller checks owner and epoch around asynchronous storage operations.
Published snapshots hide another owner's content even before an explicit rebind completes.
The application keys Resource presentation by owner and kind and clears creation composition on owner changes.
The shared recovery controller aborts stale transport work and checks binding generation before publishing results.
These source checks support privacy; they do not establish every possible host storage or session interleaving.

Navigation guards include unresolved creation, writing drafts, and storage errors.
Local discard has an explicit warning that it does not undo a backend commit.
Current mutation authority gates restore, submit, and retry.
No source path reviewed treats a stored record as authentication or authorization.

## Executed and inspected evidence

The reviewer executed this bounded command against the initial source:

```sh
node --experimental-strip-types --test studio/tests/unit/creation-drafts.test.ts studio/tests/unit/creation-workflow.test.ts studio/tests/unit/application-creation.test.ts
```

Result: 12 tests passed; zero failures; zero skipped tests.
The reviewer also executed the two standalone reproductions described above.
No native build, browser server, fixture mutation, or production operation was performed by this reviewer.

The reviewer read `/var/tmp/rom-010-creation-ui-evidence.json` and its actual unit, component, and check logs.
The logs contain 313 unit passes, 223 component browser passes, one existing skip, and zero Svelte errors or warnings.
These logs describe the initial source. The draft and CAS files changed during the review correction.

The reviewer inspected the installed consumer test source and four result files named in that evidence record.
The full SQLite and redb results each contain nine passing cases in Chromium and WebKit 2359.
This is 36 full-matrix executions. Only one of the nine cases specifically covers creation.
The full matrix therefore contains four creation executions; the two earlier focused runs add four historical creation executions.
Other cases cover selected-row recovery, navigation, session lifecycle, privacy, and startup diagnostics.

The creation case checks explicit reload restoration, exact original command replay, large integers, null expected revision,
receipt identity, one event, and retention of later invalid drafts.
It does not test descriptor-mismatch discard, creation-specific owner-change races, or unchanged-draft cleanup after reload.

The file-backed unit helper ignores its slot argument. Its owner-change test proves hiding and foreign-envelope rejection.
It does not independently prove retrieval from distinct owner slots. Bootstrap slot tests and actual IndexedDB journeys provide separate evidence.

## Remaining limits

The inspected installed results use `authoring_checkout` and state `admitted: false`.
They use synthetic loopback identity and an earlier witnessed native fixture.
They do not establish production provider identity, TLS deployment, clean-source release admission, or original Astral Plane acceptance.

The reviewer did not rerun the complete browser matrix or full local verifier.
The coordinator owns those checks and the updated source identity record.
The existing skipped browser case remains outside the executed coverage.
Source review and automated browser evidence do not replace an independent human authoring usability review.

## Proof correction follow-up review

The reviewer inspected the integrated draft matching helper, creation workflow, durable proof contract, and fingerprint implementation.
The helper hashes the complete bounded raw draft, including exact integers and nested custom editor state.
The accepted record retains the proof without adding it to the backend invocation.
Legacy records omit proof and preserve drafts conservatively.

The workflow checks its owner, epoch, and current authority after each asynchronous fingerprint calculation.
Conditional deletion compares the captured draft again after hashing and uses the observed CAS version.
Later changes and concurrent stored replacements therefore do not become matching cleanup targets.
Descriptor mismatch remains a restore refusal with separately validated explicit local discard.
Navigation remains blocked by unresolved commands, pending writes, and storage errors.

The fingerprint file was formatted after its original freeze. This changed its source hash without changing the inspected algorithm.
The original `/var/tmp/rom-010-editor-proof-evidence.json` remains historical evidence.
The new inspection hashes are in `/var/tmp/rom-010-creation-proof-independent-review-evidence.json`.

**Open P2: nested invalid drafts can receive an accepted cleanup association.**

Location: `/root/ROM/studio/src/lib/application/creation-drafts.ts:33`.
`creationDraftMatches()` checks only each field editor's top-level `invalid` member.
The supported `EditorDraft` contract also stores errors in children, list rows, and aggregate errors.
For maps, `ValueEditor` retains invalid child text while the field intent keeps its last valid value.
The parent editor need not have its own `invalid` member.

The reviewer reproduced a map draft with `children.a.text = "invalid-number"`, `children.a.invalid`, and `errors.a`.
Its last valid intent was `{a: 7n}`. A separate explicit create used that last valid operation input.
The matching helper returned true. Successful workflow submission cleared the stored nested invalid raw draft.
Reproduction output is in `/var/tmp/rom-010-creation-nested-proof-review.log`.

The standard form blocks invalid submission. This defect concerns the framework's explicit submission and draft-association boundary.
That boundary already promises to preserve unrelated drafts when an external operation differs.
It must also preserve raw invalid nested drafts that retain equal last-valid wire values.

Minimal regression: store an invalid nested map or list draft; explicitly submit its last valid create operation; recover success.
The nested raw draft must remain stored and no accepted editor proof must associate it with that operation.
Repeat after restart. Keep the existing unchanged-valid cleanup and later-invalid preservation cases.
Validate supported editor error locations through a bounded traversal.
Do not treat arbitrary custom `state` properties named `invalid` as standard validation metadata.

The independently executed command was:

```sh
node --experimental-strip-types --test studio/tests/unit/recovery-editor-proof.test.ts studio/tests/unit/creation-workflow.test.ts studio/tests/unit/creation-drafts.test.ts studio/tests/unit/application-creation.test.ts
```

Result: 27 passes, zero failures, zero skips.
The log is `/var/tmp/rom-010-creation-review-proof-focused.log`.
The coordinator reports 328 complete unit passes and zero Svelte errors or warnings for the integrated candidate.
Those coordinator results were not independently rerun by this reviewer.
New browser assets and database/browser checks remain coordinator-owned.

The original matching-cleanup P2 is corrected in the inspected integration, subject to current browser and integration checks.
The nested-editor P2 remains open. This follow-up does not establish release admission or original-consumer acceptance.

## Nested correction and navigation follow-up

The coordinator moved association matching into `/root/ROM/studio/src/lib/application/creation-proof.ts`.
The inspected helper walks supported `children`, `errors`, and list `rows`, including each row's draft and error.
It does not interpret arbitrary custom `state` as validation metadata.
The workflow calls it with the already validated, bounded stored snapshot.
The nested-editor P2 is corrected in this source.

The reviewer reran the same four focused unit files after this correction.
Result: 30 passes, zero failures, zero skips.
The log is `/var/tmp/rom-010-creation-review-nested-fixed-focused.log`.
The earlier 225 browser passes with one skip and 40 installed App passes predate this helper correction.
The coordinator is rerunning current-source browser evidence separately.

**Open P2: same-kind descriptor renewal retains the obsolete creation manager.**

Location: `/root/ROM/studio/src/lib/application/controller.ts:105`.
The manager cache checks only Resource kind. Its draft validator captures the descriptor when the manager is constructed.
Same-owner renewal updates discovered descriptors but retains this manager.
A reopened form captures the new descriptor, which the obsolete manager rejects.
Explicit discard removes the stored record but does not replace the cached manager.

The reviewer executed an application-level reproducer with descriptor version 1, then same-owner renewal to version 2.
The discovered descriptor became version 2. New version-2 draft staging failed with `Form draft identity changed.`
Staging still failed after explicit local discard.
The log is `/var/tmp/rom-010-creation-descriptor-manager-review.log`.

Minimal regression: open creation under descriptor A; discard local creation; renew the same owner with descriptor B; reopen creation.
A valid B draft must persist. Also cover descriptor renewal with an unresolved accepted A command.
Preserve its exact retry identity and reject incompatible raw restoration without replacing or losing that command.
Manager invalidation must include descriptor identity and maintain these guards.

**Open P2: visible row and page controls bypass the creation navigation barrier.**

Locations: `/root/ROM/studio/src/lib/application/ResourcePage.svelte:296` and `/root/ROM/studio/src/lib/application/ResourcePage.svelte:312`.
Related guard: `/root/ROM/studio/src/lib/application/controller.ts:380`.
The table's Open controls have no blocked state. Page controls check ordinary busy state and page eligibility only.
Creation busy, unknown outcomes, and creation storage errors do not disable these controls.
The row callback changes the inspector tab and then discards the promise from `controller.selectRow()`.
Its creation guard rejects before the controller's request catch block.
Eligible page callbacks likewise discard promises whose navigation guard rejects.

Source inspection establishes the enabled controls and uncaught rejection paths. No browser reproduction was executed by this reviewer.
The controller retains the backend navigation barrier, so this is a presentation and error-handling defect, not unauthorized dispatch.
Minimal browser regression: create an unknown outcome; click an available Open control and an eligible page control.
The controls must prevent navigation or present a handled guard message, with no `pageerror` and unchanged selection context.
Repeat with creation draft storage refusal. Keep programmatic controller guards.

These two P2 findings remain open at this checkpoint.
The report preserves earlier findings and evidence as dated review stages, rather than relabeling them as current-source checks.

## Descriptor and navigation correction review

The reviewer inspected the coordinator's corrected manager, writer, creation form, Resource table, and navigation callbacks.
The manager now records the canonical creation descriptor identity.
New draft staging and new submission require the current definition.
Restore, retry, and explicit discard retain the existing manager and original command identity.
An existing draft blocks manager replacement until explicit discard resolves it.
An obsolete writer cannot stage after another writer replaces its manager.

The creation form retains its displayed descriptor and raw draft during renewal.
The changed-definition banner explains the next action. The original command remains available for explicit retry.
Input and new submission remain disabled until the operator discards incompatible edits.
Successful discard adopts the current descriptor, resets the form, remounts its controls, and closes the confirmation dialog.
The inspected implementation corrects the obsolete-manager P2 without silently converting drafts.

The Resource table accepts an optional disabled state. ResourcePage supplies its complete blocked-navigation state.
Row and eligible page controls therefore include unresolved creation and draft storage barriers.
The callbacks also check the current barrier and catch asynchronous controller guard failures.
The controller guards remain intact.
This corrects the row/page presentation P2 in the inspected source.

The reviewer independently reran the same four focused Node unit files.
Result: 32 passes, zero failures, zero skips.
The log is `/var/tmp/rom-010-creation-review-descriptor-nav-focused.log`.
This run includes the new same-kind descriptor renewal and explicit-discard writer cases.
Current inspection hashes are in `/var/tmp/rom-010-creation-descriptor-nav-review-evidence.json`.

The coordinator reports 333 complete unit passes, zero Svelte errors or warnings, and ten focused creation browser passes across both engines.
The coordinator also reports the actual navigation and confirmation-dialog failures observed before their corrections.
The reviewer did not rerun browser tests. Complete components and installed App checks were still running at this checkpoint.
Historical 40 installed App passes do not cover the final manager, navigation, and dialog corrections.

No previously reported creation P2 remains open in this inspected correction.
The reviewer found no additional concrete blocker in the reviewed draft association, owner checks, conditional CAS deletion, or descriptor handling.
This conclusion concerns the inspected source and focused tests. It does not establish production identity, full verifier success, or release admission.
The coordinator reports a full-verifier failure in a portable-provider Node fixture before Rust checks.
That failure and current-source browser/database evidence remain coordinator-owned integration work.
