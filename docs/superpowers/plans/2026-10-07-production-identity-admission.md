# Production Identity Admission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish reproducible R7 evidence through isolated Authentik and ROM public identity APIs.

**Architecture:** An external consumer uses verified extracted ROM source and a frozen lock. A separately owned Authentik deployment supplies actual browser login, code exchange, tokens, keys, and durable provider state. A bounded supervisor owns every fixture process and evidence file.

**Tech Stack:** Authentik 2026.8.3 community profile, PostgreSQL 16, Compose v2, isolated TLS transport, Rust 1.99, ROM SQLite/redb, existing admitted Playwright runtimes.

**Spec:** [Production identity research](../../research/rom-0.1.0-production-identity-admission.md) and [R7](../../research/rom-0.1.0-consumer-research-plan.md:62).

## Global constraints

This plan is proposed. No task below was executed by the research author.
Obtain coordinator source ownership, process allocation, and endpoint reservations before implementation.
The existing authorization covers isolated work; it does not authorize changing existing deployments or using live credentials.
Do not commit, deploy, remove retained evidence, or share native build targets without coordinator allocation.

Use real provider behavior for success cases. Synthetic faults supplement actual provider evidence; they cannot replace it.
Preserve public API paths, module boundaries, and the generic Resource model.
Do not add provider dependencies to ROM's core. Do not clone producer development dependencies into the consumer.
Use extracted source with path dependencies resolved only within the admitted extraction.
An authoring checkout run must be labelled separately from candidate artifact admission.

## Proposed ownership and modules

| Proposed path | Responsibility |
| --- | --- |
| `tests/identity/provider/verify.mjs` | Source admission, finite orchestration, adapter/engine matrix, final result. |
| `tests/identity/provider/supervision.mjs` and separate unit tests | Process identity, deadlines, resource checks, signal/drain ownership, exclusive evidence paths. |
| `tests/identity/provider/evidence.mjs` and separate unit tests | Source/lock/image identity, report admission, secret redaction, no partial pass. |
| `tests/identity/provider/authentik/compose.yml` | Isolated server/worker/PostgreSQL network and limits; no production network references. |
| `tests/identity/provider/authentik/provision.py` | Fresh users, client, exact redirects, explicit grant policy, keys, and controlled rotation via provider administration. |
| `tests/identity/provider/browser.spec.mjs` | Actual redirects, consent/password interaction, local logout, CSRF, browser binding, and resource effects. |
| `tests/identity/provider/consumer/Cargo.toml`, `Cargo.lock`, `src/{main,identity,note}.rs` | Public ROM host, ordinary identity provisioning, generic owned Note, actual adapter choice; facades contain no implementation. |
| `tests/identity/provider/tls.mjs` | Node HTTPS reverse transport with generated fixture certificates; normal host forwarding, no token/JWKS response mutation. |
| `tests/identity/provider/package.json`, `package-lock.json` | Exact minimal browser harness graph and frozen lock; reuse existing admitted Playwright version deliberately. |
| `crates/rom-studio-host/src/oidc.rs`, `oidc_tests.rs` | Only independently reproduced JWKS compatibility and acquisition classification fixes, if approved. |
| `crates/rom-auth/src/keys.rs`, `jwt.rs`, `oidc/header.rs`, their separate tests, and public exports/docs | Only independently reproduced bounded key-identifier compatibility; one shared limit across all verification paths. |
| `crates/rom-studio-host/src/{authentication,session}.rs` and separate tests | Only required session lifecycle corrections, if narrower acquisition classification cannot suffice. |

These are proposed files, not ownership grants. The coordinator must resolve overlap before edits.
Any OpenSpec change, release manifest integration, public error addition, or provider-wide logout feature requires its own approved scope.

## Finite execution envelope

