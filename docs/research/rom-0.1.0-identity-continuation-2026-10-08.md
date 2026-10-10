# Identity continuation, 8 October 2026

## Scope and executed result

The continuation implements the existing isolated TLS profile in the real-provider HTTPS fixture.
It does not change ROM authentication code, global trust, package locks, or a live application.

The maintained Node suite passed 89 tests with no failures or skips.
The new browser-command selector first failed because its module was absent.
Its three focused cases then passed.

Evidence logs:

- `/var/tmp/rom-010-identity-browser-profile-red-20261008.log`
- `/var/tmp/rom-010-identity-browser-profile-green-20261008.log`
- `/var/tmp/rom-010-identity-continuation-node-20261008.log`

The real Authentik HTTPS journey passed four cases: SQLite and redb, each with Chromium and WebKit.
Both owned processes exited with status zero.

Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-e0398d2697d190a3e2882a84/result.json`.
Logs: `/var/tmp/rom-010-identity-patched-https-historical-20261008.log` and `/var/tmp/rom-010-identity-patched-provider-window-20261008.log`.

Each case exercised original provider callbacks, protected reads, rejected Origin/CSRF, and current User/link/provider revocation.
Reactivation did not restore the old session. Fresh provider login restored access.
Local Host logout returned 204 and removed protected access.

WebKit loaded the previously admitted private GnuTLS and libtasn1 libraries.
Actual process mappings identify both libraries in each WebKit result.
Each WebKit process entered a fresh private mount namespace with a read-only fixture trust directory.
The host trust namespace, CA path, and CA hash remained unchanged.
No certificate verification was disabled.

All eight case process groups drained.
The provider launcher stopped its three owned containers and exited.
A fresh post-exit record confirms provider drain and vacancy of ports 44389–44393.
All stopped containers, private volumes, original responses, failed records, and acquisition evidence remain retained.

## Source admission limits

This run deliberately used the historical compiled Host.
Its immutable binary still matches build identity `host-corrected-build-048d49b41ef305ffdcfc99f1`.
The current `crates/rom/src/resource/definition.rs` differs from that build's source witness.
The result records that exact mismatch and separately records unchanged current sources during execution.
The current fixture closure and binary hashes remained unchanged during the run.

This is historical-Host transport compatibility evidence, not current-source R7 admission.
It does not establish extracted artifact admission, current consumer usability, or production dependency approval.
The patched crypto supplement does not resolve every advisory for the retained GLib and browser runtime.
The previously executed eight TLS positive/negative controls remain separate prerequisite evidence.

The next current-source run requires a new source-fenced Host build under the coordinator's native allocation.
Do not relabel this result after that build.

## Remaining lifecycle cases and bounded next steps

| Case | Inspected implementation | Next executable oracle |
| --- | --- | --- |
| JWKS outage | OIDC re-verifies after the cached proof expires. `SessionStore::failed` removes sessions on `Denied` or `Panicked`. Network classification requires an actual failure. | After login, interrupt the owned provider endpoint, wait beyond the measured proof interval but before original token expiry, and attempt a protected read. Require bounded denial without disclosure, restore endpoint, and test whether the same session recovers. Preserve session-loss RED before proposing Rust changes. |
| Provider restart | `ProviderEngine::observe` rejects changed init birth identity. Restart cannot reuse the old admitted PID as current ownership evidence. | Add a closed fixture restart operation with fresh init and conmon admission. Prove retained provider state and new original callback after restart. Keep old and new births as separate records. |
| Token expiry | OIDC checks configured clock against original expiry before proof reuse. | Configure an actual short-lived provider token, capture only public expiry timing, wait using wall-clock time, and require denial with no protected value. Do not alter token bytes or extend proof lifetime. |
| Key rotation | Original JWKS and signing-key changes must come from provider administration. The existing fixture has no rotation test. | Read the synthetic provider's admitted signing-key configuration, create a fresh provider key, perform explicit cutover, and observe unchanged old-token behavior and fresh-login recovery. Do not assume an overlap window. |
| Provider denial and unlinked identity | Current fixture repeatedly authenticates the linked synthetic administrator. | Provision separate fresh denied and unlinked synthetic accounts with explicit grants. Require no linked Host session or protected disclosure. |
| Provider-wide logout | Only local Host logout was executed. | Keep provider-wide logout unsupported until its own public contract and provider behavior are implemented and tested. |

These steps remain incomplete. Source inference is not an observed regression.

## Resource and coordination boundaries

The fresh admission checked the isolated engine only; it found no running containers.
The provider cgroup was drained, and all five allocated ports were vacant before launch.
The run filesystem had 7,131,934,720 free bytes.
The retained acquisition/run envelope remains 32 GiB with its existing hard filesystem limits.
No new image, Rust build, or supplemental library was acquired.
No other project, process, global trust store, or existing evidence was modified.

AP-UX-006 session boundaries, AP-UX-013 observation, and AP-UX-021 ownership/privacy remain open for original-consumer acceptance.
The protocol matrix does not prove that an application clears stale private views.

## Actual outage and restart RED

A fresh maintained lifecycle supervisor owned PostgreSQL, server, worker, relay, and its private mailbox.
The fixture stopped only its exact admitted Authentik server.
PostgreSQL and worker remained running in the same owned deployment.
The ordinary four-case matrix ran again, with the outage oracle added to SQLite/Chromium.

Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-9754041548888f850390407d/result.json`.
The matrix had one intended failure and three passes.
The browser matrix exited with status 1. The provider supervisor exited with status 0.

