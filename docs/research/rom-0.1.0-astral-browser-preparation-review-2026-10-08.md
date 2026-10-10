# Astral browser preparation: independent review

Date: 2026-10-08.

## Outcome

No blocking preparation defect was found for the isolated loopback HTTP trial.
The reviewer did not start the application, browser, or provider. No native build ran during this review.

Reviewed sources:

- `tests/astral-adoption/prepare-browser.mjs`
- `tests/astral-adoption/browser-login.mjs`
- `tests/astral-adoption/provider-redirect.mjs`

The prepared capsule is `.worktrees/astral-010-public-adoption/web/.superpowers/rom-actual-recovery-lowXAY`.
Preparation is not a passing browser gate or ROM 0.1.0 release admission.

## Independently checked preparation

All 22 installed frontend asset hashes match `preparation.json`.
The original recovery source, generated source, and host configuration hashes match their recorded values.
The copied recovery source matches `/root/astral-plane/web/tests/recovery.spec.ts`.
Its SHA-256 is `18bb8c504b7f4cbe4cb7870c92a67e66d446ca365c2b7ce92aafbb9dbfe63cf0`.

The generated file retains the original imports and complete source after the login helper.
Only login is replaced, with two required imports added.
Three test declarations expand to four cases because the first declaration runs for create and delete.
The application code and original recovery source are not patched by preparation.

The identity proof file hash matches the preparation witness.
The proof records exit code zero and `accepted: true`.
The referenced private input hash matches the exact bytes used to derive the configured subject.
The configured subject equals the subject in that verified input.
This check uses the prior accepted proof; it is not a fresh provider verification.

The capsule directory has mode `0700`.
The client secret, login input, host configuration, generated test, browser configuration, and preparation witness each have mode `0600`.
Secret contents were not printed during review.

The binary SHA matches the previously independently reviewed package-adoption executable:
`6dc6d80f4f5423e3477541ad3e6be122f5d5e06070063d053c8aad7e5eff707b`.
The preparation script rechecks that binary against adoption acceptance before preparing inputs.

## Real and injected boundaries

The replacement login follows actual Authentik navigation and requires a 303 application callback.
It checks the actual application session identity and a host-only HttpOnly session cookie.
The explicitly supported HTTP loopback profile requires a non-Secure cookie. It does not prove production TLS behavior.

The preserved create and delete cases fetch the actual mutation response, then intentionally lose its acknowledgement.
Their retries and reload checks use the actual application.
The conflict case injects a 409 response. It tests client recovery, not an actual database conflict.
The session case injects one session request failure, then forwards an actual fetched response.
It is therefore incorrect to describe every response as uninjected.

The recorded host smoke requires HTTP 200 HTML and an unauthenticated HTTP 200 session.
Its HTML hash matches the prepared `index.html` hash.
The smoke record reports listener ownership, owned shutdown, and target absence.
This review read that record; it did not independently repeat the host launch or shutdown.
The smoke explicitly records `provider_login: false`.

## Checks before actual execution

Use an exclusively owned synthetic provider and the exact allocated callback.
Recheck source, asset, binary, and private-input fences before and after execution.
Require at least the planned 285000 ms provider window before starting the 240000 ms browser phase.
The caller must enforce the phase deadline and own browser, host, and provider cleanup.
The preparation script itself does not enforce these execution boundaries.
Bound logs and failure artifacts. Keep private artifacts inside the private capsule.

The reviewer reran the redirect scope tests: 12 passed, with no failures or skipped tests.
The separate provider scope report records their restoration guarantees and remaining limits.

One minor assertion can be stronger: `browser-login.mjs` accepts an empty string as `csrf_token`.
Require a nonempty token if the browser gate claims a usable authenticated mutation token.
Actual successful mutation cases would provide additional evidence, but preparation alone does not.

## Limits

The preparation uses dirty candidate packages with version 0.0.3.
It does not admit final 0.1.0 package payloads.
It does not establish successful provider login, browser recovery, production TLS, or deployment behavior.
These claims require the future bounded gate and its execution evidence.

## Initial actual runner review

The reviewer checked `tests/astral-adoption/run-browser.mjs` and `tests/astral-adoption/admission.mjs` before actual execution.
Node syntax checking passed. All four maintained admission tests passed.
The new `rom-actual-recovery-HjNx1f` preparation requires a 330000 ms remaining provider window.
The earlier 285000 ms preparation is not suitable for this runner's admission contract.