- Reserve all ports with the coordinator. No port number in this plan is an allocation.
- Use a unique Compose project and isolated network. Bind published HTTP/TLS ports to numeric loopback only.
- Run server, worker, PostgreSQL, one ROM host, one Node supervisor/TLS process, and one browser engine at a time.
- Set provider aggregate limits to four CPU cores and 4 GiB memory. Allow another 4 GiB for host, supervisor, and browser.
- Budget 12 GiB for admitted image/tool acquisition and 8 GiB for one run's volumes, logs, builds, and reports.
- Stop before exceeding either storage budget. Report measured growth every two seconds during acquisition/build.
- Limit acquisition/build to 20 minutes. Limit the complete behavioral run to 15 minutes, excluding acquisition.
- Limit readiness to 120 seconds, each ordinary browser case to 30 seconds, each rotation/restart/expiry case to 180 seconds.
- Limit host acquisition to two seconds and 64 KiB. Set 16 sessions, eight login attempts, and two authentication jobs.
- Configure short-lived provider tokens for timing cases; assert observed original `exp` before waiting. Use wall-clock expiry for the admission proof.
- Bound retained public logs to 32 MiB total. Disable secret-bearing HAR, traces, request dumps, and environment dumps.
- Stop every owned child on failure. Wait five seconds before forced termination. Record exit status for every child.
- Preserve stopped fixture volumes, source, failed runs, and evidence. Do not remove existing containers, images, volumes, or targets.

These are proposed caps, not measurements or upstream recommendations. Report allocation incompatibility before starting.
Use an independently isolated engine data directory or root-approved dedicated fixture engine. Never attach to the live application engine.
The current `rom-dev` has no admitted Docker/Podman prerequisite; the coordinator must select the local execution environment first.

## Task 1: Admit the fixture and reproduce the first success failure

- [ ] Assign the proposed files and reserve the isolated engine, browser ports, and native build allocation.
- [ ] Write failing unit cases for existing output paths, source mismatch, lock drift, unknown image/platform identity, missing engine result, timeout, and secret-bearing reports.
- [ ] Run those cases and preserve their actual failure output.
- [ ] Implement exclusive evidence creation and strict admission using existing archive/source helpers.
- [ ] Record artifact hash, verified source identity, fixture manifest, frozen locks, toolchain, provider platform manifests, image IDs, and licenses/advisories.
- [ ] Reconcile the locally declared Authentik/PostgreSQL digests with the actual downloaded image identities. Do not assume tag or version equivalence.
- [ ] Provision only fresh accounts: allowed, provider-denied, and authenticated-but-unlinked. Keep administrator material private and fixture-local.
- [ ] Configure exact issuer with final slash, explicit endpoints, stable `user_id`, strict callback, code flow, S256 PKCE, and RS256.
- [ ] Configure an explicit application access binding. Provider defaults must not silently grant every user.
- [ ] Start the external consumer on SQLite and use real Chromium browser login.
- [ ] Preserve unchanged public discovery/JWKS and require successful callback, sealed identity binding, authorized Note creation, receipt, and one event bundle.

Expected initial behavioral RED: Authentik's actual JWKS includes certificate metadata rejected by ROM's closed JWK decoder.
Fresh Authentik keys also have 86-character identifiers, which exceed ROM's current 64-byte limit.
This expectation is source-derived. Record an observed RED only after the actual command fails for that reason.
If another prerequisite fails first, retain it and repair only the fixture before claiming the compatibility RED.

## Task 2: Correct only the reproduced identity boundary