The original provider token had `iat=1791442997` and `exp=1791443297`.
The fixture retained only timing and a token hash, not the token or other claims.
It waited until original response receipt plus 32 seconds, beyond ROM's existing 30-second proof bound.
The protected request remained before original token expiry.
The stopped-provider request returned 401 in eight milliseconds, without protected content.

The supervisor restarted the same immutable owned server container.
It admitted new init and conmon births: 538412→542886 and 538410→542883.
The provider network namespace remained bound to the unchanged PostgreSQL namespace.
After provider readiness, the original session still returned 401 without protected content.
A fresh original callback returned 303 and restored an authorized protected read with status 200.
This reproduces session loss on a temporary provider outage.
It also proves real server restart and fresh-login recovery for this historical-Host case.

The original historical-binary and artifact limitations remain unchanged.
The first lifecycle supervisor did not yet capture its complete imported helper closure.
The maintained supervisor now captures that closure before launch and after drain for the next run.
Do not inherit this improved source admission into the earlier result.

A separate post-exit check confirmed the new provider cgroup was drained and all five ports were vacant.
The private mailbox has a four-request maximum: two exact server pause/resume cycles.
It rejects extra IDs, unsupported actions, sequence drift, symlinks, changed file inodes, and overlapping polls.
The overlapping-poll regression had one pass and one intended failure before its guard; both cases then passed.
The maintained Node suite subsequently passed 94 tests.

## Narrow acquisition classification correction

Investigation read the exact locked `reqwest-0.13.5/src/error.rs` source.
`is_connect` walks Hyper connection errors and does not isolate TLS validation from transport availability.
The correction therefore does not use a blanket `is_connect` rule.

Three maintained native regressions failed for their intended classification assertions before the correction:
HTTP 429, a refused TCP connection, and a timed-out acquisition all returned `Denied`.
After the correction, all three tests passed.
The same tests reject non-temporary HTTP statuses, malformed JWKS, and plaintext sent to a TLS endpoint.
The timeout test also proves that retaining session context does not extend original expiry.

The correction maps HTTP 429 and 5xx to the existing `Overloaded` result.
A maximum 16-source error walk recognizes specific network I/O failures and timeouts.
`InvalidData` and `InvalidInput` take precedence over temporary errors.
Unknown errors and excessive source chains remain `Denied`.
No cached authorization proof, expiry extension, new public error, or provider-selected trust source was introduced.

Native logs:

- `/var/tmp/rom-010-identity-acquisition-classification-red-20261008.log`
- `/var/tmp/rom-010-identity-acquisition-classification-green-20261008.log`

Current-source actual-provider recovery requires a rebuilt source-fenced Host and remains pending.

