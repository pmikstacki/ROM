# Corrected WPE runtime probe for source 584c01b

This is a browser-runtime correction probe, not a ROM implementation change or completed release. Playwright JavaScript remains locked at 1.63.0; the new executable is official Linux WPE revision 2364 rather than its default revision 2359. The existing 2359 runtime, SDK lock, old failed suites, and crash records remain intact.

## Cause and upstream remedy

The [upstream Playwright issue](https://github.com/microsoft/playwright/issues/42637) reports the exact build-2359 compositor null-read offset `0x60a0f8a` recorded in both bounded local crashes. Its maintainers link [WebKit PR 73621](https://github.com/WebKit/WebKit/pull/73621), which adds a null check before using the underlying SkImageFilter in SkiaCompositingLayer, and [Playwright roll PR 42748](https://github.com/microsoft/playwright/pull/42748), which rolls Linux WPE to revision 2364. The exact signature and maintainer links strongly support this upstream defect as the local cause; local stripped crash stacks alone did not identify a function.

## Runtime identity and bounded adaptation

Official archive: https://cdn.playwright.dev/dbazure/download/playwright/builds/webkit/2364/webkit-ubuntu-24.04.zip

Archive SHA256: `043a4e597f3dc1db502103ec6ad06df6168dd1a400ab51894076007465e3f3f7`. Download size 107,337,541 bytes; archive declared expanded size 304,186,339 bytes. ZIP paths were validated as relative without parent traversal before extraction. Adapted WPE tree occupies approximately 150 MiB. Both sizes are below the authorized limits.

The separate `/var/tmp/rom-studio-webkit-2364/pw_run.sh` copies WPE only. `adapt.mjs` applies the same pinned Nix loader, library-store list, four executable interpreter/rpath edits, and MiniBrowser LD_LIBRARY_PATH insertion as the existing repository preparer. It checks every required store exists and rejects an existing destination or changed upstream wrapper contract. There are no new rendering switches, forced clicks, changed assertions, changed timeouts, retries, package installs, SDK changes, or application source changes. `rom-runtime.json` records raw and adapted WPE tree hashes. The downloaded GTK tree is not executed.

## Application identity and execution

Tests execute from the retained source archive `/var/tmp/rom-003-preview-application-584c01b/source/studio`, with its installed locked Playwright 1.63.0 client. `application-identity.json` records the unchanged optimized native SHA256 `ddeda5f8674333da679703674d679173553fad6294b0a559b7c22b8525109c2b` and freshly built explicit demo asset inventory. Every application asset and native hash was independently verified against the earlier acceptance receipt before the probe.

Focused command: existing actual-host semantic cases for both SQLite and redb, WebKit only, `--repeat-each=5`, one worker, list/JSON reporting. Results are recorded in `focused.log`, `focused.json`, and `focused-results/`. Full 28 cases may execute only after all ten focused cases pass.

Focused result: all10 cases passed in138.35 seconds, with zero skipped, unexpected, or flaky results. Exit0. Full28 result: all28 actual-host cases passed in317.75 seconds, exit0, zero skipped, unexpected, or flaky results. This includes both browser engines and both storage backends. The archived source, native bytes, and all app assets were rehashed after the run and are unchanged. Raw evidence is in full.log, full.json, and full-results/. Final complete producer, independent artifact verifier, deployment, and current live acceptance remain outside this probe and pending owner integration. A passing result with2364 must never be attributed to2359 or described as the locked default browser runtime passing.

## Limits

These actual host cases can establish compatibility for their exercised unthrottled browser protocol paths with locked client1.63. They do not establish every protocol feature or every Safari platform. Upstream2364 includes an optional Network bandwidth/latency protocol change; no unexecuted throttling compatibility claim is made. Finite passing repetitions support the targeted remedy and cannot prove universal absence of future native crashes. Historical failed2359 suites remain failures.


## Later integration result — 2026-10-05

The probe above ended before deployment. Its original pending integration statement records that boundary.
The completed producer and independent artifact verifier passed for source `584c01b`.
The [live preview receipt](preview-acceptance.json) now records successful activation and full container restart.
All 14 baseline authorized Resources retained their values and revisions.
The [attachment receipt](attachment-preservation.json) confirms unchanged attachment paths and bytes at both stages.
Both application and provider services are active and enabled.
The executed WPE 2364 application tests remain distinct from the producer’s original runtime acceptance.
