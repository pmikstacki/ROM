# Bounded WebKit failure diagnosis for 584c01b

Date: 2026-10-05.
Reviewed clean source: `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
Conclusion: the immediate failure is an intermittent native WebKit web-process segmentation fault on a compositor thread.
No application selector, database, authorization, or mutation-contract defect is demonstrated by these failures.
The underlying native engine/environment/rendering cause remains unresolved.
No speculative UI, SDK, timeout, force-click, or assertion change is justified by this evidence.

## Original producer failure

The preserved production browser log is `/var/tmp/rom-studio-accepted-source-BxvdEJ/browser.log`.
Its SHA-256 is `9c0f5e9f5f6c09b09e47fe4da00fd6eba6b1164e877cc43dce7176910f844ea4`.
The suite reports 27 passes and one failure.
The failed case is WebKit with SQLite, semantic catalog creation/patch/actions/query.
It ran for 18.7 seconds before the page closed.
Chromium SQLite/redb and the subsequent WebKit redb semantic case passed.

The failing helper resolved the exact Set value option within the open listbox.
Playwright was waiting for that option to become stable when the target page/context/browser closed.
This is not a missing-element timeout or a failed canonical-value assertion.
Source inspection found no page/context/browser close in the semantic workflow or selection helper.
The retained error context contains the same close error and source location; it has no surviving DOM snapshot.
No trace archive or screenshot was retained in that failed case directory.

The bounded host journal window, 15:21–15:25 UTC, records:

- At 15:22:28, compositor thread 1375383 faulted at address zero inside `libWPEWebKit-2.0.so.1.12.0`.
- WPEWebProcess PID 1375363 terminated with signal 11, SIGSEGV.
- Its coredump identifies the crashing thread and top library offset `0x60a0f8a`.

This directly identifies native browser process termination during the failed browser case.
No OOM-killer signature appeared in the inspected bounded lines.
The coredump service's memory peak is dump-processing evidence, not proof that the browser was killed for memory exhaustion.
The stack is unsymbolized and contains no actionable native function name beyond its library/offset.
The review did not extract core memory or inspect browser profiles.

## Unchanged focused repetition

The owner ran five focused repetitions against the same production assets and native host.
`/var/tmp/rom-003-webkit-584-repeat/run.log` records four passes and one failure.
`report.json` records four expected results, one unexpected result, zero skips, and zero flaky results.
This is not a five-pass acceptance run.
The first repetition reports the same option actionability wait followed by target closure.
The remaining four repetitions passed without source changes.

The additional explicitly bounded journal window, 15:25:35–15:26:15 UTC, confirms the repeated native failure:

- At 15:25:43, compositor thread 1379601 faulted at address zero in the same WebKit library.
- WPEWebProcess PID 1379581 terminated with signal 11.
- The coredump again identifies top offset `0x60a0f8a`.

The exact matching native crash site strengthens the engine/process-crash classification.
A probabilistic rendering interaction may trigger the engine defect; the available stack does not distinguish that from environment or driver instability.
The browser's stability waits do not establish the root cause, and bypassing them would conceal behavior rather than diagnose this segmentation fault.
Four unchanged passes do not prove the crash absent.

## Historical correspondence and support boundary

The release acceptance document records an earlier native WebKit compositor crash for candidate 5538323.
Its exact timestamp/PID/signature was not supplied to this review.
No broad historical journal or coredump scan was performed.
Therefore this report claims correspondence with the recorded failure class, not verified identity with that older crash site.

The affected automation uses the local Playwright WebKit 2359 WPE backend.
The result does not establish failure on every Safari/WebKit platform, an application-side authorization bypass, a server crash, or database corruption.
It does establish that this native WebKit automation environment can intermittently terminate during a real generic select interaction.
That limitation must remain visible in acceptance evidence, including the repeated 4/5 result.
Do not transform the failed producer stage into a passing stage or report it as a test skip.

The owner has started one unchanged full producer retry.
If it completes, its successful result must identify its own manifest, assets, source, and logs.
The earlier failed stage and native-crash records remain separate preserved evidence.
A passing retry may establish the required gate for its run; it does not erase the known intermittent native WebKit limitation.
No completed artifact, deployment, or release-goal claim follows from this diagnosis.

## Read-only scope and retained evidence

Only files under `/var/tmp/rom-003-webkit-584-diagnosis/` were written.
The review read the preserved browser log/error context, the two named coredump records, and bounded journal windows.
It did not change source, dependencies, browser arguments, native binaries, test controls, private profiles, or services.
It did not run further browser cases or broad tests.

Retained local evidence:

- `journal-bounded.log` and `compositor-stack.log`: original native failure.
- `repeat-journal-bounded.log` and `repeat-compositor-stack.log`: matching focused-repeat native failure.
- `browser-failure.log` and `error-context.md`: original test error.