- [ ] Reproduce the exact actual JWKS shape in a focused host decoder test without private material.
- [ ] Test oversized certificate metadata, duplicate keys, wrong algorithm/use, non-RSA keys, extra trust-source URLs, and malformed core parameters.
- [ ] Define the narrow supported certificate metadata policy before editing the parser.
- [ ] Reproduce the actual fresh Authentik key identifier through public OIDC/JWT verification and host JWKS decoding.
- [ ] Review a shared finite identifier bound; test its exact boundary and overflow in every affected verification path.
- [ ] Preserve old short-identifier callers and publish any added constant through a stable facade.
- [ ] Accept only inert bounded metadata required for this provider; continue verifying with approved `n` and `e`.
- [ ] Never fetch certificate URLs, trust supplied certificate roots, or widen JOSE header admission.
- [ ] Run the focused RED→GREEN host tests under coordinator native allocation.
- [ ] Repeat unchanged actual-provider login through the public consumer.
- [ ] Stop the provider after login, wait beyond the existing proof interval, and request a protected operation before original token expiry.
- [ ] Require bounded temporary failure, no protected disclosure/commit, retained session context, and successful verification after provider recovery.
- [ ] Preserve the expected initial session-loss RED. Separate transport unavailability from invalid token, expired proof, and explicit provider denial.
- [ ] Implement the smallest approved classification correction and repeat recovery, expiry, current revocation, and observation tests.

Do not increase proof lifetimes to pass recovery. Do not turn expired or invalid credentials into a cached authorization success.
If public error changes are necessary, stop that change at a reviewable design and coordinate its separate scope.

## Task 3: Execute the real provider lifecycle and negative matrix

Run the same consumer source on both actual native adapters. Run browser cases in Chromium and WebKit, sequentially.

| Case | Required oracle |
| --- | --- |
| Provider restart with retained PostgreSQL volume | Record old/new process IDs, unchanged public signing key where no rotation occurred, preserved provider account/grant, and original-token verification after recovery. No reprovisioning conceals lost state. |
| Host restart | Original cookie becomes anonymous; fresh real callback creates a new generation. Resource, receipt, and ownership survive native reopen. |
| Key rotation | Use the provider's actual administrative key selection. Observe distinct public `kid`, real new token, and unchanged JWKS. Verify new-key admission and measured old-key behavior after the existing proof interval. |
| Expiry | Wait until observed original `exp`; protected requests and old cookie deny. Fresh login succeeds. No host clock rollback or token rewrite substitutes for the wall-clock case. |
| Local logout | Valid Origin/CSRF returns logout success, clears cookie, cancels an active observation, and rejects the saved cookie. Unrelated session remains valid. |
| Provider logout | Execute actual OP end-session. Report fresh-login behavior and existing ROM-cookie behavior separately; no immediate global logout claim. |
| Provider denied access and consent | Actual denied account/consent yields no session or Resource mutation. Unlinked valid identity remains denied independently of provider authentication. |
| Current ROM revocation | Disable provider, link, and User separately through authorized Resource actions. Existing sessions/receipts cannot restore revoked authority. Observe denial before further private disclosure. |
| Callback attacks | Missing/wrong/duplicate state, wrong browser cookie, expired attempt, reused code/state, wrong callback provider, and mismatched response issuer produce no session. |
| PKCE | Actual token endpoint rejects code exchange with incorrect verifier. The control exchange with the original verifier succeeds. |
| Token trust | Actual independently issued wrong-client and wrong-provider tokens fail public verifier audience/issuer checks. Wrong retained nonce and altered signature fail. Correct unchanged token succeeds as the control. |
| CSRF | Missing/wrong/duplicate Origin or CSRF, duplicate session cookie, and cross-origin logout fail before Note state/event changes. The same session remains usable after rejected CSRF. |
| Secret and endpoint admission | Bad file permissions/reference, unapproved endpoint, redirects, wrong TLS host, and untrusted certificate fail before authenticated Resource work. No dynamic browser-selected discovery source is accepted. |

Authentik key cutover does not promise retained old keys. If R7 requires uninterrupted old-token verification, review an actual overlap procedure separately.
Provider-wide logout is unsupported by the current ROM host. Record it as a separate open contract rather than relabel local logout.

## Task 4: Admit isolated TLS and package evidence

