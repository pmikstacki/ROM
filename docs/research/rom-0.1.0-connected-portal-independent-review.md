# Connected maintenance portal independent review

Date: 2026-10-08. Assessment: both reported defects are corrected in the re-reviewed frozen candidate. Release admission remains open.

## Scope and method

Reviewed the NEW `studio/tests/ui-composition/consumer/**`, `verify.mjs`, `http.mjs`, and `playwright.config.ts` sources.
The accepted baseline is `584c01b614127b3f62799f1a26a1cdf3f734c3dc` (v0.0.3).
The current HEAD is `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`. The reviewed fixture is untracked candidate work.
Unrelated dirty changes are outside this review.

Requirements came from Task 4 in `docs/superpowers/plans/2026-10-07-ui-composition-admission.md`, `docs/studio-compositions.md`, and the maintenance layout fixture report.
Applied `AGENTS.md`, `docs/quality.md`, `docs/writing.md`, and `skills/project/rom-studio/SKILL.md`.
Used the requesting-code-review workflow as the assigned independent reviewer. No further reviewer was dispatched.

This review inspected sources, result files, browser reports, provenance, and hashes.
It did not launch a compiler, native fixture, proxy, or browser. Findings below are source review, not locally executed regressions.
Only this NEW report was written. Product sources, fixtures, index, HEAD, and historical evidence were preserved.

## Verified frozen evidence

| Matrix | Chromium | WebKit | Skipped | Unexpected | Flaky |
| --- | --- | --- | --- | --- | --- |
| `/var/tmp/rom-010-portal-grid-sqlite-final-v2/result.json` | 13 passed | 13 passed | 0 | 0 | 0 |
| `/var/tmp/rom-010-portal-grid-redb-final/result.json` | 13 passed | 13 passed | 0 | 0 | 0 |

Both results report `completed: true`, `admitted: false`, `dependency_bootstrap: false`, and `source_mode: supplied_source_archive`.
Each records successful locked installation, type checking, build, and browser commands.
The browser reports independently confirm each table count. These are inspected prior executions, not this reviewer's reruns.

Both original matrices have identical producer and fixture manifests. At the initial review, every fixture hash matched the reviewed fixture.
At the initial review, every producer hash matched Studio source, with root license and notice files resolved explicitly.
The witness `/var/tmp/rom-010-portal-grid-matrix-witness.json` reports matching identities and counts.
These comparisons were independently repeated using Node filesystem reads and SHA-256.

- Source archive SHA-256: `661581fa0662b7361d3821afaef476c7bb38d726c61241d62735271480c22e61`.
- Native identity: `/var/tmp/rom-010-maintenance-native-e7Sabk/identity.json`.
- Native binary SHA-256: `660ae055aec7ef0734b01b54da147cf5f2cd1faf3560de7f05ceac64d9bf4b8d`.
- Native binary size recorded in identity: 116039208 bytes.

An independent streaming hash of the preserved binary matches the native identity and both results.
The binary is the earlier witnessed core. These matrices do not execute the coordinator's later `Runtime.calculate` addition.
They do not establish updated-core, packaged-release, or production-identity acceptance.

## Strengths

The external consumer imports public client, observation, recovery, controls, UI, component, and style entries.
It uses the shared mutation recovery controller and an explicitly opened principal-bound IndexedDB store.
It does not implement another mutation repository or receipt store.

The proxy forwards actual backend requests. A lost acknowledgement is injected only after a successful upstream invocation response.
The inspection scenario compares both serialized invocations exactly and checks the real journal after backend restart and browser reload.
The invalid date and corrupt layout scenarios exercise actual server rejection.

Authority generation clears disclosed rows, selections, history, drafts, and export results.
Held export work checks current generation before publication. History and restore errors have sanitized, generation-guarded callbacks.
The maintained history refusal and restore failure cases inspect browser errors and preserved original intent.

Layout tests inspect displayed panel positions and dimensions, then compare them after reload.
Document-relative positions include `scrollX` and `scrollY`, avoiding false movement from Playwright scrolling controls into view.
Keyboard layout activation, mobile details resizing, draft retention, Escape, focus return, and horizontal overflow have recorded browser coverage.

The verifier preserves installed source identities, locks dependencies, bounds commands, uses one browser worker, and requires actual WebKit configuration.
It explicitly rejects `--admit`. Existing results accurately retain `admitted: false`.