Two findings prevent an unconditional runner recommendation:

1. `sourceAudit()` initially omits the Playwright configuration and imported login helper source.
   Four unrelated passing cases could satisfy the aggregate result while the witnessed generated recovery file remains unchanged.
   Bind configuration, helper source, and private login input hashes before and after execution.
   Also require `ready.unique === providerId` before provider access.
2. Final `socket()` and `privateBytes()` observations can throw before `execution.json` is written.
   A symlink detected by the artifact monitor would also throw during final inventory.
   This can replace the primary failure and omit the failure record.
   Record failed observations without replacing the primary cause.
   Reject success after interruption or artifact-budget flags.

The initial artifact measurement also precedes final log and execution-record writes.
Reserve their bounded overhead or report a complete final measurement.
The 250 ms artifact monitor provides a detection threshold, not a hard allocation quota.

The runner reuses owned process-group cleanup and verifies actual host identity and listener ownership.
The external provider caller still owns the provider window and its cleanup.
No provider, browser, or native build was started during this source review.
These findings were sent to the runner owner before actual execution.

## Corrective runner review

The reviewer checked the corrected runner and `tests/astral-adoption/source-fence.mjs`.
The fresh capsule is `rom-actual-recovery-hmzXFq` under the same private consumer directory.
Its witness binds 20 source inputs. The reviewer independently checked every recorded hash.
These inputs include the browser configuration, private login input, client secret, runner, and imported local helpers.
The runner checks them before and after execution. It also checks the binary and complete frontend asset inventory.
Provider admission now requires `ready.unique === providerId`.

Final observation errors are categorized without replacing the primary error.
They change the final status to failed.
Interruption and artifact-monitor flags also prevent success.
The runner writes the bounded host log before measuring artifacts and reserves 65536 bytes for the execution record.
If execution-record writing fails, it attempts a separate private failure record and returns a failing exit status.
If both evidence writes fail, success remains refused; guaranteed evidence persistence is not claimed.

Node syntax checking passed again.
The independent command `node --test tests/astral-adoption/*.test.mjs` passed all 20 tests, with no failures or skipped tests.
The source-fence tests cover changed configuration and helper inputs, missing closure, primary-error preservation, and failed final observations.
These are native-free synthetic checks. They do not execute the live runner.

The two initial P2 findings are resolved within this scope.
No source-review blocker remains for the finite Chromium recovery trial.
This recommendation requires the exclusively owned provider, 330000 ms admission window, outer supervision, and existing cleanup controls.
The artifact monitor remains a soft detection limit, not a hard quota.
No provider or browser was launched by the reviewer. Release admission remains false.

## Actual first-attempt limit

The preserved `rom-actual-recovery-hmzXFq/execution.json` records a failed initial provider-schema check after one HTTP 200 GET.
The application and browser did not start. There was no redirect mutation to restore in that attempt.
The provider schema correction and fresh source closure are reviewed in the separate provider scope report.
All 20 hashes in the new `rom-actual-recovery-Iy8pqs` witness matched during review.
The reviewer reran all Astral Node tests: 22 passed.
The next actual trial still requires a new owned provider window and prior stop reconciliation.

## Actual Chromium proof audit

The completed trial used `rom-actual-recovery-Iy8pqs` and provider `535112ef70cb20ae996b780cc9f1c655`.
The reviewer read its execution record, command record, raw browser log, detailed results, and provider terminal record.
No additional application, browser, or provider was started during this audit.

The raw log lists all four original Chromium recovery cases as passing.
Detailed results contain four expected cases, zero unexpected cases, zero skipped cases, and zero flaky cases.
The browser command exited zero, with no signal, stop reason, spawn error, or output truncation.
Its owned process group cleanup reported absence.

The create and delete cases used actual login and actual committed mutations before the response was deliberately lost.
They verified an identical retry request, unchanged idempotency key, expected visible count, and the count after reload.
The conflict case still injects its 409 response. The session case still injects one failed request before actual refresh recovery.
These limits remain part of the successful proof.

All 20 preparation source hashes still match.
All 22 installed asset hashes still match.
The original and copied recovery source remain equal.
The recorded host log hash matches its file.