The complete affected Host suite passed 44 tests: 31 unit tests and 13 integration tests.
Clippy passed for all Host targets with warnings denied.
The first Clippy invocation rejected an ignored read amount in the new test helper.
The helper now asserts a non-empty read. The final suite and Clippy run passed together.
Final log: `/var/tmp/rom-010-identity-acquisition-affected-final-green-20261008.log`.
The earlier Clippy failure remains in `/var/tmp/rom-010-identity-acquisition-affected-green-20261008.log`.

The native TLS negative regression sends plaintext to a TLS endpoint.
It proves rejection of that protocol error, not a complete certificate-validation matrix for the changed Rust acquisition path.
The browser certificate controls remain separately scoped evidence.
The full release verifier, independent review, current-provider recovery matrix, and extracted artifact admission remain pending.

## Original-consumer source follow-up

Read-only investigation inspected Astral Plane's `web/src/App.svelte` and `web/tests/session-navigation.spec.ts`.
The application preserves its open view during a background acquisition failure.
It clears private views for an anonymous response or the exported `SessionExpiredError`.
Its existing navigation tests use routed synthetic session responses, not this actual-provider fixture.

Current ROM `studio/src/lib/auth/browser-response.ts` distinguishes confirmed denial from temporary 503 failures.
The lower-level transport clears CSRF after confirmed denial.
However, `studio/src/lib/auth/legacy-adapter.ts` turns `denied` into a generic error.
Astral Plane's catch branch does not clear private views for that generic error.
This is a source-derived compatibility concern for AP-UX-006/013/021, not an executed privacy regression.

The coordinator received this finding for separate UI ownership and public-consumer regression work.
No Astral Plane or Studio source was changed by this identity continuation.
Original-consumer acceptance remains separate from the Host acquisition correction.

## Prepared current-source build

The existing isolated Host manifest and lock remain the build inputs.
The exact command is:

```sh
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo build --manifest-path tests/identity/provider/host/Cargo.toml --locked --offline --target-dir /workspace/ROM/target
```

Execute it inside `rom-dev` only after the coordinator releases the active AI native lease.
Before compilation, enumerate the current transitive source directories again.
Do not reuse the historical 228-file inventory as a completeness check.
Capture added files, both manifest/lock pairs, fixture assets, and the exact compiler command.
After compilation, re-enumerate and compare the complete witness before admitting the binary.

The prior binary resides at a mutable Cargo target path.
Preserve an exclusive detached copy and its hash before rebuilding that path.
Preserve the newly compiled binary in a separate private directory before any actual browser run.
The new preservation helper caps binary size at 128 MiB and verifies an exclusive private destination.
Its three regression tests passed after the intended missing-module RED.

The 32 GiB retained fixture envelope bounds private copies and logs.
It does not independently impose a hard growth limit on the shared native target.
The coordinator must admit that remaining allocation uncertainty before compilation.
No new native compilation was started while the AI lease remained active.

## Executed current-source recovery verification

The first preservation attempt rejected the 180,316,864-byte historical binary under the provisional 128 MiB guard.
No historical binary was overwritten during that rejection.
The coordinator then approved a 256 MiB limit per preserved binary, with two copies maximum.
The helper copies and hashes through a 64 KiB buffer. It verifies the exclusive destination inode.
Both preserved binaries remain inside the retained private fixture envelope.
The shared native target still has no independent hard growth cap from this helper.

The admitted current-source build used this exact command inside `rom-dev`:

```sh
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 timeout --signal=TERM --kill-after=5s 120s cargo build --manifest-path tests/identity/provider/host/Cargo.toml --locked --offline --target-dir /workspace/ROM/target --message-format=json
```