## Findings

### Important: WorkOrder unknown outcomes have no operator recovery path

Status: corrected and verified against the later frozen source/evidence described below. The original finding is preserved.

Files: `studio/tests/ui-composition/consumer/src/App.svelte:220`, `App.svelte:227`, `App.svelte:234`, and `consumer/src/host.ts:170`.

WorkOrder completion uses durable recovery. An unknown result sets `hasUnresolvedIntent`, which disables the only completion button.
Unlike Inspection and Settings, the WorkOrder panel renders no restore or original-intent retry control.
After reload, `checkStored()` detects the retained WorkOrder slot. `commit()` at `host.ts:283` refuses another invocation while that slot exists.
The UI therefore cannot resolve that WorkOrder outcome in either lifetime.

Source-derived reproduction, not executed in this review:

1. Open the ready Alice fixture.
2. Send `/__fixture/control` the JSON `{"type":"drop-ack","route":"invoke","kind":"work-orders"}`.
3. Activate `Complete work order`.
4. Observe the unknown recovery state and absent original-intent retry control.
5. Reload the page. Observe the absent WorkOrder restore control.

The server may have completed the action. A fresh action is not a substitute for retrying the retained invocation.
Add explicit WorkOrder restore and original retry controls using the existing host methods.
Maintain an actual browser regression across both databases and engines. Compare exact invocation bodies and verify a single completion event.

### Important: Proxy bind failure does not drain the launched native backend

Status: corrected and verified against the later frozen source/evidence described below. The original finding is preserved.

Files: `studio/tests/ui-composition/http.mjs:155`, `http.mjs:369`, and `http.mjs:375`.

`startPortalFixture()` launches the backend before binding the proxy port.
The startup cleanup catches only the initial native launch. The later `server.listen()` rejection has no surrounding backend cleanup.
When the port is occupied, `startPortalFixture()` rejects before returning the object that exposes `close()`.
Its caller cannot drain the already launched native process through that object.

The native self-expiry eventually bounds the fixture lifetime. It does not satisfy immediate owned-process cleanup after failed startup.
The backend can retain its database and native allocation after the proxy failed to start.

Source-derived reproduction, not executed in this review:

1. Reserve the intended loopback proxy port with another listener under coordinator allocation.
2. Call `startPortalFixture()` using the preserved binary and a new evidence/database directory.
3. Wait for rejection with `EADDRINUSE` after native readiness.
4. Check whether the launched child exited and released its database handle.

Wrap proxy binding in cleanup that drains the already launched backend before propagating failure.
Maintain a bounded occupied-port regression. Verify child exit and database reopening; preserve its evidence.
Any native execution requires coordinator scheduling. This review requested no unscheduled launch.

## Correction re-review

Reviewed the corrections through source and previously executed evidence only. No compiler, service, or browser was launched.
Both later results have identical producer and fixture manifests. All 19 recorded fixture hashes match the current reviewed files.
All recorded producer hashes also match current Studio source. The archive and preserved native hashes remain those listed above.
The witness is `/var/tmp/rom-010-portal-recovery-matrix-witness.json`.

| Matrix | Chromium | WebKit | Skipped | Unexpected | Flaky |
| --- | --- | --- | --- | --- | --- |
| `/var/tmp/rom-010-portal-recovery-sqlite-final/result.json` | 15 passed | 15 passed | 0 | 0 | 0 |
| `/var/tmp/rom-010-portal-recovery-redb-final/result.json` | 15 passed | 15 passed | 0 | 0 | 0 |

Each result records successful locked installation, type checking, build, integrated bind-failure regression, and browser execution.
Both report `completed: true`, `admitted: false`, and no dependency bootstrap. Browser reports independently confirm the table counts.
The re-review repeated source SHA-256 comparisons and inspected the actual bind result records, rather than accepting aggregate counts alone.

### WorkOrder correction

`consumer/src/App.svelte:237` now exposes explicit restore and original retry controls for Alice's disclosed WorkOrder.
The completion button also blocks while the stored slot exists. `consumer/src/host.ts:438` supplies the WorkOrder-specific sanitized restore error.