The control record contains five successful provider calls: GET, PATCH, GET, PATCH, GET.
The reviewed helper requires exact activation and restoration readback.
Its execution controls record `redirect_restore` with `restored: true`.
Raw redirect arrays are intentionally absent from evidence. The equality claim follows the witnessed helper and successful control sequence.

The host target, wrapper, and browser group leader PIDs are absent at independent review time.
The execution record reports host shutdown drained, host target absent, and consumer endpoint vacant.
The provider terminal record reports all three containers stopped and its cgroup `populated 0`.
All recorded provider source hashes match before, after, and current files.
The three old container PIDs are absent at review time.
These checks do not assume that unrelated later provider ports remain vacant.

The capsule contains 15 files with 123040 logical bytes at audit time.
The recorded 121120 bytes preceded the 1920-byte execution record.
The complete total remains below the 65536-byte execution reserve and the soft 64 MiB artifact limit.

| Evidence | SHA-256 |
| --- | --- |
| Preparation | `989d1b12de27217eb18705cc70c2b849edfc069438876669d0b5f7afed7537cb` |
| Execution | `b1dfebf7b3ea0166d5c884ae1b96f0a0050793c240976b4c0370cd0ef8cd02d5` |
| Browser command result | `0cf264204e58a369adf4364e8cb59bc71d0db8465e76f65792440bf0dd3a0b42` |
| Detailed browser results | `62912e57fcfd37c15c634f30eb975e360fc324e9249ee720b0085d3f252fe9e0` |
| Raw browser stdout | `2b906898136d73f31de57ff010085b1842021bc0c9c5f7bd77c07b97f2f8f310` |
| Provider terminal | `4a6378ddaa3d21d5d123beec73a8a5431b9d0047e319c6ac91d38128a9503885` |

No blocker was found for this completed Chromium recovery proof.
WebKit remains unverified in this report.
The proof uses loopback HTTP and dirty version-0.0.3 candidate packages. It does not admit a production TLS deployment or final 0.1.0 release.

## Actual WebKit proof audit

The WebKit trial used `rom-actual-recovery-XnwOYb` and provider `cca0ef193f90e87cdb0102ff0727fde1`.
Its raw log lists the same four original recovery scenarios as passing.
Detailed results contain four expected cases, zero unexpected cases, zero skipped cases, and zero flaky cases.
Reported browser duration is 44567.45 ms.
The command exited zero, with no signal, stop reason, spawn error, or output truncation. Owned group cleanup reported absence.

All 20 source-input hashes and all 22 installed asset hashes matched during independent review.
The actual executable hash still equals `6dc6d80f4f5423e3477541ad3e6be122f5d5e06070063d053c8aad7e5eff707b`.
The original and copied recovery source hashes still match the value recorded above.
The host output hash matches its preserved file.

The provider control sequence contains five HTTP 200 calls and records successful exact restoration.
The host target, wrapper, and browser group leader PIDs are absent at review time.
The provider terminal record reports three successful container stops and cgroup `populated 0`.
The old container PIDs are absent. Provider source hashes match before, after, and current files.
The recorded consumer endpoint is vacant.

The complete capsule contains 15 files and 123015 logical bytes.
Its 121096-byte measurement preceded the 1919-byte execution record.
This total fits the execution reserve and the soft artifact limit.

| Evidence | SHA-256 |
| --- | --- |
| Preparation | `788ea661ecd0ed112d606e990cbba800f808d53885da409a57beae4aa4495ce0` |
| Execution | `fd97b46db20f1a96848940eedab75ee2ad6eb424238673c0e16baa74be2d757b` |
| Browser command result | `e6064fede300ee3183831f9522c40747dc4202e4f5af6bf2de98e2d8b22b7b15` |
| Detailed browser results | `15890eb82e67531b9ddd08c8dffbeccec213e189b2632d8ab42fb463cfef6dcc` |
| Raw browser stdout | `dd20d77f98000321b7b871daa9feab0db36fd8f7497202031de10ca74f6ad848` |
| Provider terminal | `9c5ff702e973c4efffd60bc226d192f06e6d35c20d86dc5ec4513fb0d23101d1` |

No blocker was found for the completed WebKit proof.
The two browser trials establish eight successful executions of four scenarios, not eight distinct scenarios.
The injected conflict and session-failure limits remain unchanged.
Both trials use loopback HTTP and dirty version-0.0.3 candidate packages. Final 0.1.0 package admission and production TLS remain outside this proof.