- [ ] Generate a fresh fixture CA and certificates into private, exclusive paths.
- [ ] Run browser-to-host and host-to-provider HTTPS with normal verification and exact origins.
- [ ] Admit the fixture CA only into isolated browser/host trust configuration. Verify the actual Rustls trust mechanism before using it.
- [ ] Fail the run if trust setup would require disabling certificate checks or modifying the machine-wide trust store.
- [ ] Record public certificate fingerprints and negotiated TLS versions, without private keys.
- [ ] Run wrong-host/untrusted-certificate controls and require no successful code exchange or Resource mutation.
- [ ] Produce `source-identity.json`, `provider-identity.json`, `resource-envelope.json`, and one result per adapter/engine/scenario.
- [ ] Produce final `result.json` only after all required cases, process drains, secret checks, and post-run source fences pass.
- [ ] Preserve expected failures as independent evidence with command, exit status, source, and timing identities.
- [ ] Request independent review of actual reports and unsupported logout/rotation boundaries.
- [ ] Coordinate affected checks, downstream compile fixtures, full local verification, and release artifact admission against the frozen source.

Proposed executor commands after implementation are:

```sh
node --test tests/identity/provider/*.test.mjs
cargo test -p rom-studio-host --locked
cargo test -p rom-auth --features oidc --locked
node tests/identity/provider/verify.mjs --source-dir <verified-extracted-source> --output <exclusive-evidence> --admit
```

These commands were not run for this design. Build allocation remains required.
The executor must implement and document the final CLI before treating this syntax as runnable.

R7 admission requires actual Authentik results, both adapters, both engines, TLS evidence, bounded failures, and independent review.
The final report must state local versus provider-wide logout and measured rotation behavior explicitly.
This gate does not establish arbitrary provider compatibility, human usability, a live deployment, or completion of R1–R14.

## Task 1 scaffolding record, 2026-10-07

The coordinator granted exclusive ownership of `tests/identity/provider/**` for pure Node helpers and unit tests.
No provider, image, engine, daemon, native build, or browser execution was admitted for this step.
The new admission, evidence, and supervision modules have separate test modules and a limitations README.

Actual `node --test tests/identity/provider/*.test.mjs` results:

- The initial module-contract RED failed because the three implementation modules did not exist.
- The additional guard RED had 11 passes and four failures: unlisted source file, private evidence, intentional shutdown, and aggregate output cap.
- The concurrency RED had 15 passes and one failure: a completed child's slot did not become available.
- The first focused GREEN had 16 passes, zero failures, and zero skipped tests.
- A later schema RED had 16 passes and one failure for malformed process records and coercible engine names.
- The final schema GREEN had 17 passes, zero failures, and zero skipped tests.

Retained logs are `/var/tmp/rom-010-identity-task1-contract-red-20261007.log`, `/var/tmp/rom-010-identity-task1-guards-red-20261007.log`, `/var/tmp/rom-010-identity-task1-concurrency-red-20261007.log`, and `/var/tmp/rom-010-identity-task1-final-green-20261007.log`.
The final log SHA-256 is `501b39c371c850dff56f1b87d2e95152d56ccfd69d1a07e7bb38b4f063fd7eb1`.
That log identifies the earlier 16-test snapshot.
The later schema logs are `/var/tmp/rom-010-identity-task1-schema-red-20261007.log` and `/var/tmp/rom-010-identity-task1-schema-green-20261007.log`.
The 17-test GREEN log SHA-256 is `994564c8febe41aa5de9a31f7acf07de09dbc2c0c0ed12d01bfc7d8317acd957`.
The source directory uses approximately 40 KiB. Its tests use retained exclusive scratch directories.

Ruling: permit an owned intentional graceful SIGTERM stop in final process evidence; reject unexpected signals, forced termination, deadline, and failed drain.
This distinction is necessary for actual restart tests. It does not prove a provider restart occurred.
Ruling: enforce six concurrent children and retain at most 32 process records, including completed children; reject PID reuse.
This preserves lifecycle evidence without preventing sequential host/browser replacement.