Compilation completed in 8.76 seconds with exit status zero.
The fresh inventory contained 230 source and build-input paths.
The inventory was unchanged before and after compilation.
The build record preserves separate historical and current binary copies.
Current binary SHA-256: `2f50b17117c13a77f3de73e1481d88d052979b18fdadaee1186453f306500984`.
Build record: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/host-corrected-build-b5fef228e869cf9943a3daa6/build-identity.json`.

All four current-source provider cases passed: SQLite and redb, each with Chromium and WebKit.
The SQLite/Chromium case also stopped and restarted the actual owned Authentik server.
PostgreSQL and the provider worker remained unchanged during this sequence.
The test waited beyond the 30-second proof window while the original token remained unexpired.
During the outage, the protected read returned 503 without the protected value in 6 ms.
After restart, the existing session returned 200 with the protected value.
A separate fresh login also returned 200 with the protected value.
This reproduces the historical session-loss failure and verifies the current acquisition correction.
The outage sequence was not repeated in the other three cases.

Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-7fcafc351334604da10bbf69/result.json`.
Both source fences remained unchanged. The preserved current binary remained unchanged.
The WebKit cases used the admitted private crypto libraries and read-only private trust namespace.
The host trust store remained unchanged.
The provider supervisor retained its complete helper-source witness before and after execution.
All three exact owned containers stopped successfully.
A separate post-exit check confirmed a drained provider cgroup and five vacant private ports.
Drain record: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/current-source-post-drain-337c6bc6c5739b2e19eb828b2d4825ec.json`.

This evidence covers source-authoring execution, not extracted release artifacts or deployment approval.
Actual token expiry, signing-key rotation, denied and unlinked provider accounts remain separate open cases.
Original Astral Plane consumer acceptance also remains open.
The coordinator reported an additive `SessionDeniedError` correction and 17 passing legacy-adapter tests.
Astral Plane still needs explicit confirmed-denial handling or lifecycle integration before privacy acceptance can close.

## Actual provider expiry experiment

The coordinator approved a finite validity change for this synthetic provider only.
The fixture records the original `access_token_validity` and sets it to `seconds=40` through the actual provider API.
Each browser reads timing from the unmodified original token response.
The test waits until original `exp` plus two seconds. It does not change tokens, clocks, or JWKS.
The oracle requires 401 without protected data, an anonymous session, and a successful fresh login.
The fixture restores the recorded original provider duration before requesting owned shutdown.
The provider duration is not a production configuration change.

The bounded expiry helper had an intended missing-module RED and then passed its regression.
The complete maintained provider Node suite passed 98 tests before the actual run.
Logs: `/var/tmp/rom-010-identity-token-expiry-red-20261008.log`, `/var/tmp/rom-010-identity-token-expiry-green-20261008.log`, and `/var/tmp/rom-010-identity-expiry-fixture-node-current-20261008.log`.

Read-only actual API inspection also identified the provider's `signing_key` and its certificate-keypair endpoint.
The inspection stored field names and non-secret configuration only.
The generation endpoint's OPTIONS response exposes the generic certificate serializer.
That response alone does not prove the generation request format.
Key cutover therefore requires further actual source or API inspection before a mutation.
The first read-only projection attempt used a response method instead of its status property and failed before saving evidence.
The corrected projection saved `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/provider-read-only-schema-20261008-1791444924275.json`.
The OPTIONS projection saved `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/provider-read-only-actions-20261008-1791444936012.json`.

The actual expiry matrix passed all four current-source cases across SQLite/redb and Chromium/WebKit.
Every expired token produced 401 without protected data and an anonymous session.
Every subsequent fresh login produced 200 with the protected value.
The provider duration was restored from `seconds=40` to its original `minutes=5` before shutdown.
Native and fixture source fences remained unchanged. The current binary and host trust store remained unchanged.
The supervisor stopped all three exact owned containers and retained an unchanged provider helper closure.
A fresh post-exit check confirmed the provider cgroup was drained and all five private ports were vacant.
Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-e250f4ca1f2834e4cc477396/result.json`.
Drain: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/expiry-post-drain-ae1f1dd02e86e63f076bb1795b823a15.json`.
Provider and matrix handles both ended with exit status zero.

A later read-only provider-source inspection rejected the server identity after owned shutdown had begun.
No source-reading exec ran against that stopped identity.
The fixture did not weaken its live process admission to continue inspection.
Signing-key inspection requires a fresh bounded provider window.

## Provider-source inspection and portable unit fixture

A fresh owned source-inspection window captured the actual `/authentik/crypto/api.py` file.
The read-only helper validated the exact container image, limits, init birth, and monitor birth before its exec.
The captured generation serializer requires `common_name` and `validity_days`; its algorithm defaults to RSA.
Source: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/provider-crypto-source-540da3338740e321fb6a7f41.txt`.
SHA-256: `71ec412f988cc1e534bcdef9778fe178a7eb0e5794c2727f33102f9c42f3a4b2`.
The helper ended with exit status zero and confirmed its inspection cgroup was drained.
An earlier import-based attempt failed because Django settings were not initialized.
Its immediate drain assertion also observed a non-empty cgroup.
That ad-hoc attempt did not record its group identity, so a later check cannot prove its specific drain.
The maintained helper records failures and performs a finite drain recheck; it does not stop an unowned process.
The source-inspection provider window then stopped all three owned containers.
Its complete provider helper closure remained unchanged during execution.
Drain: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/source-inspection-post-drain-2673faae1850f44ec907c47113ded76f.json`.

The coordinator's full verifier found an independent portability failure in the mailbox unit fixture.
The test assumed a historical provider directory existed inside `rom-dev`.
The focused container RED reproduced one pass and one ENOENT failure.
The unit now creates its own private OS temporary root and injects that approved root into the mailbox.
The production default remains the exact retained provider private root.
Root admission checks canonical paths, private permissions, owner UID, and child containment.
Adversarial tests reject the unchanged default for unrelated temporary paths, outside-root paths, non-private roots, and symlinked mailboxes.
The focused container suite passed three tests.
The complete current provider Node suite then passed 99 tests inside `rom-dev`.
Log: `/var/tmp/rom-010-identity-mailbox-portability-container-node-20261008.log`.
The coordinator must rerun the full release verifier. These affected tests do not replace that gate.

Actual signing-key cutover and denied/unlinked provider-account coverage remain the next fixture work.
The actual source inspection supplies the key-generation request contract; the next test must retain the generated key and restore the original provider key.
The fixture must observe original JWKS and token key IDs rather than substitute a JWKS response.
Original consumer privacy acceptance and packaged artifact admission remain separate release requirements.

## Actual signing-key cutover and bounded setup failure

The first actual cutover matrix completed with three semantic passes and one setup failure.
SQLite/WebKit and both redb browsers generated real RSA keys through Authentik's inspected API.
The original JWKS changed and removed the old token's key ID.
After the 30-second proof interval, the old session returned 401 without protected data.
Each fresh login returned 200 with an original provider token signed by the new published key.
Each case restored the original provider signing key. Generated keypairs remain in the retained synthetic database.

SQLite/Chromium generated its key, then exceeded the fixture's three-second signing-key PATCH deadline.
Its original key was restored. The failure did not reach the old-token revalidation oracle.
This is a fixture setup failure, not demonstrated Host rejection of a valid new token.
The later fixture uses an eight-second API deadline and records bounded method, status, and elapsed-time evidence.
It does not retry an uncertain key-control request.
Request bodies, API credentials, and private key material are excluded from these records.

Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-3ded691e0c89c86798600918/result.json`.
Native and fixture source fences, binary identity, and host trust remained unchanged.
The provider supervisor stopped all three owned containers with its helper closure unchanged.
Drain: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/rotation-post-drain-4140ff26b8b749e9b6da3e3892f0b0c6.json`.
This three-pass result is retained independently of the focused repetition.

A separate provider startup failed before any repetition began.
Its owned relay exited with status one; namespace mounting completed successfully.
The maintained readiness deadline ended, then all three owned containers stopped.
That fixture attached stderr after target admission and missed the early relay diagnostic.
The exact relay failure cause remains unproven.
Terminal record: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/authentik-terminal-5635705ec22c5d589b30bd26d2043b2b.json`.
The corrected fixture attaches diagnostics before target admission and rejects an early-closed relay during readiness.
Its maintained regression passed after the intended missing-module RED.
A later fresh owned window reached readiness without a global network change.

