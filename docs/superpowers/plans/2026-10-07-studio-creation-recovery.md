# Studio creation recovery

This work implements the remaining creation path for AP-UX-004 and AP-UX-005.
The owner authorized autonomous implementation and release. No new approval gate is added.
The release coordinator owns the files listed below. AI and identity workers retain their existing ownership.

## Required behavior

An unfinished creation form retains its exact ID text and field drafts after reload.
Invalid text stays separate from the accepted wire operation.
A submitted create retains its original target, wire, command key, retry epoch, and commit knowledge.
Recovery does not send a request automatically. Current authority applies to every explicit retry.
An owner change hides private drafts and commands. Local records do not grant authority.
Storage refusal blocks submission and navigation. Concurrent tabs use compare/exchange ownership.

## Selected design

Keep the existing Resource-target slot format unchanged.
Add an optional creation slot to the explicit profile contract.
The default bootstrap uses a distinct tuple shape, scoped to namespace, record role, owner, and Resource kind.
No Resource ID, including a synthetic-looking ID, can collide with this tuple.
One creation form per owner and kind uses separate editor and intent slots in the existing stores.

Reuse `createMutationRecovery` for accepted commands, persistence, receipt recovery, and current binding checks.
Add a bounded restore factory that validates a stored target before it constructs this existing controller.
The factory checks record structure, namespace, owner, and the caller's target constraint.
Its second read uses the existing controller scope check to reject a changed target.
It never performs HTTP. The caller still performs explicit retry through the restored controller.

A creation editor envelope stores raw ID text and the existing field snapshot.
The envelope has a separate format and kind binding. It does not invent a Resource target for invalid or empty ID text.
Bind both stores into the application controller and session lifecycle.
Before dispatch, freeze the complete command in the stable creation intent slot.
Publish creation recovery separately from selected-row recovery. Both lanes contribute to navigation guards.
Keep the latest draft separate from accepted command identity during later edits.

## Rejected alternatives

| Alternative | Reason |
| --- | --- |
| Synthetic Resource ID | Arbitrary real IDs can collide; invalid ID text also needs preservation. |
| Post-dispatch target pointer | A restart can lose the pointer after a successful commit. |
| New receipt implementation | It duplicates existing idempotency and commit-knowledge rules. |
| Automatic restore and retry | It hides authorization changes and can resend without user intent. |

## Execution and evidence

1. Add creation-slot separation tests, observe failure, then implement the compatible optional profile method.
2. Add restore-factory tests for committed acknowledgement loss, foreign owners, malformed records, and target constraints.
3. Implement the factory through the existing recovery controller. Test a target change between both reads.
4. Implement bounded creation editor storage and application/session wiring with failing regressions first.
5. Connect the generic Resource creation form. Preserve labels and existing compact controls.
6. Run actual create/reload/lost-acknowledgement journeys on SQLite and redb in Chromium and WebKit 2359.
7. Run complete affected suites and the local verifier before integration.
8. Include the source-matching fixture and evidence in clean-source release admission and original-consumer review.

Steps 1–3 do not establish complete form recovery. Steps 4–8 remain required.
Existing 32 App authoring executions remain historical evidence for selected-row recovery and startup diagnostics.
Do not relabel those executions as creation acceptance.

## File ownership

The release coordinator owns bootstrap and session types, the new restore factory and tests, creation draft modules,
application/controller integration, ResourcePage creation wiring, and corresponding public exports.
The AI and identity workers must not edit these files during this increment.

## Executed foundation increment

Steps 1–3 have candidate implementation and unit evidence.
Creation slots use six tuple members. Existing Resource-target slots retain seven members and unchanged bytes.
The public restore factory validates both reads and all subsequent writes through the existing recovery controller.
It preserves exact accepted wire and refuses another operation role in a creation scope.
Same-owner session renewal retains the original retry identity.

Two creation-slot tests failed before implementation.
The restore tests first failed because the public factory was absent.
Two later regressions exposed incorrect same-owner rebind behavior and acceptance of another operation role.
Both behavior failures and their corrections are preserved under /var/tmp/rom-010-creation-*.
The affected recovery/bootstrap run passed 54 cases. The complete Studio unit run passed 301 cases.
Svelte checking reported zero errors and zero warnings.
Source hashes and exact log paths are in /var/tmp/rom-010-creation-foundation-evidence.json.

Creation editor envelopes, application/session wiring, actual creation browser journeys, and full verifier remain open.
These unit results do not establish steps 4–8 or release acceptance.

## Creation editor persistence increment