Read-only executable inspection found host Podman 5.2.3. The host `docker` command reports that same version.
The proposed isolated engine uses explicit fresh root/runroot paths, vfs storage, and remote connections disabled.
No default-store information query was executed. The coordinator still owns engine, endpoint, storage, and subsequent execution allocation.
Task 1 remains incomplete: real image/source admission, provisioning, public-consumer compilation, and actual compatibility RED are pending.

### Task 1 environment checkpoint on 2026-10-07

The coordinator admitted a source authoring environment with a separate 12 GiB acquisition cap and 8 GiB run-evidence cap.
Actual process ownership, isolated engine information, and mount/network/PID namespace prerequisites passed.
The focused Node suite now has 30 passes, zero failures, and zero skipped tests.
Additional RED cases covered execution before admission, unowned process groups, CDN credential forwarding, forbidden redirects, and bounded store paths.

Actual metadata resolves both requested image digests to Linux/amd64 manifests. Authentik's source revision label is empty.
Keep independent source-tag evidence separate from image source attestation. Neither metadata admission nor upstream advisories replace a package vulnerability assessment.

A fixed 12 GiB filesystem enforced acquisition capacity. The actual vfs pull stopped with approximately 1 GiB remaining and drained cleanly.
No provider container or PostgreSQL acquisition started. Preserve this resource RED and the partial store.
Coordinate a separate storage alternative before retrying. Do not enlarge a limit, reuse evidence paths, or remove partial layers implicitly.