## Fresh Host witness after the calculation increment

The coordinator added the core calculation module after the previous signing source fence ended.
The next build captured 231 current paths and verified them before and after compilation.
The build reused the previously detached binary after SHA-256, inode, ownership, and recorded-manifest checks.
It created exactly one new detached binary copy with the 256 MiB maximum.
The binary reuse helper passed four regressions after its intended RED.
The first scripted helper edit produced invalid syntax; its file and error log were preserved before correction.
No compilation started from that failed helper.

The corrected current-source build completed in 9.49 seconds with exit status zero.
Its Host binary SHA-256 remained unchanged because the new generic calculation is unused in this fixture.
Build: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/host-corrected-build-c8d233ba11b6fc40d3dfb56a/build-identity.json`.
This remains source-authoring evidence, not extracted artifact admission.
The current preparation suite passed 104 provider tests inside `rom-dev` before focused-case and relay checks were added.

## Account and original-consumer follow-up

A fresh read-only inspection captured the actual provider user API source.
It confirms user creation and the separate password setter's 204 response.
Source: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/provider-users-source-060f03c693eb8b7f1c8dc699.txt`.
SHA-256: `08c2497227b5c3f4ef943e5dbce6ca0f287ca83a3188bb70e92c5ed0098783d6`.
Account controls share the bounded synthetic-provider API helper with key controls.
Roles have distinct endpoint and method allowlists. Account credentials remain in memory.
The new account projection contains synthetic identity fields and excludes passwords.

