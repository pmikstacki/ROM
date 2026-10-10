# Installed Studio session investigation

Date: 2026-10-09. This is an exploratory consumer investigation, not release acceptance.

## Initial installation

A separate application imports the public `rom-studio` package, `App`, `createStudioBootstrap`, and styles.
The installation uses a source archive and an extracted SDK. It does not use a private Studio import.
The initial frozen installation selects `rom-ui` version `0.1.0-alpha.3`.

Installation, Svelte check, and Vite build passed. Each command physically closed, and its owned cgroup drained.
The measured frontend directory growth was 346,718,208 bytes. This is a measured allocation, not a quota guarantee.
The [execution record](/root/ROM/.superpowers/rom-010-installed-studio-expiry-frontend-execution-20261009/install-result.json) retains the commands and source fence.
This archive comes from the current working source. It is not a clean release artifact.

## Test corrections

The original browser test compared `current.user` and `original.user`. The session response uses `user_id`.
The execution copy now requires a nonempty `user_id` and checks that it remains unchanged.
It also checks session generation, cookie identity, and original token expiry.
The original preparation remains preserved.

The first runner selected no tests because its expression did not match Playwright's complete test title.
The corrected runner selects one named case. The no-test result remains a failure, not a browser success.

## Provider clock

The first actual Chromium renewal case stopped at the provider interaction.
Its page showed `Fixture interaction rejected`. The private provider used a fixed clock of 100 seconds after the Unix epoch.
The installed cookie implementation computes its expiry from `Date.now()` plus `maxAge`. Therefore the fixture produced dates in 1970.

The corrected native fixture starts at the current Unix epoch. Its controlled clock retains the same relative progression.
The provider signs with that initial epoch. The original token lifetime remains 300 seconds; the short authority proof remains 30 seconds.
No production ROM implementation or installed provider dependency was patched.
The corrected native fixture compiled, and its consumer check and build passed.

## Integration failure after login

The corrected Chromium/SQLite case completed real provider login and opened its first live query.
After the controlled authority-proof expiry, the observer recorded one `identity_expired` event and stream termination.
The test required a second stream with an authorized snapshot. It observed only one stream and failed.
This execution did not pass a browser case.

The [failed browser record](/root/ROM/.superpowers/rom-010-installed-studio-expiry-frontend-execution-20261009/journey-1791530308447/result.json) retains the actual result and physical closure.
The final observer recorded one snapshot, one expiry, and one ended stream. It retained no data payload.

Source investigation identifies a lifecycle conflict to test next.
`createAppSession` refreshes the session when the controller attempts recovery.
The refresh transition calls session fencing, which aborts the current subscription and changes its epoch.
The observer rejects continuation after that epoch change. Session rebinding does not restore the observation intent.
This source explanation needs a narrow regression before a production correction is accepted.

## Initial follow-up requirements

- Reproduce the managed session recovery conflict with a regression test.
- Preserve explicit stop, navigation, logout, revocation, and caller cancellation during any correction.
- Verify that recovery requires the same authorized owner and a fresh session result.
- Run the installed renewal case again, then the negative and cancellation cases.
- Repeat the actual cases on redb and WebKit. Neither backend nor browser coverage can be inferred from Chromium/SQLite.
- Run the affected checks and full local verifier after the correction.

The proposed matrix contains 20 cases. Those initial executions did not complete the matrix.
Trusted TLS identity, full mixed load, native SQLite qualification, clean packaging, and publication remain separate release gates.

## Managed observation correction

The regression uses the real application controller, managed session lifecycle, and client SSE decoder.
The initial regression failed before the correction. The corrected controller preserves observation intent across session fencing.
Explicit stop, navigation, owner changes, denial, and destruction cancel that intent.
A recovery requires an active managed session. Without a new snapshot, a repeated expiry exhausts its bounded recovery.

Independent review found two additional races during synchronous callbacks.
One callback changed the session during publication. Another changed it during transport abort.
The controller now verifies ownership tickets and current session state after each callback boundary.
Both regression groups failed before their corrections and passed afterward.

The [review record](/root/ROM/.superpowers/rom-010-managed-live-renewal-20261009/final-review.md) reports 16 passing scoped tests.
The complete Studio unit suite passed 432 tests. Svelte reported zero errors and zero warnings.
The [full local verifier](/root/ROM/.superpowers/rom-010-managed-live-final-verifier-20261009/result.json) also passed with an unchanged source fence.
This verifier uses bundled SQLite. It does not qualify the separate native SQLite version.

## Current installed consumer

A new archive includes the final guards and `rom-ui` version `0.1.0-alpha.7`.
Frozen installation, Svelte check, and Vite build passed for this separate consumer.
An [independent source comparison](/root/ROM/.superpowers/rom-010-installed-studio-renewal-final-20261009/installed-source-audit.json) verified all 409 installed SDK source files against the extracted archive.
It also verified package exports and the locked UI archive URL, integrity, and installed version.
These results do not establish complete dependency advisory coverage or clean release provenance.

## Completed browser matrix

The [matrix record](/root/ROM/.superpowers/rom-010-installed-studio-renewal-final-20261009/matrix-v2-result.json) contains 20 passing cases.
Each adapter, SQLite and redb, ran each case in Chromium and WebKit:

- Renewal after authority-proof expiry, with the same user, generation, cookie identity, and original token expiry.
- Revocation of the identity link.
- Revocation of the identity provider.
- Expiry of the original token.
- Cancellation while renewal is pending.

The matrix uses a real signed OIDC provider and the installed public Studio application.
Each native Host and browser process physically closed. Their owned cgroups drained.
The maintained source fence remained unchanged.
The native fixture retains the 300-second original token lifetime and 30-second authority-proof lifetime.
This profile uses ordinary loopback HTTP. It does not replace trusted TLS acceptance.

The first matrix stopped after ten passing SQLite cases.
The first redb case failed because the diagnostic writer replaced `trace.json` between `lstat` and `open`.
The preserved failure is not a passing redb result.
The private harness now retries that exact replacement error at most three times for diagnostic snapshots only.
Readiness files and mailbox acknowledgements retain strict single reads.
Five deterministic tests cover replacement, retry exhaustion, permissions, symlinks, and hardlinks.
The corrected matrix then completed all 20 cases. This correction changes no production ROM behavior or browser assertion.

Full mixed load, trusted TLS, recovery acceptance, native SQLite qualification, clean packaging, deployment, and publication remain open release gates.