Task 1 remains open for completed image acquisition, provider process ownership, provisioning, and observed public-consumer compatibility RED.
Native compilation, actual two-engine journeys, and the extracted release-artifact lane retain their separate admission requirements.
See the [research checkpoint](../../research/rom-0.1.0-production-identity-admission.md#isolated-prerequisite-results-on-2026-10-07) for exact identities and evidence paths.

A separately allocated overlay store subsequently passed its actual namespace prerequisite, fresh engine check, and both digest-pinned image pulls.
The coordinator expanded only the total retained envelope to 32 GiB; each acquisition filesystem remains capped at 12 GiB.
The focused suite passed 31 tests. No provider container or browser journey started at this checkpoint.
Review the exact stored-image, filesystem, upstream source, license, and advisory records before container launch.
The earlier vfs resource RED remains valid evidence of that configuration's storage cost.

### Observed SDK compatibility work

An actual Chromium code-flow callback produced an untouched Authentik ID token and original JWKS.
The public SDK rejected its 86-byte key ID with `UnknownKey`. Preserve that actual RED and private original token.
A shared 256-byte opaque UTF-8 key-ID contract now applies to JWT, OIDC, and complete issuer-selected key sets.
Reject empty IDs, Unicode control characters, and IDs above 256 bytes. Preserve exact case and Unicode representation.
These limits belong to the ROM profile; JOSE does not prescribe their length.

Signed boundary and cache tests passed after their observed RED. Related auth suites and Clippy also passed.
The immutable original token passed explicitly historical cryptographic replay and correctly failed current-time expiry.
A fresh original callback passed current-time SDK verification with the frozen binary.
Keep historical replay, expired rejection, and fresh acceptance as separate results.

Task 1 remains open for complete provider-source admission and real public StudioHost execution.
The next authoring harness uses real SQLite/redb adapters serially, ordinary identity Resources, and public StudioHost APIs.
First invoke the unchanged parser on retained original JWKS bytes to attribute the predicted metadata gap.
Then exercise the full public Host login and callback before claiming Host compatibility.
Do not alter the parser before an observed parser RED. Do not infer TLS, current authority, or packaged admission from HTTP SDK runs.
The coordinator owns the next native build allocation. Provider and consumer processes retain separate finite cgroup and filesystem admission.
See the [SDK checkpoint](../../research/rom-0.1.0-production-identity-admission.md#original-provider-sdk-compatibility-checkpoint) for counts, limits, and evidence.

### Actual Host correction sequence

The unchanged parser rejected the original public JWKS. A subsequent full callback returned HTTP 401 on SQLite and redb.
Retain both attribution and public-callback REDs. Retain preceding asset-path, Chromium SIGTRAP, and generic-loader fixture failures separately.
Use the short owned temporary path and fail-closed socket budget before subsequent browser launches.

1. Add shared public opaque key-ID validation without changing its existing 256-byte semantics.
2. Move JWKS parsing into a named module. Preserve the existing internal `oidc::parse_jwks` delegation seam.
3. Add tests before changing parsing. Exercise original bytes, metadata boundaries, duplicates, absent RSA components, and algorithm/use/key-operation rejection.
4. Permit bounded unused registered certificate metadata in the RSA `n`/`e` profile. Do not fetch `x5u` or derive keys from certificates.
5. Reject malformed types, null, invalid encodings, excessive chains/text, duplicate fields, and unsupported members.
6. Run original-JWKS tests, all Host tests, all-feature auth tests, and affected Clippy checks with warnings denied.
7. Rebuild the public harness from a new pre/post source fence. Preserve the previous compiled binary and all earlier records.
8. Run fresh actual provider callbacks on both adapters. Do not inherit the earlier SDK or callback acceptance.
9. Continue trusted TLS, two engines, rotation, restart, expiry, denied grants, outage, CSRF, logout and current-authority proof.
10. Run the independently extracted source/package lane and full local verifier before integration.

Certificate syntax checks do not establish certificate, chain, key-equality or thumbprint-equality validation.
The n/e-only trust profile and local finite metadata bounds must be explicit in public documentation before release.
AP-UX-006 remains a consumer session requirement. AP-UX-015 denotes research deduplication, not production identity.
Keep source, generic contract, regression evidence and original-consumer acceptance as separate traceability columns.

### Source-only lifecycle harness extension

The next fixture modules are `host/src/control.rs`, `control_tests.rs`, and `protected.rs`, with declarations in the existing main facade.
The existing provisioning module registers and seeds one ordinary protected Resource. Its content is synthetic fixture data.
An explicit configured Embedded actor controls User, IdentityLink and IdentityProvider activation through public Runtime Commands.
The controller never accepts an actor, authority, subject, Resource name, or arbitrary field from an input request.

Use a new private control directory for each Host process. Admit at most 16 sequential requests, each at most 4096 bytes.
Accept only closed sequence/action requests. Permit six actions: enable or disable the selected User, Link or Provider.
Reject duplicate fields, extra actor metadata, unknown actions, stale/out-of-order sequence numbers and symbolic-link inputs.
Apply revision-checked replacements through the same Runtime. Write an exclusive acknowledgement only after a confirmed commit.
An acknowledgement failure is a fixture failure with an unknown control outcome; do not invent an automatic receipt replay guarantee.

First prepare Rust tests for both real adapters and malformed request admission. Run the intended RED only after native allocation.
Then implement the closed controller and read-only Human Resource policy. Run focused native tests before browser integration.
The browser must prove protected reads before revocation and denial after actual User, Link and Provider Commands.
Reenabling an identity does not establish revival of its revoked session. Require a fresh original provider callback before protected reads resume.
Exercise anonymous reads, wrong Origin, absent/wrong CSRF and local logout separately.
Record protocol session rechecks separately from Studio application view retention and original Astral Plane acceptance.

The owner renewed the feedback marker to `1791396483`, with the latest reviewed turn unchanged and 32 canonical AP-UX groups.
AP-UX-006 and AP-UX-021/026 remain applicable. Synthetic fixture evidence does not close those original-consumer workflows.

### Observed private TLS prerequisite and module follow-up

The NSS tool acquisition and private CA import/readback completed within their admitted limits.
The first actual browser trust run passed Chromium's trusted, untrusted, wrong-SAN, and expired-certificate cases.
WebKit 2359 reported unavailable TLS support. This is a prerequisite failure, not a ROM compatibility failure.
The classifier now rejects unavailable TLS backends instead of admitting them as negative certificate cases.
Retain the original failed evidence and its separate subsequent reclassification.

The pinned runtime uses GLib 2.82.1. Its nixpkgs source selects glib-networking 2.80.1.
Official cache metadata identifies a 35-path closure: 21,640,360 compressed bytes and 98,544,128 NAR bytes.
The three absent outputs are glib-networking, libproxy 0.5.9, and Duktape 2.7.0.
The GnuTLS 3.8.6 library output is already present. Its Nix patch supports environment-scoped `NIX_SSL_CERT_FILE`.
The older GnuTLS source lacks fixes recommended by current official certificate-validation advisories.
Do not treat a successful historical-runtime probe as production dependency admission.

Before module acquisition, obtain coordinator review of the exact signed closure and finite private-store envelope.
Use a fresh Nix chroot store inside the hard acquisition filesystem. Never use the default daemon store.
Copy signed official artifacts only. Do not build, disable signature checks, or change global environment or trust.
Bound retained tool storage to 256 MiB. Bound acquisition to ten minutes and the existing process cgroup limits.
Verify closure identities and hashes after acquisition.
Project only the unchanged regular TLS module into a private GIO module directory.
Record actual loaded dependencies before repeating browser trust cases.
Use private `NIX_SSL_CERT_FILE` only after source and behavior review.
Record any supplemental runtime module as an authoring deviation until a complete runtime profile is admitted.

### Public Commands lifecycle fixture progress

The intended missing control/protected-module compilation failure is retained.
Three executed tests then passed on the actual SQLite and redb adapters.
They cover six current-revision identity Resource replacements and denial for unconfigured principal kinds.
The protected Resource remains inaccessible to unstamped synthetic Human actors.
The fixture tests do not establish real-browser revocation or session renewal.

The next prepared tests require bounded private mailbox admission and acknowledgement after actual commit.
Negative inputs include symlinks, oversized input, out-of-order files, and sequence mismatch.
Those tests are prepared but unexecuted at this checkpoint.
After their observed RED and GREEN, wire the private controls into the bounded Host lifecycle.
Then exercise real provider sessions against the ordinary protected Resource.
Keep revocation, re-enable, fresh login, logout, CSRF, expiry, outage, restart, and rotation as separate claims.

## Executed current-authority checkpoint

The source-authoring HTTPS matrix passed SQLite and redb with Chromium and WebKit 2359.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/https-host-authoring-45fe16fe6cae02a424412229/result.json`.
Each case tested real callbacks, protected reads, Origin/CSRF rejection, User/link/provider revocation, fresh recovery, and local Host logout.
Current-authority controls used public revision-checked `Command` mutations, not direct database edits.
Selected native and fixture source fingerprints and the executed binary remained unchanged during the run.
All case process groups drained. A separate post-exit snapshot confirmed provider cgroup drain after its terminal snapshot.
All earlier failed records, images, volumes, and launcher sources remain retained.

The maintained Node suite has 76 passing tests, including portable storage, actual Fetch Response behavior, and private stop-file admission.
The auth helper export now has the same JWT feature condition as its module.
Independent no-feature, JWT, OIDC, and introspection configurations passed; their exact test scopes remain separate.
A fresh Host build captured this change before the actual HTTPS run.

The next production identity steps remain open:

- Replace the historical GnuTLS runtime with a reviewed patched compatible runtime before production TLS admission.
- Execute provider key rotation, provider restart, expiry, outage, denied grants, and provider-wide logout with original responses.
- Reproduce the matrix from extracted and verified candidate source with admitted dependencies and frozen consumer locks.
- Verify application administrator-only policy separately from synthetic provider administrator authentication.
- Obtain original-consumer acceptance for AP-UX-006 session boundaries and AP-UX-021 ownership/privacy expectations.

Source-authoring success does not close release gates, other R1–R14 requirements, or original-consumer acceptance.