The creation draft module now stores exact ID text and the existing field snapshot in a separate bounded envelope.
It preserves invalid editor text and exact integer values. Restore checks owner, namespace, kind, and descriptor identity.
Concurrent helpers cannot overwrite an unobserved stored version. Storage errors block navigation.
An owner change during a pending CAS hides the previous draft, including before explicit session rebind.
This privacy regression failed before its correction. The failed result is preserved.

Mutation records and creation drafts now share the extracted CAS storage module.
The affected draft and recovery run passed 51 cases. The complete Studio unit run passed 307 cases.
Svelte checking reported zero errors and zero warnings.
Source hashes, failed setup/assertion results, actual check results, and limits are recorded in /var/tmp/rom-010-creation-drafts-evidence.json.

Step 4 remains partial: application and session wiring are not implemented yet.
Steps 5–8 remain required, including actual creation browser journeys and full verifier before integration.

Save confirmation can now clear only the draft that matches its captured input.
A later draft remains stored. This prevents acknowledgement from erasing edits made during a request.
The conditional-clear API first failed its regression. The affected suite then passed 52 cases.
The complete Studio unit run passed 308 cases, and Svelte checking reported zero errors and zero warnings.
The shared CAS refactor still requires full verifier and browser evidence before integration.

## Application and creation form increment

The application now composes creation drafts with the existing mutation recovery controller.
It freezes accepted commands in the owner/kind intent scope before dispatch.
Session rebind hides another owner's state. Current authority applies to explicit restoration and retry.
The UI exposes Restore creation, Retry creation, and confirmed local discard.
Later raw drafts remain separate from the accepted command and survive receipt recovery.
The complete Studio unit run passed 313 cases. Svelte checking reported zero errors and warnings.

The new component browser case passed in Chromium and WebKit 2359.
It uses real IndexedDB and a synthetic transport. It does not prove database commit behavior.
The installed public App creation case then passed on actual SQLite and redb in both engines.
It checked exact wire replay, null expected revision, receipt identity, one event, and later invalid draft restoration.
These four executions use authoring source and an earlier witnessed native fixture binary.
They do not establish clean-source release admission or production provider identity.

The first installed creation attempt used the wrong accessible role in its locator.
The second omitted a required nullable field. Both fixture failures remain preserved.
Corrected results are in /var/tmp/rom-010-creation-app-sqlite-3-20261007/result.json and /var/tmp/rom-010-creation-app-redb-20261007/result.json.

The expanded component suite exposed obsolete startup fixtures without an explicit host profile.
The components build now supplies a test-only profile. Production builds still require the deployment-owned profile.
The obsolete execution was interrupted after its recorded failures. Its log and stop evidence remain preserved.
The corrected run exposed four stale session-message expectations. The affected 16-case run passed after correction.
The corrected full component run and expanded nine-case public App matrix are being checked separately.
The corrected full component run passed 223 cases and skipped one existing case.
The complete nine-case public App run passed in both engines on SQLite.
The complete redb run remains pending at this checkpoint.
Evidence and source hashes are in /var/tmp/rom-010-creation-ui-evidence.json.
An independent reviewer received the frozen creation source and recorded evidence.
The subsequent complete redb run passed all nine cases in both engines.
The complete authoring matrix now has 36 passes across both adapters and engines.
Producer source, fixture source, installed lock, and native binary identities match between those two runs.
The native fixture remains an earlier witnessed authoring build. Release admission remains false.
Full verifier, independent review, clean-source admission, and original-consumer acceptance remain required before release.

## Independent review correction: descriptor mismatch

The reviewer reproduced a local draft that could not be discarded after its descriptor changed.
Normal reads correctly refused the changed descriptor and did not adopt its stored version.
Explicit discard therefore used the wrong CAS version and remained blocked.

The corrected explicit deletion path reads the current record and validates its owner, namespace, kind, format, and byte bound.
It deletes that observed version through CAS without decoding the obsolete form definition.
Normal reads and writes still require full value validation.
Conditional save cleanup still deletes only its matching captured draft through the existing versioned write path.
Two intended regressions failed before the correction. The complete Studio unit run then passed 316 cases.
Concurrent replacement and owner change during the deletion read leave the stored record unchanged.
Svelte checking reported zero errors and warnings.
The earlier browser evidence predates this correction and is not evidence for the changed source.
Logs are /var/tmp/rom-010-creation-descriptor-discard-red.log and /var/tmp/rom-010-creation-discard-all-unit.log.
The corrected component bundle built successfully. Evidence is in /var/tmp/rom-010-creation-discard-evidence.json.
Independent review is in docs/research/rom-0.1.0-creation-independent-review.md.

## Remaining review correction: matching cleanup after restart

The reviewer reproduced a completed retry that retained its unchanged original editor draft after restart.
The current in-memory accepted draft cannot establish matching cleanup across restart.
Preserving that draft prevents data loss, but it does not complete the expected ergonomic workflow.

