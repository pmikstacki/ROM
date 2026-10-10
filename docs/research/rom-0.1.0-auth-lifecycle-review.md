# Public session lifecycle candidate review

Date: 2026-10-07. Status: independently reviewed candidate, not release acceptance.

The reviewer inspected the public facade, lifecycle, browser protocol decoder, transport, cancellation, clock, and identity checks.
The facade exports `createSessionLifecycle` and `createBrowserSessionDriver` through `rom-studio/auth`.
Its observable state contains no CSRF credential. The browser driver keeps credentials in a private closure.

## Review correction

A custom driver's throwing or rejected logout could retain local credentials after observable identity was cleared.
The worker reproduced both cases before the correction. The lifecycle now starts logout, then clears local credentials before host hooks.
This order preserves the browser driver's captured logout token. Epoch checks prevent a superseded callback from restoring authority.
Unknown remote logout results remain unconfirmed; local invalidation does not prove remote revocation.

## Independent verification

The reviewer checked all 10 frozen source hashes against the worker's corrected record.
The independent rerun passed 24 targeted tests and 208 Studio unit tests.
Type checking reported zero errors and zero warnings.
Public import first failed with `ERR_PACKAGE_PATH_NOT_EXPORTED`. It passed after the package export was added.

The minimal external consumer completed locked installation, source matching, type checking, and build.
It uses public `/auth`, `/client`, `/recovery`, `/controls`, and root exports without private aliases.
Its existing mutation composition case also exercises session acquisition, transient retention, and local logout through a fabricated driver.
The installed auth implementation hashes match the corrected frozen source.

Two separate browser runs each passed 12 cases across Chromium and WebKit, without skips or flaky results.
One used candidate WebKit 2364. The second used WebKit 2359, which matches the committed runtime and Playwright 1.63.0 selection.
These runs establish the tested installed consumer behavior, not every browser runtime or production identity journey.

Evidence: `/var/tmp/rom-auth-root-review/result.json` contains source hashes, log hashes, commands' output locations, and limits.
Installed source evidence: `/var/tmp/rom-auth-installed-root-20261007/result.json`.
Worker RED and corrected evidence: `/var/tmp/rom-auth-lifecycle-evidence/review-evidence.json`.

## Remaining acceptance

The original review left legacy delegation open. The subsequent candidate now shares protocol decoding and acquisition with the public driver.
The public R4 renewal recipe and actual application view sequencing remain open.
Production-provider identity, clean artifacts, deployment, and human usability acceptance remain separate requirements.
No feedback group or release gate is closed by this candidate review.

## Legacy delegation: independent review

The reviewer inspected the shared protocol, transport, public driver, legacy adapter, and 14 new regression cases.
The shared acquisition supports separate waiter cancellation. It cancels the driver when no waiter remains.
Both adapters check their epoch after the shared transport completes. Logout cannot republish a completed earlier session.
Provider responses have a 100-choice bound, unique identifiers, and a valid primary selection.
Legacy anonymous generation, expiry exception, and encoded login URLs remain compatible.
Public identity authority remains an explicit host input. Neither adapter derives it from credentials or a URL.

All 18 frozen source hashes matched before and after the independent checks.
The independent rerun passed 45 targeted tests and 222 Studio unit tests.
Type checking reported zero errors and zero warnings.
The separate release artifact suite passed 60 tests. These tests do not compile a clean release consumer.

A fresh external consumer completed locked installation, source matching, type checking, build, and 12 browser cases.
Six cases passed in Chromium. Six passed in WebKit 2359, matching the committed runtime selection.
The verifier reported `admitted: false` and `source_mode: authoring_checkout`.
These browser cases exercise public surfaces and a fabricated lifecycle. They do not establish actual App session sequencing or provider behavior.

Independent evidence: `/var/tmp/rom-auth-legacy-root-review/result.json`.
Installed consumer evidence: `/var/tmp/rom-auth-legacy-installed-root-20261007/result.json`.
The App still disconnects its controller after refresh exceptions and generation changes.
That behavior can clear selection, history, and pending mutation state. The reviewed adapter does not correct it.
`AP-UX-004`, `AP-UX-005`, and `AP-UX-006` require separate App and recovery integration tests.