Read-only review of current Astral Plane `web/src/App.svelte` still finds an expiry-only catch branch.
It clears private views for anonymous refresh or `SessionExpiredError`, but has no explicit `SessionDeniedError` handling.
AP-UX-006/013/021 original-consumer acceptance therefore remains open.
Actual provider expiry and denial results must not be presented as application adoption evidence.
Temporary 503 acquisition failures must remain distinct from confirmed denial in that migration.

## Completed focused key cutover

The focused SQLite/Chromium repetition passed against the current 231-path Host witness.
It generated a real RSA key and selected it through the provider API.
The actual JWKS removed the original token key.
After 32 seconds, the old session returned 401 without protected data.
A fresh login received a token with the new published key and returned 200.
The case restored the original signing key before shutdown.

The API operations took 58 ms, 581 ms, 92 ms, and 631 ms.
Each operation had an eight-second deadline, with no automatic retry.
Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-12f4fb8078cf75f3c15d155c/result.json`.
Drain: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/focused-rotation-post-drain-3ce1e6ff68d4fba413dfe230b365eee2.json`.
Together with the earlier three passes, this covers both adapters and both browsers.
These are separate retained runs, not a rewritten four-case result.

## Actual denied and unlinked accounts

The current-source account matrix passed all four SQLite/redb and Chromium/WebKit cases.
Each case disabled the linked ROM User through the fixture's existing Resource mutation path.
A valid provider login then returned 401 and an anonymous ROM session.
Restoring the ROM User allowed a fresh linked login and protected read with status 200.
This tests current ROM User admission, not a disabled account in Authentik.

Each case also created a new active internal account through the actual provider API.
The password setter returned 204. Passwords remained in memory and were excluded from evidence.
The real provider issued a token with a distinct subject, recorded only as a SHA-256 digest.
The unlinked identity's callback and protected read returned 401 without protected data.
A subsequent fresh linked login returned 200.
No JWKS, token, or provider response was substituted.

Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-e35c064847644a32daa6ceee/result.json`.
Build identity: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/host-corrected-build-c8d233ba11b6fc40d3dfb56a/build-identity.json`.
The source fence, detached binary identity, and host trust remained unchanged.
The owned provider stopped all three containers. Its exact cgroup had no live descendants.
Ports 44389–44393 were vacant after shutdown.
Drain: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/accounts-post-drain-a8cbcf0ba481d391aa9d9cc9ed6f062d.json`.

## Consumer feedback and remaining admission

AP-UX-006, AP-UX-013, and AP-UX-021 concern confirmed authority loss and private-view lifecycle.
Current provider tests establish backend rejection after revocation, expiry, key removal, and unlinked identity admission.
The outage test separately establishes temporary 503 failure with same-session recovery.
Neither result establishes private-view clearing in the original Astral Plane application.
Its reviewed source catches expiry only; explicit `SessionDeniedError` adoption remains open.
The shared feedback and coordination records retain this acceptance requirement.

The provider suite is source-authoring evidence. Packaged artifact admission and original-consumer acceptance remain separate.
Patched private GnuTLS/tasn1 dependencies address the isolated TLS prerequisite; other recorded runtime advisory limits remain.
The coordinator must run the combined local release verifier after the final source changes.