Add a bounded accepted-editor fingerprint to the existing durable mutation recovery record.
The fingerprint identifies the exact raw creation ID and form snapshot. It is not authority or a backend receipt.
Capture it before dispatch. Retain it with the accepted command and validate it during restore.
After successful retry, compare it with the current raw draft before conditional deletion.
Later invalid fields or changed raw text must remain stored.
Records without that fingerprint retain conservative explicit cleanup behavior.
Use the shared recovery contract instead of a separate creation receipt store.

Required tests cover unchanged cleanup after restart, later edits, exact integers, owner changes, and descriptor mismatch.
Run the actual public App cases on both database adapters and browser engines.
Do not close this finding from canonical operation equality; different raw drafts can produce the same wire operation.
The shared recovery worker owns the optional proof record, validator, state, fingerprint helper, and new unit regressions.
The release coordinator owns creation workflow composition, UI, and actual browser acceptance.
Use the native digest interface documented by the [Web Cryptography recommendation](https://www.w3.org/TR/2017/REC-WebCryptoAPI-20170126/).
The fingerprint remains local matching metadata and confers no authority.

## Integrated accepted-editor proof candidate

The shared recovery record now retains an optional accepted-editor fingerprint. Legacy records remain valid without it.
Creation captures the fingerprint before dispatch. Successful retry removes only the matching current draft through conditional CAS deletion.
Later raw text, invalid fields, and unrelated direct-create input remain preserved.
The backend invocation does not contain the local fingerprint.

The browser regression failed in both engines before the corrected bundle. The original form remained open after confirmed restart recovery.
The corrected candidate closes that form. Explicit restore then finds no original draft.
Another test preserves later invalid edits and replays the exact original command.

Independent review found a further nested-editor defect. Three direct-create regressions reproduced deletion of invalid nested data.
The shared association helper now checks children, error maps, and list rows. It does not inspect arbitrary plugin state.
A separate regression rejects field-name delimiter collisions.
An intermediate test run failed because expected objects did not match the decoder's null prototypes. That failure remains preserved.
The corrected assertions compare the normalized captured editor data.

The current candidate passed 331 Studio unit tests. Svelte checking reported zero errors and warnings.
The complete component suite passed 225 tests and skipped one existing test.
The installed public App passed ten cases in each engine on each adapter: 40 executions in total.
Producer source, fixture source, consumer lock, and native binary identities match between the two adapter runs.
These are authoring checks with an earlier witnessed native fixture. Release admission remains false.
Evidence is in /var/tmp/rom-010-creation-proof-integrated-evidence.json.

Independent review still identifies two P2 findings: same-kind descriptor renewal and enabled row/page navigation during pending creation.
Those corrections, the full local verifier, clean-source admission, and original-consumer acceptance remain required.

## Descriptor renewal and navigation corrections

Two application regressions reproduced reuse of an obsolete creation manager after same-kind descriptor renewal.
The manager now tracks the canonical creation definition. New writes require the current definition.
Existing restore, retry, and discard retain the original manager and command identity.
A retained draft prevents silent replacement. Explicit discard permits editing with the new definition.
Obsolete writers cannot stage data after manager replacement.

The visible form retains its original descriptor until explicit discard. A definition-change notice explains the available actions.
Original pending creation still supports explicit retry under current authority.
Browser tests cover retained edits, adoption after discard, and exact pending-command replay after renewal.

Navigation regressions failed in both engines before the correction. Row and page controls now share the existing blocked state.
Handlers refuse blocked events and catch navigation failures. Forced events do not produce uncaught promise rejections.
The controls become available after confirmed recovery.

An additional browser regression reproduced a discard dialog that remained open after successful deletion.
Successful discard now closes the dialog. Failed deletion retains the confirmation state.
The failed preliminary browser run and the focused dialog failure remain preserved.

The corrected source passed 333 unit tests and Svelte checking with zero errors and warnings.
The complete component suite passed 231 tests and skipped one existing test.
The installed App matrix passed 40 executions across SQLite, redb, Chromium, and WebKit.
Both adapter runs used identical producer source, fixture source, installed lock, and native binary identities.
Independent review reran 32 focused tests and found no remaining reported creation P2 finding.

The full local verifier exited one before Rust checks. Its Node stage passed 190 tests and failed one test file.
The provider browser-storage test required a host-only directory absent in the development container.
The identity worker owns that portability correction. A complete verifier rerun remains required before integration.
Evidence is in /var/tmp/rom-010-creation-renewal-evidence.json.
Clean-source admission, production identity, original-consumer acceptance, and the complete release objective remain open.