The maintained scenario at `consumer/tests/portal.spec.ts:3` covers lost acknowledgement, backend restart, browser reload, and explicit restore.
It injects IndexedDB failure, checks the WorkOrder error label, and confirms the stored original remains recoverable.
It switches to Bob, checks absent recovery controls, and verifies actual server refusal of Bob's original-body replay.
After returning to Alice, it restores and retries the original invocation. Both Alice invocation bodies must match byte-for-byte.
The receipt must show revision two and completed state. The actual journal must contain only seed and completion events.

The scenario at `consumer/tests/portal.spec.ts:118` holds an actual original replay, then switches principal before release.
It checks cleared WorkOrder state, absent old retry control, and no old title disclosure. Both invocation bodies remain identical and Alice-owned.
Both cases passed on both adapters and engines in the later matrices. The page-error lists remain empty in their assertions.

Preserved failure evidence includes `/var/tmp/rom-010-portal-work-red/browser.log` and `/var/tmp/rom-010-portal-work-restore-red/browser.log`.
Their result records show browser failure after successful installation, type checking, and build. The initial missing-control failure remains historical evidence.
These corrections close the specific WorkOrder recovery finding for the recorded candidate, not every possible storage or authority failure.

### Proxy startup correction

`http.mjs:369` now catches bind failure. It releases held work, closes connections, and awaits `stopBackend()` before propagating the error.
The existing stop path waits for native exit and requires successful shutdown. This closes the identified normal `EADDRINUSE` cleanup defect.

`consumer/tests/bind-failure.mjs` owns an occupied listener and records exactly one launched fixture child.
Before its fallback cleanup, it requires that child to be absent from `/proc` after startup rejects.
It then reopens the same database with the immutable binary, performs an authorized Settings read, and requires clean exit.
`verify.mjs:232` executes this regression before browser execution, using the adapter and bounded verifier command wrapper.

The actual SQLite bind record identifies PID 880869. The actual redb bind record identifies PID 883946.
Both record `bind_error: EADDRINUSE`, `child_terminal_before_cleanup: true`, and `reopened: true`.
Records are under each later matrix's `bind-failure/result.json`. The corresponding command exits are zero.

Preserved `/var/tmp/rom-010-portal-bind-sqlite-red.log` and `/var/tmp/rom-010-portal-bind-redb-red.log` fail the intended child-terminal assertion.
Their bind result records show `child_terminal_before_cleanup: false` and `reopened: false`.
The actual database reopening evidence closes the specific bind-failure finding on both adapters.

The later matrices still use native SHA-256 `660ae055aec7ef0734b01b54da147cf5f2cd1faf3560de7f05ceac64d9bf4b8d`.
They do not execute later core calculations or the separately built public-guest backend. No guest browser acceptance is inferred.

## Considered and declined to judge

- Public guest compute/export: explicitly required, but still open in these reviewed matrices; their preserved backend admits private trusted-human Resources only.
- Actual authentication renewal, transient authority retention through expiry, and production credential verification: fixed fixture credentials do not establish these behaviors.
- Original Astral Plane adoption: this independent fixture provides no original-consumer execution.
- Held-out human authoring, screen-reader, contrast, and complete mobile accessibility acceptance: agent/browser checks cannot replace those reviews.
- Packaged release provenance and updated-core compatibility: matrices use checkout-source archives and the earlier immutable native binary.
- Full local verifier and complete affected suites: not executed by this reviewer under the read-only allocation.
- Corrupt-store or future-schema Settings parsing: `model.ts:59` can throw, but this backend's admitted layout codec rejects malformed JSON and geometry. No reachable valid HTTP failure was established.
- Host retry/discard authority race: inspected the recovery controller's post-storage `requireCurrent()` checks. No reachable cross-principal publication defect was established.
- History navigation barrier: current history selection changes only a revision marker; it does not navigate away or replace the unresolved editor.
- True browser background timer throttling: the visibility scenario injects the adapter event; it does not background the browser.

These items are not acceptance closures. The coordinator must preserve each open requirement and decide its next evidence increment.

## Assessment

The later frozen matrices support both reported corrections, including original WorkOrder replay and failed-startup database reopening.
No reported defect remains open in this narrow correction re-review. The original findings and earlier matrices remain historical evidence.
The separate public-guest, updated-core, full-verifier, release, production-identity, original-consumer, and human acceptance gates remain open here.
No release-ready claim follows from this review.
