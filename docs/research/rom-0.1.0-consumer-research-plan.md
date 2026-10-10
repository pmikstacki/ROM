# ROM 0.1.0: research plan from production consumer feedback

Date: 2026-10-07. Status: acceptance plan; implementation and release admission remain in progress.

## Purpose and scope

Make ROM usable for the owner's first production application within an explicit, tested deployment profile.
Use Astral Plane as the first external consumer. Add a second, unrelated consumer to test generic behavior.
Keep one Resource model for application data, settings, identity configuration, and durable workflow state.
Transport, database, identity, and telemetry implementations remain behind their existing contracts.

This plan does not declare production readiness or authorize changes to the deployed consumer.
It does not select new dependencies, promise multiwriter support, or replace the existing release gates.

## Evidence basis

The referenced [consumer conversation](thread://01a111a5-617a-7800-b94f-eb82b11da39d?hostId=remote-ssh-codex-managed%3A7455bafd-bcdf-4924-b743-87f13bd5b771) was read on 2026-10-07.
The [consumer handoff](astral-plane-production-feedback-2026-10-07.md) records framework friction, application defects, and reported tests separately.
The inspected producer HEAD is `d7ef529040eec60dc869034c2d33130219db85fe`.
The inspected consumer HEAD is `86dca6f`; its working tree was clean during this inspection.
The [0.0.3 completion record](rom-0.0.3-release-completion.md) remains the accepted framework baseline.
The [independent source review](rom-0.1.0-consumer-source-review.md) records callback, export, styling, and notice-path findings.

The conversation reports production Authentik login, Resource persistence, conversation reload, public projections, and research publication.
Those reports are consumer evidence, not independently reproduced 0.1.0 acceptance.
This planning task inspected selected source files. It did not repeat their tests or contact production.

### Findings confirmed by source inspection

- Producer `Definition::validate_transition` stores a function pointer. The consumer uses a captured `Arc<dyn Fn(...) + Send + Sync>` callback.
- Producer `studio/src/index.ts` exports `Input`, but not the broader control catalog requested by the consumer.
- The handoff's Input-and-Button wording describes a consumer baseline, not the inspected producer entry.
- The consumer also adds authentication helpers. Their public support contract needs a separate decision from control exports.
- Producer notice ownership uses lexical `node_modules` paths. The consumer patch addresses a shared-install path mismatch.
- The consumer enrichment workflow persists prepared publications and implements its own worker coordination over Resources.
- ROM already has operator recovery and database backup capabilities. Their composition and discoverability need consumer evaluation.

Relevant consumer files are under `/root/astral-plane`:
`docs/rom-catalog-validator-patch.md`, `vendor/rom/crates/rom/src/resource/definition.rs`,
`vendor/rom/studio/src/index.ts`, `vendor/rom/studio/build/notices/ownership.mjs`, and `src/enrichment/`.
Record full revisions, patches, lock hashes, and file hashes before each experiment.
Do not silently copy the consumer vendor tree into ROM.

### Application defects that do not establish ROM defects

Provider deadline splitting, invalid AI request schemas, model cold loading, and unsuitable generated answers belong to application execution.
Clipped scrolling and hidden answers belong to application presentation unless a shared control regression reproduces them.
Use these cases to test deadlines, diagnostics, and control composition. Do not add Human Design or AI model policy to the core.

## Research work packages

Every experiment below is planned, not executed. P0 blocks the production release; P1 supports operational acceptance.

| ID | Priority and question | Compare or investigate | Experiment and acceptance criterion |
| --- | --- | --- | --- |
| R1 | P0: Can authors validate with runtime-owned state? | Existing function pointers, captured synchronous callbacks, and immutable dependency snapshots. | Two independent runtimes use conflicting catalogs. No process-global registry or cross-runtime state leakage. Existing callers compile. Rejected transitions emit no success event. |
| R2 | P0: Can an application use Studio through its public entry? | Supported control exports, documented subpaths, and wrapper/control styling contracts. | Build an extracted external consumer with select, checkbox, textarea, slider, labels, dates, and a compact composer. No imports from private source paths. Assess authentication helpers separately. Chromium/WebKit verify focus, sizing, and value semantics. |
| R3 | P0: Does packaging handle supported installation layouts? | Ordinary installs, whole-directory dependency symlinks, and independently extracted packages. | Reproduce the notice failure before correction. Valid layouts retain complete licenses and portable module IDs. Unknown owners, missing licenses, and invalid package identities still fail. |
| R4 | P0: Can ROM own reusable mutation recovery? | Existing public client/controller composition and a small shared mutation workflow. | Save, lose the acknowledgement, retry, reload, and switch views. Preserve operation identity and expected revision. Test changed input, stale revision, cancellation, logout, and permission revocation. One commit creates one event bundle. |
| R5 | P0: Can authors compose durable work without rebuilding a scheduler? | Existing reactions, channels, operator recovery, and the consumer coordinator. | Implement prepare, publish, and follow-up through documented ROM APIs. Inject restart at each boundary. Retain prepared input, original identity, budgets, and consumed attempts. Confirm no duplicate publication when the receiver supports deduplication. |
| R6 | P0: Are public projections safe and reusable? | Existing authorization/projection contracts and a narrow shared projection helper where necessary. | Anonymous access exposes only declared public fields. Raw jobs remain denied. Private edits and permission changes invalidate authorized results. Conditional caching cannot cross principals or reuse stale disclosure. |
| R7 | P0: Is production identity reproducible? | The existing OIDC host profile with the consumer's Authentik setup; current session and secret contracts. | Test provider restart, signing-key rotation, expiry, logout, denied grants, CSRF, and issuer/audience mismatch. Repeat against the real provider in an isolated deployment. No fixture signing keys or grants enter production. |
| R8 | P0: Can a complete application recover on a clean host? | Existing logical database archives and a coordinated application backup manifest. | Restore database, blobs, configuration references, and unfinished work on a fresh host. Verify receipts, tombstones, journal fences, and attachments. Measure RPO/RTO. Never auto-restore over a live database. |
| R9 | P1: Can operators explain a slow or stuck operation? | Existing Runtime status and Work views, plus new maintained tracing; optional OpenTelemetry export. | Correlate action, receipt, event, reaction, and external attempt. Distinguish waiting, execution, retry, and unknown outcome. Verify redaction, bounded labels, and exporter-failure isolation. |
| R10 | P0: What workload can the release support? | Measured single-instance deployment profiles on the actual supported adapters. | Run bounded mixed reads, writes, streams, uploads, and work. Inject disk-full, slow consumers, unavailable providers, and process interruption. Measure latency, memory, disk growth, queue age, and recovery. |
| R11 | P0: Can users upgrade safely? | Current format/migration contracts, matching Rust/Studio package sets, and application rollout procedures. | Upgrade a populated 0.0.3 fixture, with pending work and custom codecs. Preserve historical receipts and data. Verify application rollback separately from data restore; reject incompatible downgrades explicitly. |
| R12 | P1: Does the framework remain pleasant and generic? | Consumer implementation before/after the selected changes. | The owner implements a small feature using public APIs and skills. A second non-AI application repeats the same patterns. Record internal imports, vendor patches, duplicate orchestration, diagnostic clarity, and completion effort. |

| R13 | P0: Can one routing policy manage OpenRouter invocation safely? | Existing consumer routing, provider-native routing, and a small provider-neutral policy layer. | Discovery and every attempt share an end-to-end deadline. Eligibility includes tools/schema/context requirements. Verify run-scoped fallback continuation, bounded attempts, rate limits, reservations and actual cost accounting with deterministic clocks and simulated providers. |
| R14 | P0: Can authors compose durable agent flows ergonomically? | Existing reactions/channels/work plus typed tools and an optional flow facade. | Two unrelated consumers execute validated tools and prepare/publish steps. Restart after each boundary; preserve stable identities and authority. Separate ephemeral tokens from committed results. Denied tools, invalid output, budget exhaustion, and unknown external outcomes remain visible. |

### Important experiment boundaries

R1 must evaluate two Runtime instances, not only two Definition objects.
Captured validators remain synchronous, bounded, and non-reentrant. External I/O does not run inside transition validation.
Benchmark validator overhead separately from database commit cost.

R3 first tests the consumer’s whole-directory symlink. Individual package links and other package managers require separate support decisions.
Vite normally resolves real file paths; notice admission must still verify actual package identity and license ownership.
See [Vite symlink resolution](https://vite.dev/config/shared-options#resolve-preservesymlinks).

R4 does not automatically retry with a new key. Revoked authority may prevent disclosure of an earlier result.
That refusal must not be presented as proof that the earlier mutation did not commit.
Compare generic recovery paths on SQLite and redb, not only mocked HTTP responses.

R5 retains the agreed chain semantics: earlier commits remain; a failed step retries within its policy.
Compensation is an explicit domain action. A timeout does not automatically trigger compensation.
Record limits on chain depth, age, fan-out, attempts, and external side effects.
A browser progress view needs an authorized application projection; an operator Work view may require broader authority.

R6 keeps public views separate from raw host-only Resources. A public cache is not an authorization mechanism.
Backend authority remains decisive for every disclosure, including live updates and cached responses.

R8 extends existing backup capability rather than claiming backup is absent.
Database consistency alone does not establish a matching blob snapshot or a working identity deployment.
Process interruption tests do not certify power-loss durability.

## Primary-source research queue

Use upstream documentation and the installed versions' source. Latest documentation alone does not prove locked-version behavior.

| Topic | Primary reference | Planned use |
| --- | --- | --- |
| Captured callback ownership | [Rust Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html), [Rust Fn](https://doc.rust-lang.org/std/ops/trait.Fn.html) | Review sharing, callback bounds, captured state, and backward-compatible authoring. |
| Library composition | [Vite library mode](https://vite.dev/guide/build.html#library-mode), [Svelte package](https://svelte.dev/docs/kit/packaging) | Test public exports and installation boundaries without forcing a package migration. |
| HTTP conditional results | [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html#name-if-none-match) | Define HTTP cache validators in the adapter. Keep generic projection and authority rules in ROM. |
| OAuth deployment security | [RFC 9700](https://www.rfc-editor.org/rfc/rfc9700.html), [OIDC Core](https://openid.net/specs/openid-connect-core-1_0.html) | Review production identity threat cases against the host's actual flow. |
| Telemetry composition | [OpenTelemetry overview](https://opentelemetry.io/docs/specs/otel/overview/) | Assess API/SDK separation, trace context, optional exporters, and measured overhead. |
| Existing ROM recovery | [Operator recovery](../operator-recovery.md), [backup results](maintained-backup-results.md), [support limits](../release-support.md) | Reuse established contracts and retained evidence before proposing new APIs. |

HTTP cache validators are transport semantics described by RFC 9110. They do not specify Resource-level authorization.
OpenTelemetry separates instrumentation APIs from SDK implementations. Its specification supports investigating an optional exporter boundary, not adopting one without measurement.
RFC 9700 provides OAuth security guidance. Conformance for ROM's selected provider and host still requires review and executed tests.

The [active coordination record](rom-0.1.0-coordination.md) extends this plan with the owner-authorized UI and AI release scope.

## Execution order and ownership

1. Freeze the consumer feedback and the inspected producer/consumer source identities.
2. Run R1, R2/R3, and R4 as separate experiments with separate file ownership and evidence directories.
3. Run R5/R6 against the selected generic APIs, followed by the second consumer.
4. Run R7/R8/R11 on isolated deployment data and actual external-provider fixtures.
5. Run R9/R10 after instrumentation and deployment contracts are available.
6. Complete R12 with the owner, then map each release requirement to executed evidence.

Concurrent work must use separate output directories, database files, browser ports, and bounded build allocations.
Do not share mutable fixtures or change the production consumer during research without a separate deployment task.
Preserve failures, prototypes, worktrees, and source snapshots.

## Decision rules

Prefer existing public contracts when the consumer experiment proves they are sufficient.
Add a shared helper when repeated orchestration leaks framework responsibilities into application code.
Reject a helper that changes idempotency, current authority, or commit semantics for convenience.

Before selecting a strategy, record correctness, authoring effort, implementation size, dependencies, and performance under the same workload.
Use distributions and resource measurements rather than one best latency sample.
Document accepted and rejected alternatives, including what the experiment did not establish.

## Owner decisions and proposed defaults

| Decision | Proposed default; not yet an accepted release target |
| --- | --- |
| First deployment profile | Single instance with one active owner per database. Keep SQLite/redb conformance; do not make core depend on one driver. |
| First application workload | Astral Plane plus an unrelated small application. Agree record volume, concurrent sessions, stream count, upload limits, and work rate before load tests. |
| Data recovery targets | Agree RPO and RTO before acceptance. The restore drill measures whether the selected backup policy meets them. |
| Service targets | Agree availability and latency targets for ordinary Resource operations separately from external-provider work. |
| Distribution | Keep source-only GitHub publication and local release scripts unless the owner changes that policy. |

## 0.1.0 release gate

The release uses one matching Rust/Studio source set and independently verified clean-source artifacts.
The external consumer works without private source aliases or unreviewed vendor patches.
Every supported adapter retains the same Resource mutation and replay semantics.

An isolated application deployment passes real-provider auth, bounded load, failure recovery, backup restoration, and upgrade acceptance.
The owner can complete the documented author workflow. Accepted limits and remaining failures stay visible.
Public documentation states the measured deployment profile, compatibility policy, and operations procedures.
Passing a screenshot test, application deployment, or vendor-only verifier is insufficient by itself.

## Checks for this planning increment

This increment changes documentation only. It introduces no implementation or dependency change.
Verify local links, source identities, and the separation between reported, inspected, planned, and executed evidence.
Run OpenSpec validation only if OpenSpec artifacts change. Existing task states remain unchanged.

## Current evidence navigation

The preceding investigation records the initial planning boundary, not current completion.
Current implementation limits are in [coordination](rom-0.1.0-coordination.md), the [UI composition plan](../superpowers/plans/2026-10-07-ui-composition-admission.md), and the [AI flow plan](../superpowers/plans/2026-10-07-ai-routing-and-flows.md).
The [feedback gap review](rom-0.1.0-ui-feedback-gap-review.md) maps all current intake groups and remaining consumer acceptance.
These records do not establish a completed 0.1.0 release.

## Owner-directed diagnosis requirement: 2026-10-09

The owner requires web research before accepting a cause for the failed mixed-load trial.
This requirement supplements R7 and R10. The complete 0.1.0 release objective remains active.

Use official specifications, upstream documentation, and the installed versions' source for each hypothesis.
Separate a source defect, a reproduced defect, and the cause of this recorded trial.
A matching failure pattern alone does not establish causality.

The trial recorded 5,045 successful requests, 6,201 denied requests, 1,240 overloaded requests, and one timeout.
The final ledger contained 11,025 Done records, below the unchanged target of 12,000.
The proxy recorded one upstream EPIPE. Its owned processes and connections subsequently closed.
Preserve this failed evidence. Do not treat closure as workload acceptance.

Investigate these hypotheses separately:

1. Cached identity evidence expires while a caller waits for the credential mutex.
2. Authentication admission or current-identity binding saturates under combined request and stream traffic.
3. Provider key acquisition fails or becomes temporarily unavailable during proof renewal.
4. Token, session, provider configuration, link, or User state causes a valid rejection.
5. A client timeout, upstream failure, or stream cancellation explains the recorded EPIPE independently.

For each hypothesis, record a falsifiable prediction and a bounded reproduction command.
Reproduce the defect before accepting its correction. Retain genuine revocation and expiry rejection controls.
Do not increase credential validity, ignore denial, or lower the workload oracle to obtain a passing result.
After the correction, repeat the unchanged mixed profile on each supported adapter.

Primary references include [OIDC token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation),
[Tokio 1.53.1 Mutex](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Mutex.html),
and [Node.js system errors](https://nodejs.org/api/errors.html#epipe-broken-pipe).
The focused research report must connect each reference to inspected code and actual retained evidence.

### Renewed diagnosis and release work: 2026-10-09

The owner renewed the requirement for web research and an updated goal plan.
The active goal remains publication of complete ROM 0.1.0. Diagnosis is a release prerequisite, not a replacement deliverable.

The [follow-up diagnosis](rom-0.1.0-auth-bind-expiry-follow-up-2026-10-09.md) records the latest failed mixed trial and a second reproduced expiry race.
It links official sources to inspected code, deterministic controls, and unresolved causal questions.

The next work must follow this order:

1. Correct cached-proof expiry during authoritative binding. Preserve original-token expiry, signature checks, and current identity revocation.
2. Run the existing nine controls, affected Host tests, and the full local verifier against the corrected source.
3. Add bounded, secret-free stage evidence where aggregate failures cannot identify the cause.
4. Verify stream expiry, admission rejection, and client recovery as separate behaviors. Do not extend a captured Actor silently.
5. Build fresh source-bound artifacts. Repeat the unchanged mixed acceptance profile on SQLite and redb.
6. Complete provider upgrade, revocation, rollback, restore, and packaged-consumer gates against the matching release source.
7. Review the complete requirement matrix, deploy the verified preview, and publish 0.1.0 with its measured support limits.

Do not classify overload as permanent identity revocation. Do not classify every denial as a transient expiry race.
Do not widen TTL, capacity, deadlines, or acceptance thresholds to obtain a passing result.
Keep historical failures and their source identities. A later correction does not change an earlier trial's verdict.

The second expiry correction has passed ten focused controls, 54 affected Host tests, and the full local verifier.
The [verification record](/root/ROM/.superpowers/rom-010-auth-bind-expiry-verifier-execution-20261009/root-terminal-review.json) identifies the executed source and physical results.
The remaining next gate is stage-specific denial and stream investigation before fresh matching-artifact load acceptance.

The [upstream coordination record](rom-0.1.0-upstream-coordination-2026-10-09.md) tracks the owner's separate ROM UI and ROM-extras tasks.
Studio still pins alpha.3 while the inspected UI worktree declares alpha.5. Final artifact selection must resolve that difference through installed-consumer tests.
Provider task reports and unfinished capabilities must not silently expand the release support matrix.

### Resumed release work after unrelated chat messages

On 2026-10-09, the owner withdrew the latest chat composer redesign messages. They belonged to another conversation.
Do not add that redesign to this release. Preserve the previously agreed release scope and upstream ownership boundaries.

The public Host expiry experiment now has execution priority. It tests SQLite and redb through the same public Host contract.
It separates captured-stream termination, same-session proof renewal, fresh-stream disclosure, and confirmed identity revocation.
Its prepared source is not execution evidence. Actual terminal results must identify the resolved lockfile and executed sources.

The next load trial also needs bounded stage attribution. Aggregate HTTP categories cannot identify the rejecting authentication boundary.
Keep the workload, proof lifetime, admission capacity, and acceptance thresholds unchanged while diagnosing that boundary.

### Explicit correction of the mixed stream oracle

The [lease investigation](rom-0.1.0-mixed-stream-lease-investigation-2026-10-09.md) found a contradiction in the original raw-stream test.
Its final arrival occurs after 119,990 milliseconds, but each original stream captures an Actor with at most 30 seconds of validity.
The reader never reacquires that stream. Correct expiry therefore fails its zero-error oracle.

Preserve the original test and its failed results. Use a separately versioned, reviewed lease-recovery workload for continued observation acceptance.
This explicitly supersedes the earlier permanent-original-stream zero-error condition; do not relabel historical executions.
Keep 13,280 successful workload HTTP requests, all 12,000 scheduled groups, and final 12,000 Done records.
Count lease expiry, EOF, additional revalidation/acquisition traffic, fresh snapshots, and recovery gaps explicitly.
Require bounded authorized recovery with no unexpected stream failures or post-expiry disclosure. Do not extend a captured Actor.

The actual public Host controls passed two tests containing eight scenarios across SQLite and redb.
Installed-client recovery and fresh matching-artifact mixed acceptance remain open. Stage attribution is still required for HTTP failures.

### Bounded authentication attribution and review follow-ups

Optional Host authentication diagnostics now record fixed operation and stage counters. Capture is disabled by default.
Detailed failure records have a fixed capacity of 32. They contain no credentials, identities, or arbitrary error payloads.
Independent source review found no security or task-ownership blocker in the frozen implementation.
It requested attribution when original-token expiry occurs during binding and public documentation of accounting-counter limits.

A separate deterministic native-bind test reproduced the missing expiry record before its correction.
The correction adds that record without adding a clock sample, renewal, or validity extension.
The current affected Host suite passed 72 tests: 59 unit tests and 13 integration tests. Warning-denied Clippy also passed.
The [root execution review](/root/ROM/.superpowers/rom-010-auth-diagnostics-followup-green-20261009/root-terminal-review.json) records the current source and physical closure.
Historical credential test bodies remain unchanged. Their shared barrier helper now has module-visible access for the separate test.

The versioned lease reader has passed its private tests and focused independent review. Maintained coordinator integration is in progress.
The combined full verifier, real mixed trials, installed Studio recovery, and final release artifact gates remain open.

### Verified checkpoint after installed recovery and renewed load preparation

Studio now selects the pinned `rom-ui` alpha.7 archive. The earlier alpha.3/alpha.5 comparison remains historical investigation evidence.
The [installed recovery matrix](/root/ROM/.superpowers/rom-010-installed-studio-renewal-final-20261009/matrix-v2-result.json) passed all 20 cases.
It covers SQLite and redb, Chromium and WebKit, renewal, link revocation, provider revocation, original-token expiry, and cancelled renewal.
This result does not establish trusted-TLS deployment or mixed-load acceptance.

The [source-bound build record](/root/ROM/.superpowers/rom-010-load-auth-verifier-build-20261009/result.json) passed the full local verifier, fixture Clippy, and release compilation.
The retained execution identity binds 499 source files to the detached binary.
The current load fixture passed 24 tests. Final release acceptance still requires the matching clean-source packages and consumer tests.

Fresh lease-v2 preparation passed independent source review. Actual provider configuration matched before and after the same finite window.
SQLite material preparation completed both NSS commands. The parent rejected execution before it launched the coordinator.
The [rejection record](/root/ROM/.superpowers/rom-010-mixed-lease-v2-refresh-20261009/root-sqlite-rejected-entry.json) shows changed Playwright hardlink count and change time.
Its content hash, physical inode, and package alias remained unchanged. The actor that changed those metadata is unknown.
Use an isolated browser dependency graph for the next preparation. Do not weaken source fences or relabel this attempt as mixed acceptance.

Independent closure review found a retained provider network namespace mount after process shutdown.
Root removed only that attempt's owned mount through a bounded command.
The [closure review](/root/ROM/.superpowers/rom-010-mixed-lease-v2-root-continuation-20261009/actual-provider-and-rejected-entry-review.md) verifies the follow-up cleanup.
The lifecycle cleanup correction, fresh mixed runs, trusted-TLS recovery, native SQLite qualification, and final release gates remain open.

### Actual namespace cleanup and isolated browser preparation

The maintained lifecycle cleanup correction passed independent review and 54 affected pure tests.
The [actual kernel probe](/root/ROM/.superpowers/rom-010-provider-netns-cleanup-fix-20261009/kernel-probe-result.json) mounted and removed one namespace in a private mount namespace.
The [full verifier](/root/ROM/.superpowers/rom-010-netns-cleanup-full-verifier-20261009/root-terminal-review.json) passed against 3733 source files.
Its process exited with zero, its cgroup was empty, and its executed sources remained unchanged.

The actual private browser copy contains the same three Playwright packages and their 189 package files.
The [independent copy review](/root/ROM/.superpowers/rom-010-mixed-public-tool-isolation-20261009/evidence/actual-results-review.md) verified separate inodes and exact content.
The copy does not depend on shared package hardlink counts during execution.
The actual public export check passed without launching Chromium.

The first metadata seal failed because it expected five TypeScript inputs. The frozen manifest contains six.
Its recursive closure already contained these six inputs. A simple concatenation would also have rejected duplicate paths.
The reviewed correction retains each exact source once and rejects conflicting hashes or byte counts.
The [actual seal review](/root/ROM/.superpowers/rom-010-mixed-public-tool-isolation-20261009/evidence/actual-seal-review.md) verified all 64 runtime inputs.
The original failure, partial outputs, and source manifests remain unchanged.

### Fresh SQLite trial stopped before workload traffic

The [actual SQLite trial](/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-4f19800b8dafacdc2f4b1e90/result.json) failed during browser login.
The authorization-flow observation records 21677 milliseconds and the closed error labels `Other` and `Other`.
The browser helper waits for either the identification form or the callback, with finite deadlines.
This observation does not establish which browser observation failed or why.
Do not change selectors, TLS validation, or deadlines without further evidence.

The proxy recorded 86 requests, no rejected requests, no TLS errors, and no token responses.
All Host authentication operation counters remained zero. The mixed workload did not start.
This attempt therefore provides no performance acceptance for SQLite or redb.
The owned trial processes ended, its cgroups were empty, its source checks passed, and its private mount was removed.

The fresh provider configuration matched the original before and after this attempt.
The [provider closure record](/root/ROM/.superpowers/rom-010-mixed-isolated-provider-preparation-20261009/root-provider-terminal-review.json) verifies all 54 recorded process births were absent.
The maintained cleanup removed the provider namespace. All five test ports were vacant.
The next gate is a bounded login diagnosis with secret-free stage evidence, followed by the unchanged workload acceptance profile.

Playwright's [locator documentation](https://playwright.dev/docs/locators#locate-in-shadow-dom) states that its locators support open shadow roots by default.
This does not prove that the expected Authentik stage appeared in this attempt.
Authentik's [identification-stage documentation](https://docs.goauthentik.io/add-secure-apps/flows-stages/stages/identification/) describes a configurable stage, rather than a guaranteed page state.

### Login diagnosis and proxy closure

The [fresh login diagnosis](/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-login-probe-375a2c32f45bf4320f33e99f/result.json) established an authenticated session in 3910 milliseconds.
Both dispatch branches completed. The callback returned 303, and the proxy recorded one token response.
The diagnostic recorded no HTTP, transport, or page errors. All three owned processes physically exited with zero and no signal.
This result does not explain the earlier login failure. The workload was not dispatched.

The overall diagnosis failed its proxy closure gate. The pre-exit snapshot recorded one socket, despite later physical process closure.
Do not replace this failure with a passing process-exit result.
The original database and executed sources remained unchanged. The private workload mount was removed.
The [provider terminal review](/root/ROM/.superpowers/rom-010-isolated-login-provider-preparation-20261009/root-provider-terminal-review.json) confirmed normal provider cleanup and full before/after configuration equality.

A scripted regression reproduced the original snapshot ordering defect.
The corrected proxy waits for tracked close events and server callbacks before writing its snapshot.
It retains the zero-socket and zero-upstream criteria. Its one-second drain deadline fits within the existing shutdown allowance.
The [independent source review](/root/ROM/.superpowers/rom-010-proxy-drain-investigation-20261009/source-independent-review.md) passed against the held derivative.
A [real loopback helper test](/root/ROM/.superpowers/rom-010-proxy-drain-investigation-20261009/root-real-loopback-result.json) passed on Node 22.16.0.
This single HTTP connection test does not establish TLS, provider, or mixed-workload acceptance.

Node's [socket documentation](https://nodejs.org/download/release/v22.16.0/docs/api/net.html#event-close_1) identifies the close event as completed socket closure.
Its [HTTP documentation](https://nodejs.org/download/release/v22.16.0/docs/api/http.html#servercloseallconnections) recommends stopping acceptance before closing all connections.
These references support the correction. They do not identify the particular socket in the failed actual snapshot.
The next execution gate remains the complete, unchanged mixed workload with the reviewed closure correction.

### Current native qualification and fresh TLS failure

The [current native qualification](/root/ROM/.superpowers/rom-010-native-sqlite-qualification-readiness-20261009/execution-current/parent-result.json) passed all four admitted stages.
The linked-engine stage passed one test against SQLite 3.53.4. Batch recovery passed 42 tests.
Adapter runtime passed 179 core tests and 97 SQLite tests. Shared-consumer qualification passed 406 tests.
These stages overlap. Their counts do not identify unique tests.
Five adapter tests and ten shared-consumer entries were ignored.
The [independent actual review](/root/ROM/.superpowers/rom-010-native-sqlite-qualification-readiness-20261009/actual-independent-review.md) identified seven child fixtures exercised by passing parent tests.
Eight qualification cases remain unexecuted: five adapter cases and three shared-consumer cases.
The shared-consumer total includes four nested child summaries. It contains 402 top-level passes.

The [root terminal review](/root/ROM/.superpowers/rom-010-native-sqlite-qualification-readiness-20261009/root-terminal-review.json) verified physical closure, empty stage cgroups, absent recorded process births, and unchanged executed sources.
This native qualification does not certify the separate bundled SQLite workload binary or complete the release.

The [fresh mixed SQLite attempt](/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-51407f760a44fdf9f6fca751/result.json) failed before workload dispatch.
Login navigation failed after 826 milliseconds. The diagnostic recorded one transport failure on `host-login`, classified as `Other`.
The proxy recorded two TLS errors. These observations do not establish the certificate failure cause.
The corrected proxy closure succeeded with zero sockets and zero upstreams. The failed login still prevents acceptance.

The original database and executed sources remained unchanged. The private mount was removed, and the owned workload processes ended.
The [fresh provider terminal review](/root/ROM/.superpowers/rom-010-mixed-lease-v2-drain-provider-preparation-20261009/root-provider-terminal-review.json) confirmed normal cleanup, vacant ports, and equal before/after provider configuration.
No second adapter trial or automatic retry followed this failure.
The next investigation compares browser trust inputs with the earlier successful login diagnosis.

### Original-consumer feedback refresh on 2026-10-09

The coordinator read the live Astral Plane thread and its current application evidence.
The [native adoption report](/var/tmp/astral-release-rom-010/docs/evidence/native-rom-adoption.md) records fourteen registered Resource kinds and a frozen ROM candidate.
The application reports 193 passing Rust tests, one ignored test, and successful formatting, Clippy, and binary builds.
These are consumer-reported results with retained logs. They do not establish final ROM package acceptance.

Its comparison checked 314 Resources, 524 receipts, 221 events, and fourteen descriptors against a consistent predecessor snapshot.
Canonical values matched, and the source bytes remained unchanged. The snapshot contained no queued Work.
This comparison therefore does not establish unfinished Work recovery or whole-application release rollback.

Two current browser failure snapshots show `overloaded` after provider login.
Their retained cases concern signed-in navigation and recovery of a committed Friend creation.
See the [consumer release report](/var/tmp/astral-release-rom-010/docs/rom-release-2026-10-09.md).
The cause and generic recovery behavior need investigation before assigning these cases a passing result.

The consumer's complete verifier also reproduced an AI fixture startup failure.
An isolated 75-test tool suite passed with one test thread, but the combined serial verifier still failed.
A recorded consumer overlay extends fixture readiness polling while retaining the callback deadline and ownership assertions.
The default-concurrency combined result remains pending. Do not import this overlay without source review and affected regression tests.
A separate revoked-grant failure remains under investigation in the consumer report.
These findings supplement the release gates. They do not add the withdrawn chat appearance requests to the scope.

### Attributed mixed workload on 2026-10-09

The separate NSS comparison found the same stored CA fingerprint and trust flags in both inspected databases.
This observation does not prove which database Chromium used. It does not identify the earlier TLS failure cause.

The [attributed SQLite attempt](/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-c2728a06fb45271d96d0a08e/result.json) reached the complete workload.
Login succeeded after 6862 milliseconds. The proxy recorded zero TLS errors.
The generator scheduled all 12000 groups, with zero rejected scheduling and zero remaining active or pending groups.
The client recorded 8804 successful HTTP outcomes, 4029 overloaded outcomes, seven denied outcomes, one unknown outcome, and six timeouts.
These failures prevent performance acceptance. They cannot be removed by successful process cleanup.

All four normal readers received an initial snapshot. Four server-terminal events ended their streams.
No expected expiry or successful stream recovery was recorded.
Authentication diagnostics recorded capacity rejections, principally in `ProtectedMiddleware`.
Investigation must distinguish admission saturation, credential locking, expiry handling, and terminal stream semantics before changing the implementation.

The proxy completed drainage with zero sockets and zero upstreams. Eleven upstream `ECONNRESET` observations still produced a failed proxy result.
The [provider terminal review](/root/ROM/.superpowers/rom-010-mixed-tls-attribution-provider-preparation-20261009/root-provider-terminal-review-v2.json) confirmed physical exit, empty cgroups, vacant ports, and absent recorded process births.
Provider configuration before and after the attempt was equal. Executed sources and the original database remained unchanged.
The first audit used the wrong field for the workload unmount result. Its corrected audit checks the actual zero exit and absent signal.
The failed audit remains retained. No redb attempt or automatic retry followed this workload failure.

The [independent upgrade preparation review](/root/ROM/.superpowers/rom-010-r11-current-host-refresh-prep-20261009/v2/independent-review.md) rejected the current source proposal.
It found discarded terminal evidence when a cgroup postcheck failed, and compiler identity claims that exceeded preflight process receipts.
The next source proposal must preserve failure evidence and distinguish pre-execution context from actual executable observations.
No compiler or upgrade runtime was launched from the rejected proposal.

### Auth queue and AI fixture follow-up on 2026-10-09

The focused auth queue RED execution reproduced four failures and one passing test.
The [first GREEN execution](/root/ROM/.superpowers/rom-010-auth-queued-green-20261009/result.json) passed all 67 Host library tests.
The actual process exited with code zero. The owned cgroup was empty, and the captured Cargo sources remained equal.
The active authentication limit remains eight. Waiting callers do not own active work until admission succeeds.
Accepted work keeps its permit after caller cancellation. Result delivery follows physical permit release.

Review identified an arrival-order limit in the first waiting policy.
Four active stream checks can leave more request waiters than the eight request waiting positions.
A separate regression and bounded waiting-policy correction are pending. The full mixed workload has not passed.

The load harness now preserves fixed public error categories for terminal stream events.
Unknown server strings map to `other`; arbitrary strings are not retained.
Six RED parser tests failed before the correction. All 154 affected harness tests passed afterward.
See the [diagnostic verification record](/root/ROM/.superpowers/rom-010-stream-terminal-diagnostics-20261009/verification.json).
This improves failure attribution. It does not weaken stream acceptance or establish performance acceptance.

The AI fixture now observes physical child entry through a retained one-use signal.
It retains worker errors and panics after joining. Cleanup reports actual worker and Runtime ownership.
Production callback and delivery deadlines remain unchanged. Default-concurrency execution of the revised suite is pending.

The first remaining-native-gate attempt ran one passing native test, but its attribution monitor rejected the execution.
The short child ended before the required live process observation. The exact failed stage was not separately recorded.
Its [terminal evidence](/root/ROM/.superpowers/rom-010-native-remaining-gates-prep-20261009/execution-current/parent-result.json) remains failed qualification evidence.
A new source proposal distinguishes direct pinned execution from optional live process observations.
No remaining native gate is accepted from the failed attempt.

### Default-concurrency follow-up results on 2026-10-09

The stream-first regression reproduced one failure before the waiting-policy correction.
The request waiting limit is now twice the active limit. The separate stream waiting limit equals the active limit.
The default permits eight active tasks, sixteen waiting requests, and eight waiting stream checks.
Both classes still use the same FIFO active semaphore. The wait deadline and typed overflow refusal remain finite.
The [second Host GREEN execution](/root/ROM/.superpowers/rom-010-auth-stream-first-green-20261009/result.json) passed all 69 library tests.
This includes both arrival orders and explicit overflow, cancellation, closure, and permit-lifetime cases.
It does not establish full mixed-workload acceptance.

The first revised AI suite aborted in the new concurrent SQLite/redb case with a stack overflow.
Its [failed result](/root/ROM/.superpowers/rom-010-ai-readiness-default-green-20261009/result.json) remains retained.
Tokio stores joined futures inline on one task. See the [versioned join documentation](https://docs.rs/tokio/1.53.1/tokio/macro.join.html).
The recovery future measured 43368 bytes. This measurement is not the full poll-stack depth.
The test now boxes both child futures while keeping concurrent execution and the original assertions.
The joined future measured 72 bytes. Stack settings, test concurrency, and production deadlines remain unchanged.

The isolated concurrent case passed in 16.37 seconds.
The [complete tools suite](/root/ROM/.superpowers/rom-010-ai-readiness-boxed-suite-green-20261009/result.json) then passed all 85 tests in 23.77 seconds.
Both executions exited with code zero, retained equal source captures, and ended with empty owned cgroups.
This proves the tested fixture correction. Workspace feature unification and final extracted-package checks remain separate gates.
A focused formatting check passed after correcting one whitespace difference in `observation_gate.rs`.
The earlier execution records retain their original source identity.

Read-only review also identified a possible session TTL gap during queued authentication.
The next regression must check session expiry before and after queued work without extending token or proof expiry.
This source finding is not an established cause of the seven denied outcomes in the earlier workload.

### Remaining native qualification gates on 2026-10-09

The [fresh eight-gate execution](/root/ROM/.superpowers/rom-010-native-short-child-attribution-prep-20261009/execution-current/parent-result.json) passed all eight required selections.
The actual outer process exited with code zero. Each gate recorded zero exit, physical closure, unchanged sources, and no remaining group members.
The final owned cgroup was empty. No compiler rebuild occurred.

The four retained native binaries keep their original 768-input build ledger.
All 439 relevant inputs across eleven packages matched that ledger. The execution baseline separately captured 772 current Cargo inputs.
Changes in unrelated Host and AI modules do not become claimed build inputs of these retained binaries.
The final producer must still rebuild and verify extracted packages from its final selected source.

The large-header test passed in 10.04 seconds under a 1-GiB no-swap leaf and a 1-GiB private filesystem image.
Its private mount was unmounted, the loop device was detached, and both owned groups drained.
The diagnostics test passed in 8.34 seconds and emitted all eighteen fixed measurements.
The remaining cases covered genuine format10 refusal, restore, upgrade, candidate conversion, accepted-writer compatibility, and Runtime codec receipt replay.
The warm target allocation stayed at 1772449792 bytes. Retained logs totalled 9492 bytes.
The new admission used a 112-GiB free-space floor and a monitored 4-GiB target-growth limit.
These measurements are not a filesystem quota. The earlier failed 81410 attempt retains its original result and admission.

The new ordinary receipt proves direct pinned execution through owned delegation and typed test output.
It distinguishes that proof from optional live process observations. It does not invent a sampled native PID or executable mapping.
The separate header worker also retained its actual direct spawn event.
These qualification results do not establish final mixed-load, production-provider, whole-application rollback, or release-package acceptance.

### Session expiry and full verifier results on 2026-10-09

The session TTL regression confirmed expiry during queued authentication and authoritative binding.
The [RED execution](/root/ROM/.superpowers/rom-010-auth-session-ttl-red-20261009/result.json) passed two controls and failed four expiry cases.
Session validity is now checked before and after authentication work. Stream checks use the same validity contract.
Cancellation retains priority. Token expiry, proof expiry, and production deadlines remain unchanged.
The [GREEN execution](/root/ROM/.superpowers/rom-010-auth-session-ttl-green-20261009/result.json) passed all 75 Host library tests.
This defect is not an established cause of the earlier seven denied workload outcomes.

The [current load-host build](/root/ROM/.superpowers/rom-010-load-queued-auth-refresh-prep-20261009/build-current/result.json) completed with equal source captures and physical exit zero.
It includes the queue and session corrections. It uses bundled SQLite, separately from the selected native SQLite qualification profile.
Its build result does not establish mixed-workload acceptance.

The [full verifier execution](/root/ROM/.superpowers/rom-010-full-verifier-refresh-prep-20261009/result.json) passed the unchanged `./scripts/check` sequence.
The sequence passed 591 Node tests, workspace checks, default-concurrent Rust tests, documentation, compile fixtures, examples, and authentication feature checks.
The AI tools suite passed all 85 tests within the complete workspace execution.
The process exited with code zero. Final source and runtime checks passed. The owned group and cgroup were empty.
Target allocation increased by 11255808 bytes. Retained verifier output totalled 327394 bytes.
These results do not establish actual two-engine browser acceptance, final extracted-package acceptance, or complete release readiness.

### Actual mixed workload and corrected queue envelope on 2026-10-09

The [fresh SQLite workload](/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-10f4575e3707fd10a9203603/result.json) failed after 12850 traffic requests.
Its outcomes included 7930 successes, 4868 overload classifications, seven denials, one unknown outcome, and 44 transport timeouts.
Four streams refused session reacquisition. Work completion reached 11527 records against the required 12000.
The failure is retained. No redb workload followed this failed SQLite attempt.

The actual scheduler permits 32 active groups and 64 pending groups. Earlier queue sizing assumed sixteen callers.
Session reacquisition adds request-lane callers. Stream authentication uses a separate waiting lane but shares the eight active permits.
The diagnostic recorded 4643 authentication admission refusals. All 16543 admitted authentication tasks completed and released their permits.
These counters do not distinguish a full waiting lane from its five-second deadline.
Other HTTP overload classifications cannot all be attributed to authentication without matching per-request evidence.

Both provider state checks passed and compared equal in the same window.
The provider stopped with physical exit zero. Independent review confirmed absent process births, empty groups and cgroups, and vacant ports.
See the [actual execution review](/root/ROM/.superpowers/rom-010-queued-auth-sqlite-actual-review-20261009/actual-execution-review.md).

Two new regressions use 32 HTTP callers, four session reacquisitions, and five streams in both arrival orders.
The [RED execution](/root/ROM/.superpowers/rom-010-auth-32-profile-red-v2-20261009/result.json) failed both intended admission assertions.
The request waiting budget now equals five times the active capacity: forty positions at the default capacity.
Eight active positions, eight separate stream waiting positions, FIFO ordering, and the five-second deadline remain unchanged.
The overflow regression accepts forty waiting requests and rejects the next request.
The [GREEN execution](/root/ROM/.superpowers/rom-010-auth-32-profile-green-20261009/result.json) passed all 77 Host library tests.
The earlier full verifier preceded this change. A fresh full verifier and mixed-workload execution remain required.

The first focused RED entry refused the old 112-GiB floor before launching Cargo.
A separately admitted small execution used a 96-GiB floor and a monitored 256-MiB target-growth limit.
Earlier capacity policies and failed evidence remain unchanged. No cache deletion or unrelated process change occurred.

### Factor-five workload follow-up on 2026-10-09

The unchanged full verifier passed against the factor-five source: 591 Node tests, 77 Host tests, and 85 AI tool tests.
Session 68068 closed with physical exit zero, matching source inventories, an empty cgroup, and no OOM event.
Evidence is in `.superpowers/rom-010-full-verifier-factor5-20261009/` and its separate independent review directory.

The fresh optimized loadHost build passed in session 65715.
Its binary is `e5767b00ca5fd90e6165e64e2f6ae8a10d6c6d02d91c765112ff58a6ee0c8378`.
Its build identity is `5e492142b1e437a07b8dd8cc6dab53cc9066ece1575aa89a336bd5a1d2b8f161`.
This build uses bundled SQLite. It is separate from the selected native SQLite qualification.

The actual full SQLite workload remains failed. Session 80355 closed with physical exit one and unchanged source fences.
The result is `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-90072a6e5b4e452818219b5e/result.json`.
The driver recorded 9,284 HTTP outcomes: 8,072 successes, 1,204 overloads, seven denials, and one unknown outcome. It recorded no timeout.
The scheduler rejected 3,615 groups. These are scheduling outcomes, not additional HTTP responses.
Every authentication admission counter recorded zero capacity rejection. All 17,281 admitted authentication jobs completed and released their permits.
This confirms the queue correction within this workload. It does not explain the remaining overloads or establish performance acceptance.
The storage-call record measures aggregate adapter wall time. It does not attribute individual HTTP failures or measure exclusive CPU time.
No adapter call recorded a failure. That evidence does not support attributing the HTTP overloads to SQLite busy errors.

Both actual provider-state reads passed in the same window. The owned provider session 52964 then closed with `provider-terminal-green`.
The coordinator retains `drain-failed`. Only 11,488 of the required 12,000 Work records completed.
Physical process closure does not establish complete durable work. No redb workload followed this failure.
Further diagnosis must preserve the workload, authorization checks, and execution deadlines.

The separate R11 v7 current-host build passed in session 27882.
Its detached binary is `08165d1e7687f5958cad6bb397b8cc7fbdb4cb6b29afce74eb743601801a067f`.
Independent review verified compiler receipts, static linkage, source equality, process closure, and preservation of historical binaries.
The test wrapper emitted jobserver warnings. Two jobs were requested, but that handoff was not qualified; kernel resource bounds remained enforced.
This build result does not establish the full upgrade sequence, native qualification, or release acceptance.

### Genuine upgrade preparation failure and recovery

Three genuine NSS preparations passed before the provider started. Their profiles remained unused after the subsequent preparation failure.
The full sequence watcher started first and accepted the actual provider window. Registration passed; preparation then failed before any application stage.
Session 79157 therefore remains failed. It does not establish the 23-step upgrade result.

The sequence constructed a registration reference without its required `bytes` field. The preparation's first input verifier rejected that reference.
The exact source expression reproduced the failure. A bounded single-read reference with `path`, `bytes`, and `sha256` passed the same guard.
These guard tests do not establish that later preparation or application stages pass.

Registration had changed the provider callback. Independent review established absent application roots, closed preparation processes, and vacant application ports before recovery.
The unchanged restoration contract rejected concurrent changes before PATCH and verified the full original JSON after PATCH.
Actual recovery session 12423 passed with physical exit zero. The restored JSON matched original SHA256 `64cf9b91767d768e79e06725bdc3a7322399c9311d65c43ebaf4650962b8d6d5`.
Independent review then confirmed 50 recorded process births absent, five empty cgroups, and vacant provider/application ports.

Provider session 26794 retains `provider-terminal-incomplete`. Its terminal validator expected one launcher result; the current lifecycle records a namespace mount and a relay.
Both actual roles drained and namespace cleanup passed. The stale validator still prevents protocol acceptance of that recorded run.
A future validator must match both exact owned roles and reject duplicate, unknown, or undrained records. It must not accept arbitrary process counts.
All failed evidence remains preserved. The next sequence preparation applies these two harness corrections without changing application assertions or deadlines.

### Upgrade application entry failure and exact restoration

The v8 sequence passed registration and preparation. Its application entry then failed before any of the 15 application processes started.
Session 63920 closed with physical exit one. All three outer stages drained; independent review confirmed vacant application ports and absent process births.
This remains a failed upgrade attempt. It does not establish any of the 23 application assertions.

The entry read historical dependency metadata with the private-file guard. The actual file had mode `0644`; that guard requires private permissions.
The closed envelope, exact references, full plan binding, three NSS profiles, source fences, and runtime input freeze passed separate read-only checks.
The dependency read rejected the actual file before the application launcher existed. No missing application terminal record was invented.
A future derivative must use a new private copy with identical bytes. The historical file and private-file guard remain unchanged.

Root-owned restoration session 8062 passed with physical exit zero. The existing contract compared the entire registered JSON before PATCH.
It then verified the entire original JSON after PATCH, with SHA256 `64cf9b91767d768e79e06725bdc3a7322399c9311d65c43ebaf4650962b8d6d5`.
Only after physical restoration closure did root request the owned provider's shutdown. Provider session 46099 closed with `provider-terminal-green`.
The strict two-role validator accepted the actual mount and relay closure. The failed sequence remains unchanged.

A separate source review corrected two proposed overload tests. Managed gates and the Rayon pool prevent waiting for 32 simultaneous action-body entries.
The native IO barrier must precede those serialized boundaries. Exact generation-retry hooks belong in core unit tests, where `cfg(test)` is available.
These are test-design corrections, not implemented counters or executed saturation tests. Core overload attribution and full load acceptance remain open.

### Real upgrade traffic and acknowledgement-loss diagnosis

The v9 sequence crossed all entry guards and launched nine actual application children. The accepted-version login, backup, and current-version upgrade passed.
The current browser phase failed at `traffic-unknown-post-b0`. Sequence session 13476 therefore remains failed; the complete 23-step result remains open.
The fixture drained the nine actual children. It did not invent terminal records for the six children that never started.

The actual fault record reports upstream HTTP 200 and a dropped acknowledgement for the intended request. That record does not establish durable state.
The public consumer reads the Resource only after its mutation state becomes `unknown`. A successful submission instead triggers a deliberate assertion failure.
The retained diagnostic does not identify which branch failed. A failed follow-up read is therefore unproven.

Chromium's [HTTP transaction implementation](https://chromium.googlesource.com/chromium/src/+/b5035d51ba96304a61e151eed3e2eef67fc63086/net/http/http_network_transaction.cc) permits resend on reused connections before response headers arrive.
This supports a competing hypothesis: transparent browser retry can return the durable receipt after the injector disarms.
Actual retry remains unproven. The proxy's lifetime limit of 128 requests is another candidate; aggregate browser counts cannot identify the proxy ordinal.
A narrow browser experiment will distinguish these causes before another full upgrade attempt. It must preserve the unknown-outcome assertion and record bounded categories.

Restoration session 36336 passed with physical exit zero and exact original provider JSON equality.
Provider session 45202 then closed with `provider-terminal-green`. Independent review confirmed 55 process births absent and 15 empty cgroups.
All five provider/application ports were vacant. The used NSS profiles and failed database remain preserved.
No final package, full upgrade acceptance, or release publication follows from this partial result.

### Installed-browser acknowledgement retry experiment

Root executed one bounded HTTP loopback probe in session 76434. The process closed with exit zero.
The probe used the installed R11 Chromium executable and fictitious data. It did not use provider credentials or a database.

| Injected interruption | Matching POST requests | Native fetch result |
| --- | ---: | --- |
| Fresh connection, before response headers | 1 | Fetch error |
| Reused connection, before response headers | 2 | Success after transparent retry |
| Reused connection, after headers, truncated body | 1 | Body decoding error |

The probe observed nine HTTP requests. Browser contexts, servers, process group, and cgroup drained; source pins remained equal.
Evidence is retained in `/var/tmp/rom-ack-retry-907e5ddcd52679501536d966` and `.superpowers/rom-010-browser-ack-retry-root-execution-20261009/root-execution.json`.

These results establish transparent retry for this installed browser in the isolated HTTP case.
They do not establish the exact cause of the earlier TLS failure or prove a durable ROM commit.
The next fixture should interrupt the body after genuine success headers and retain the unchanged unknown-outcome assertion.
The original proxy request limit remains 128. A full upgrade rerun still requires fresh sources, build evidence, and unused browser profiles.

### Core overload attribution: executed RED stage

The first focused compilation failed because the private test Resource lacked `Debug`. Session 35798 did not establish behavior RED.
The fixture received three narrow corrections: `Debug`, removal of an unused import, and Rust 1.99's `try_update` atomic method.
The original compiler output and failed execution remain preserved.

Root then executed the same focused command in session 3758. Six controls passed; four boundary tests failed at their intended zero-counter assertions.
The tests covered action admission, IO admission, observation generation exhaustion, and actor generation exhaustion.
They retained cancellation ownership and completed their cleanup before the assertions.
Cargo exited 101. The runner accepted only the exact four failures and recorded completed ownership, unchanged sources, and an empty cgroup.
The warm target grew by 15,204,352 bytes. Evidence is in `.superpowers/rom-010-core-overload-red-execution-v2-20261009`.

This is an executed RED stage, not a successful counter implementation or load qualification.
The next change adds counting at these exact boundaries without changing admission limits, authorization, or cancellation ownership.
Terminal aggregate counts require completed async producers as well as drained native work. They do not attribute failures to individual HTTP requests.

Root added counting at the four reviewed boundaries and executed focused GREEN in session 14976.
All ten selected tests passed. Cargo and the outer runner exited zero; sources remained unchanged and the owned cgroup drained.
The warm target grew by 24,576 bytes. Evidence is in `.superpowers/rom-010-core-overload-green-execution-20261009`.
The change preserves existing limits, authorization, error semantics, and permit ownership. Closed intake and nested overload errors do not increment these counters.

This result covers the focused counter regressions. It does not replace the affected security tests, full verifier, or fresh mixed-load qualification.
Private Host capture, producer-completion review, and new load-build provenance remain required before interpreting workload attribution.

### Subsequent execution: Host evidence, full verifier, and current build

The following results extend the earlier focused counter tests. They do not establish complete release acceptance.

The private load Host now writes `load-final-core-overload-proof.json` after Host and reaction-worker completion.
The file has a 4,096-byte bound, private permissions, and an exclusive creation path.
It records a fresh baseline, final counters, producer completion, intake state, and owned work.
Counter saturation prevents a claim of exact totals. Authentication evidence capture still runs when core evidence capture fails.

Session 4640 completed all eight Host evidence tests with Cargo exit zero.
Its original wrapper rejected the log because two expected diagnostics appeared before the first test's `ok` line.
The original failed wrapper result remains preserved. A separate strict parser regression accepted only these two diagnostics for that test.
Twenty parser controls passed. Independent review accepted the retained physical completion and the separate classification record.

Session 20943 passed affected checks, then stopped the full verifier during a capacity scan.
GNU `du` reported a missing temporary `target/debug/deps/rmetamFZxgX` directory. The failure was not a disk-growth violation.
The corrected measurement helper permits one classified temporary-directory race, followed by a complete bounded rescan.
It does not accept a partial total. Twenty-five helper controls passed; the original failed execution remains preserved.
The installed Rust compiler's [exact source](https://github.com/rust-lang/rust/blob/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/compiler/rustc_metadata/src/fs.rs) creates `rmeta` temporary directories.
This source supports the traversal-race explanation; it does not identify the process that removed this directory.

Root then executed session 21315. Affected core tests passed ten cases; the private Host passed 32 cases and both Clippy checks.
The unchanged `./scripts/check` completed with physical exit zero. It recorded 591 passing Node tests and 206 successful Rust summaries.
The Rust summaries include repeated suites and 21 ignored cases. They are not counts of unique tests.
Both phases completed and drained without OOM. Independent review confirmed matching source and tool identities.
Evidence is in `.superpowers/rom-010-current-core-host-verifier-rmeta-prep-20261009`.

Session 2512 built the current optimized load Host with locked offline dependencies. The compile took 46.82 seconds.
Its detached binary is `bd1e63b7826f2a84d3985781b37663b3573ce500e6a291a3421662aaf81d019f`.
Independent review confirmed the artifact, compiler context, source identities, and physical closure.
The build uses default bundled SQLite. It does not replace the separate selected SQLite 3.53.4 qualification.
Evidence is in `.superpowers/rom-010-current-load-host-build-rmeta-prep-20261009`.

### Subsequent execution: unchanged SQLite workload and HTTP boundary

Session 17215 executed the unchanged full SQLite workload with this current binary. It failed the original acceptance criteria.
The trial retained 12,000 scheduled groups, maximum active 32, maximum pending 64, and the original HTTP and Work requirements.

| Observation | Actual result | Acceptance requirement |
| --- | --- | --- |
| Scheduling | 12,000 groups; 3,936 scheduling rejections | Complete workload without these rejections |
| HTTP | 8,942 outcomes: 7,520 success, 1,411 overloaded, ten denied, one unknown | 13,280 successful workload requests, plus the separate operator requests |
| Durable Work | 11,396 completed records | 12,000 completed records |
| Core counters | Four final counters zero; fresh zero baseline; exact terminal scope | Diagnostic evidence, not workload acceptance |
| Authentication | Zero capacity rejections and zero overloads; four retained `CachedBind` denial-stage records | Preserve actual failures and authority checks |

Host, proxy, and login completed with physical exit zero. The client exited one; the coordinator retained its coarse `drain-failed` status.
The four normal streams and the unread stream retained 16 actual expiry/recovery cases without stream errors.
Root verified the full provider JSON before and after the trial. Both 1,891-byte values matched SHA256 `64cf9b91767d768e79e06725bdc3a7322399c9311d65c43ebaf4650962b8d6d5`.
Only then did root request the owned provider stop. Provider session 35477 completed with physical exit zero.
Independent review confirmed absent owned processes, empty groups and cgroups, vacant ports, namespace cleanup, and unchanged source identities.
No redb trial followed this failure. Evidence is in `.superpowers/rom-010-current-core-host-mixed-prep-20261009`.

The core and authentication counters exclude their measured overload boundaries in this trial. They do not explain every HTTP rejection.
Source inspection identified another boundary in `crates/rom-http/src/request.rs`.
HTTP decoding acquires a separate body permit before the actor resolver. The routes retain that permit through their awaited operation.
The actual Host uses the default 16 body permits and five-second body timeout; the application core uses 32 action and IO permits.
Both unavailable body permits and body-read timeouts map to `overloaded`. Neither result increments the four core counters.

The aggregate difference between protected-middleware admissions and generic-resolver admissions is 1,410.
Read, query, replay, and commit overloads also total 1,410. One additional reserve overload remains separate.
This agreement supports an HTTP-admission hypothesis; it does not provide request-by-request attribution.
A deterministic test of the real HTTP route remains necessary before any capacity or waiting-policy change.
The scheduling rejections also remain a separate performance problem.

[Tokio's semaphore contract](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) distinguishes immediate acquisition failure from asynchronous waiting.
[Tower concurrency limits](https://docs.rs/tower/latest/tower/limit/concurrency/index.html) bound active requests.
[Tower load shedding](https://docs.rs/tower/latest/tower/load_shed/index.html) rejects work when the inner service is not ready.
These primary sources explain the boundary choices. Their current documentation versions do not establish ROM's installed dependency versions or this trial's cause.

### Subsequent execution: narrower TLS acknowledgement probe

Session 19449 exercised the proposed TLS injector with installed Chromium and a fictitious upstream.
The first matching POST received success headers, then a body-read error. The browser did not transparently repeat that first invocation.
A later fictitious verification read and an explicit repeated POST succeeded. The unchanged 128-request proxy limit also rejected its next request.
The owned browser, proxy, and upstream closed. Independent review accepted the narrow physical and source evidence.
Evidence is in `.superpowers/rom-010-r11-actual-injector-probe-prep-20261009`.

This result covers native browser fetch, not public-client mutation state or a durable ROM commit.
The retained consumer's top-level `dist/traffic.js` does export the public `client()` helper. An empty `dist/assets` directory does not establish its absence.
Fresh public-client and complete 23-stage upgrade execution remain open. The separate current R11 Host must have matching build and source evidence.

### Subsequent execution: HTTP admission characterization

Session 1075 passed Clippy and three tests of the HTTP boundary. The tests use the actual router and decoder.
Sixteen requests held the body permits while their actor resolvers waited. Sixteen further requests received `429` before authentication or core admission.
Cancellation returned eight owned permits. Replacement requests then entered the resolver without a core overload.
The separate incomplete-body test reached authentication before its body timeout produced `429`.

The unchanged full verifier also completed with physical exit zero. Independent review confirmed matching sources, empty process groups, and empty cgroups.
These tests distinguish overload mechanisms. They do not attribute every overload in session 17215 or establish workload acceptance.
Evidence is in `.superpowers/rom-010-http-admission-characterization-prep-20261009`.

### Subsequent execution: reference budget and verifier failure

The reference Host now sets `http_limits.bodies = 32`. The framework default remains 16.
This setting changes two independent pools: generic HTTP bodies and blob JSON bodies. BlobService's four-operation limit remains unchanged.
The comparison retains 12,000 groups, active limit 32, pending limit 64, the 13,280-request target, and 12,000 completed Work records.
The detached binary from the actual 16-permit trial remains preserved.

Session 76216 passed Host Clippy and all 32 Host tests. Its full verifier failed with physical exit 101.
The failing reconciliation test expected `Completed` after later provider acceptance, but received `Unresolved`.
Both phases completed and drained. Independent review confirmed matching sources and empty groups and cgroups.
No new build or workload comparison followed this failure.
Evidence is in `.superpowers/rom-010-load-http-bodies32-refresh-prep-20261009`.

The failing test inherits a 20 ms verification deadline from its delivery setting. Adjacent positive verifier tests use an explicit two-second deadline.
ROM supervises a spawned task and applies the timeout to its `JoinHandle`.
[Tokio 1.53.1](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn.html) does not poll the callback synchronously when it spawns the task.
Its [timeout contract](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html) concerns the wrapped future, which here is the join handle.
Scheduling delay is a supported hypothesis; the failed trial did not record the exact callback outcome.

Three separate diagnostic executions passed with unchanged source: session 52154 passed 19 tests, and sessions 63507 and 43775 each passed one test.
The last execution covered the existing timeout, panic, and malformed-evidence control. It preserved held work without granting an unsafe resend.
These results do not erase the full-verifier failure or prove its cause.
A test candidate now separates the positive verification deadline from the delivery deadline and asserts that the first callback actually ran.
Verification of this candidate remains pending.
Production timeouts and reconciliation rules remain unchanged. Evidence is in `.superpowers/rom-010-operator-reconciliation-diagnosis-prep-20261009`.

### Subsequent execution: public SDK acknowledgement handling

Session 47810 used the retained public SDK bundle through the actual proposed TLS injector, with a fictitious upstream.
After success headers and a truncated response body, submission rejected and the pending mutation became `unknown` without a result.
A public-client read succeeded. An explicit retry of the same pending mutation succeeded with an unchanged request fingerprint.
The two matching POSTs comprise the first submission and that deliberate retry. The first submission was not automatically repeated.

The proxy forwarded 128 requests and rejected request 129 with `403`. No other proxy refusal occurred.
Independent review confirmed source identities, physical completion, vacant ports, and empty process groups and cgroups.
Evidence is in `.superpowers/rom-010-r11-public-client-ack-probe-prep-20261009`.
This proves the retained SDK's behavior for the fictitious transport. Durable commit, provider integration, TLS trust, and complete R11 acceptance remain open.

### Subsequent execution: positive verification deadline

Session 89721 passed reconciliation Clippy and all 19 reconciliation tests. The unchanged full verifier then completed with physical exit zero.
The positive test uses a two-second verification deadline and confirms that its first callback ran.
The existing pending-verifier control retains 20 ms. Production timeouts and reconciliation rules remain unchanged.
Independent review confirmed matching sources, empty process groups, and empty cgroups.
The earlier full-verifier failure remains preserved. This passing run does not establish its exact cause.
Evidence is in `.superpowers/rom-010-operator-reconciliation-positive-budget-check75-prep-20261009`.

### Subsequent execution: HTTP32 workload comparison

Session 88190 built the current reference Host in the optimized release profile. Compilation completed in 23.80 seconds.
Independent review verified the detached binary, matching sources, physical completion, and empty owned process groups.
The earlier HTTP16 binary remains preserved.

Session 44190 exercised the unchanged SQLite workload with HTTP32. The workload failed with physical exit one.
The comparison retained 12,000 scheduled groups, active limit 32, pending limit 64, and the original acceptance requirements.

| Measurement | Earlier HTTP16 trial | HTTP32 trial |
| --- | ---: | ---: |
| HTTP requests | 8,942 | 10,733 |
| Successful HTTP outcomes | 7,520 | 10,724 |
| HTTP overload outcomes | 1,411 | 0 |
| Scheduler refusals | 3,936 | 2,290 |
| Work records | 11,396 | 11,821 |
| Completed Work records | 11,396 | 11,821 |

The HTTP32 trial also recorded six HTTP denials and three unknown upload outcomes.
These aggregate results do not identify each request's cause. Authentication diagnostics recorded four cached-bind denials and successful renewal.
All existing Work records completed. The trial still lacked 179 records against the required 12,000.
Sixteen stream recoveries completed. No client deadline was reached.

The provider's full before and after JSON were byte-equal: 1,891 bytes with SHA `64cf9b91767d768e79e06725bdc3a7322399c9311d65c43ebaf4650962b8d6d5`.
ROOT requested the owned provider stop after this comparison. Session 66630 then completed with physical exit zero.
Independent review confirmed restoration and physical closure. No redb trial followed the failed SQLite workload.
Evidence is in `.superpowers/rom-010-current-http32-mixed-prep-20261009` and `.superpowers/rom-010-current-http32-mixed-execution-review-20261009`.

Source inspection identified repeated authority checks during projected queries. The storage observer recorded 771,283 load calls across the Host lifecycle.
This is a throughput hypothesis, not proof of the scheduler refusals' cause.
The next characterization must preserve current denials, expiry, callback behavior, and final authority checks before any optimization.
Workload acceptance, matching package qualification, and release completion remain open.

### Authority characterization after the HTTP32 trial

Session 4357 executed six core authority tests. All six passed; 189 tests were filtered out.
Compilation took 4.67 seconds. The tests took 0.01 seconds.
Independent review confirmed physical exit zero, drained ownership, no OOM, and unchanged sources.

A projected query returned 50 rows through 52 gate callbacks and 104 authoritative key reads.
The fixture gate reads two keys per callback. This count describes the current implementation, not a minimum security requirement.
An external authority change during disclosure or after the last row caused the entire query to return `Denied`.
Expiry during selection, local revocation before the next query, and an initial gate denial also prevented disclosure.
This fixture uses a bounded memory store. It does not measure the real identity gate or a durable adapter.

Evidence is in `.superpowers/rom-010-authority-batch-characterization-prep-20261009/actual-authority-batch`.
Independent review is in `.superpowers/rom-010-authority-batch-independent-review-20261009/execution-review.json`.
The next experiment measures the real identity gate with SQLite, including fresh provider, link, and user reads.

Source inspection also found repeated SQL preparation in SQLite's keyed reads.
[rusqlite's prepared-statement cache](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Connection.html#method.prepare_cached) reuses SQL statements, not query results.
[SQLite documents SQL preparation](https://www.sqlite.org/c3ref/prepare.html) as a separate step before execution.
Statement reuse is a candidate optimization. No implementation or measured gain is claimed here.
Fresh values, revisions, authorization checks, and error behavior must remain unchanged.

Session 50515 executed the real identity profile. One test passed; eight tests were filtered out.
Ten projected queries each returned 50 complete documents through 52 identity checks and 156 fresh identity reads.
The actor came from an actual signed fixture token, provider activation, and identity binding.
The test also confirmed denial at the actor's exclusive expiry boundary and drained all owned work.

| Instrumented measurement | Median of ten queries |
| --- | ---: |
| Complete query | 3.2418795 ms |
| Identity gate, inclusive | 2.5786115 ms |
| Authorization reads, inclusive | 2.1636075 ms |

These spans overlap. Do not add them or interpret their difference as exclusive decode time.
The fixture uses SQLite in memory and the debug test profile. Signing, setup, and identity binding were outside query measurements.
A separate replay of 52 actual gate checks against cloned fixture rows took 0.507584 ms.
This replay does not reproduce storage, runtime accounting, or concurrent query execution.

Independent review confirmed physical closure, unchanged sources, no OOM, and 180,224 bytes of target growth.
Evidence is in `.superpowers/rom-010-real-identity-gate-profile-prep-20261009/actual-identity-profile`.
The result supports a prepared-statement reuse experiment. It does not establish the cause of the mixed workload's refusals.

### SQLite statement reuse and full verification

The implementation now reuses prepared statements in two keyed-read paths: ordinary persistence and the bounded native reader.
Every call still executes its SQL query. It does not cache identity values or authorization results.
The native reader still checks the byte budget before copying or decoding the value.

Six SQLite tests passed before and after this change. They cover both storage formats.
The cases include missing keys, revisions, deletion, invalid values, transaction rollback, schema errors, recovery, and byte limits.
Earlier fixture compilation and format-selection failures remain in the retained evidence. They were corrected before the successful baseline.

Session 25598 repeated the real identity profile. All ten queries retained 52 checks and 156 fresh identity reads.

| Instrumented measurement | Before reuse | After reuse |
| --- | ---: | ---: |
| Complete query, median | 3.2418795 ms | 2.7074755 ms |
| Identity gate, inclusive median | 2.5786115 ms | 2.0914465 ms |
| Authorization reads, inclusive median | 2.1636075 ms | 1.715379 ms |

The complete-query median decreased by approximately 16.48% in this pair of ten-query trials.
These are debug measurements against SQLite in memory. The spans overlap; this comparison is not a mixed-workload qualification.

Session 31505 stopped at a Clippy warning in the new authority test. The full verifier did not run in that attempt.
After the test-only correction, session 38625 passed affected Clippy and the unchanged `./scripts/check`.
Independent review confirmed matching sources, physical exit zero, drained ownership, and no OOM.
Target allocation increased by 13,828,096 bytes during the successful combined check.

Evidence is in `.superpowers/rom-010-sqlite-cached-read-cached-trial-prep-20261009/actual-cached-read`,
`.superpowers/rom-010-real-identity-gate-cached-profile-prep-20261009/actual-identity-profile`,
and `.superpowers/rom-010-sqlite-statement-reuse-full-check-retry-prep-20261009`.
Independent review is in `.superpowers/rom-010-sqlite-cached-read-baseline-independent-review-20261009/actual-full-check-retry-review.json`.
The next step is a matching LoadHost build and the unchanged SQLite mixed workload.
Native qualification, package qualification, deployment, and release acceptance remain open.

### Optimized SQLite mixed workload after statement reuse

Session 62752 built the current optimized LoadHost. Compilation took 49.21 seconds.
Independent review matched the detached binary to the current sources and confirmed physical exit zero.
The build used bundled SQLite. It did not qualify the selected external SQLite library.

Session 74931 then executed the unchanged SQLite mixed workload. It failed with physical exit one.
The client offered all 12,000 groups but refused 5,337. It reached the unchanged active and pending limits of 32 and 64.
The client issued 7,358 workload requests: 7,348 succeeded, five were denied, and five had unknown outcomes.
These whole-run counts include warm-up. Measured-phase results separately recorded four denials and three unknown upload outcomes.
All four operator requests succeeded. All 16 stream recoveries completed; no client deadline was reached.

| Fresh scheduler observation | Result |
| --- | ---: |
| Refusals with available active slots | 156 |
| Refusals with all active slots occupied | 5,181 |
| Largest due-arrival batch | 1 |
| Maximum offer lateness | 5.769796 ms |
| Maximum queue residence | 2,780.523708 ms |

The fresh diagnostics expose a dispatch-order defect in the generator. However, that defect alone does not explain most refusals.
The Host retained 11,450 Work records, all Done. It lacked 550 records against the required 12,000.
Core overload counters remained zero. Their lifecycle scope does not establish each HTTP request's cause.

Normalized adapter timings were worse than the earlier HTTP32 trial in this single comparison.
They include waiting and concurrent activity. They do not isolate statement reuse, transaction work, or shared-host contention.
The next probe must separate connection-lock waiting, preparation, publication, and native commit before another database optimization.

Independent review verified absent owned processes, empty cgroups, unchanged sources, and restored namespace backing.
Both full provider GETs returned 1,891 byte-equal bytes with SHA `64cf9b91767d768e79e06725bdc3a7322399c9311d65c43ebaf4650962b8d6d5`.
ROOT requested the provider stop after this comparison. Session 61634 completed with physical exit zero.
No redb trial followed the failed SQLite workload.

Evidence is in `.superpowers/rom-010-current-sqlite-cached-mixed-prep-20261009` and
`/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-9a080de42e09e6381147a1d6`.
Independent review is in `.superpowers/rom-010-sqlite-cached-read-baseline-independent-review-20261009/actual-current-mixed-failure-closure-review.json`.
The initial 70 GiB preparation is preserved. The executed preparation used ROOT's new 68 GiB floor and separate growth monitors.
These monitors are not filesystem quotas. The floor change did not alter workload or security requirements.

### Scheduler correction and SQLite diagnostic regression tests

The scheduler now dispatches queued groups before each new offer. It also dispatches each newly admitted group when a slot is available.
This correction preserves FIFO order, planned arrival times, and the limits of 32 active groups and 64 pending groups.
Four regression tests cover capacity, failed continuations, cancellation, and unchanged 10 ms arrival timestamps.
The affected suite passed 19 tests. ROOT's retained repeat took 87.701484 ms, with no failures or skipped tests.
The actual mixed workload has not yet qualified this correction. The full verifier must run again after the diagnostic changes.

Session 12672 executed five diagnostic tests against genuine SQLite mutations and reactions.
Compilation took 8.69 seconds. All five tests failed for the intended missing-evidence or missing-rejection conditions; 35 tests were filtered out.
Cargo returned 101. The owned processes exited, the cgroup was empty, and captured sources remained equal.
The private output qualifier rejected the new compiler's thread PID formatting. Its original failure remains in the evidence.
This qualifier failure does not change the actual test results. Supplementary review must verify their exact failure locations.

These tests expose a missing diagnostic lifecycle, not a production mutation failure.
The next test must exercise the same Host `serve` body used by the load fixture.
It must verify real terminal counters after HTTP and reaction producers stop. Diagnostic results cannot establish release throughput acceptance.

Evidence is in `.superpowers/rom-010-scheduler-fifo-implementation-20261009/root-affected-repeat.log` and
`.superpowers/rom-010-mixed-stage-red-prep-20261009/actual-stage-red`.

The first Host integration attempt, session 30524, failed before the lifecycle started.
Its fixture used different persisted storage limits. This failure did not qualify as the intended diagnostic regression.
The corrected fixture opens its database through the same `Database::open` configuration as `serve`.
Session 30977 then exercised the actual shared `serve` body with an ephemeral listener and a file-backed SQLite database.
The Host and reaction worker stopped before the test checked the terminal evidence.
The test failed at the expected missing-file read: zero passed, one failed, and 40 filtered out.
Cargo returned 101. Source captures remained equal, owned processes exited, and the cgroup reported no OOM or PID-limit failure.

Evidence is in `.superpowers/rom-010-mixed-stage-integration-red-prep-20261009/actual-integration-red` and
`.superpowers/rom-010-mixed-stage-integration-red-retry-prep-20261009/actual-integration-red`.
The diagnostic implementation and subsequent successful regression run remain required.

### Diagnostic lifecycle verification and current SQLite load result

The Host now records SQLite stage and publication evidence after its HTTP and reaction producers stop.
The evidence requires a fresh baseline and complete, nonoverflowed counters. It is diagnostic evidence, not release acceptance.
Session 88722 passed all six focused tests. These include the actual shared Host lifecycle with a file-backed database.
Session 65638 passed formatting, all-feature Host Clippy, 41 Host tests, and the full local `./scripts/check` verifier.
Independent review confirmed unchanged captured sources and physical closure for both executions.

Session 37766 built the optimized diagnostic Host in 17.47 seconds. Its only selected feature was `storage-stage-timings`.
This build used bundled SQLite. It does not qualify the selected external SQLite 3.53.4 release configuration.
The detached binary has SHA `863cd5008f6006ed09c0807067c6d1529660cd15744a9d5d1d808000274f6bd4`.

Session 91425 executed the unchanged SQLite mixed workload and failed with physical exit one.
It issued 8,467 workload requests: 8,430 succeeded, seven were denied, and 30 timed out.
All 12,000 groups were offered. The scheduler refused 4,343 groups, all with occupied active capacity; none had available active slots.
Maximum queue residence was 8,365.925234 ms. Thus, the FIFO correction removed the observed idle-capacity refusal defect in this run.
It did not establish workload acceptance. Eleven authentication admission overloads and four overloaded terminal streams were also recorded.
No redb trial followed this failed SQLite run.

| Diagnostic operation | Native commits | Time inside `tx.commit()` | Mean per commit |
| --- | ---: | ---: | ---: |
| Resource commit | 1,955 | 35,980.306029 ms | 18.404249 ms |
| Work update | 6,447 | 59,148.479059 ms | 9.174574 ms |

These are aggregate elapsed times. They include concurrent waiting and descheduling; they are not CPU costs or individual request spans.
The timer excludes ROM's later acknowledgement checkpoint. SQLite may still run its automatic WAL checkpoint inside the timed commit.
SQLite documents synchronization on FULL WAL commits and automatic checkpoints on the committing thread.
See [WAL performance](https://www.sqlite.org/wal.html#performance_considerations) and [automatic checkpoints](https://www.sqlite.org/c3ref/wal_autocheckpoint.html).
No syscall evidence identifies synchronization as the sole cause. The next experiment must measure rather than assume that cause.

The Work publisher updates its header even when its before and after values are equal.
Publication counts alone do not prove additional WAL frames: SQLite can avoid writes when page contents remain unchanged.
The focused test below characterizes unchanged Work publication on file-backed WAL/FULL SQLite, including reopen and changed-state controls.
Any optimization must retain read-set validation, atomic publication, acknowledgement semantics, and FULL durability.
Existing Work batching supports bounded ordered operations. Independent HTTP receipt group commit would require a larger contract change.

The provider's full before and after responses matched all 1,891 bytes before ROOT requested its stop.
Provider session 67226 then failed cleanup. Its original terminal and supervisor failures remain preserved.
ROOT separately removed only the recorded owned namespace mount after fresh process and mount-identity checks.
Independent review subsequently confirmed 51 absent process births, 30 absent groups, empty cgroups, five vacant ports, and restored namespace backing.
This proves physical closure after remediation. It does not convert either failed execution into a pass.

Evidence is in `.superpowers/rom-010-current-sqlite-stage-diagnostic-mixed-prep-20261009` and
`.superpowers/rom-010-current-sqlite-stage-diagnostic-mixed-independent-review-20261009/execution-review.json`.
The independent review has SHA `6d2429dac1dfe6052391a94838c8677b3940a2ef36a3da76fe84fa9bf3c63470`.
The diagnosis and source pins are in `.superpowers/rom-010-wal-full-commit-diagnosis-20261009`.

### Idle Work characterization rejects the first performance hypothesis

Session 32864 passed three real file-backed SQLite tests: empty claims, live-lease idle claims, and completed-work idle batches.
Compilation took 1.91 seconds. The tests took 0.03 seconds, with zero failures and 108 filtered tests.
Each case recorded 16 header publications. Each case added zero WAL bytes and zero WAL frames during its idle operations.
The tests also checked unchanged state and reopen behavior. The lease test checked a genuine durable transition after lease expiry.
WAL mode and `synchronous=FULL` remained enabled. A setup checkpoint established a fresh WAL baseline before each measured case.

These results reject the assumption that the unchanged header UPDATE necessarily adds WAL writes in these conditions.
They do not isolate syscall latency in the mixed workload or prove that idle SQL has no execution cost.
No production optimization was applied from this hypothesis. The next probes concern actual changed commits and concurrent authoritative credential binding.

Independent review confirmed matching source captures, absent owned processes, an empty cgroup, and no OOM or PID-limit event.
Evidence is in `.superpowers/rom-010-native-work-idle-baseline-prep-20261009/actual-native-work-idle`.
The independent execution review has SHA `8be70d487a6d1cc7bafda99ac8497bfd959653e8c4b854d570977b2b12ad90f6`.
These characterization tests do not qualify workload or release acceptance.

The mixed Host's separate final Work proof records 11,511 records, all Done. This is 489 below the required 12,000.
Its SHA is `f5e64009797b712e571a3917de7665bfe2fc1e1524d6e31d3624a8c37eea181c`.
Authentication admission can reject before core work starts. Its retained counters combine queue capacity and queue-wait deadline rejection.
Therefore, eleven authentication overloads and zero core overload counters are not contradictory.
Holding the session credential mutex across authoritative binding is a candidate source of serialization; this run does not prove its contribution.
Any concurrency change must retain current identity checks, serialized proof renewal, original expiry, and post-bind session checks.

Session 47737 subsequently passed workspace formatting, SQLite all-targets/all-features Clippy, SQLite tests, and the unchanged full local verifier.
The focused SQLite library run passed 106 tests and intentionally ignored five. Two integration tests also passed.
The five ignored tests require preserved format10 fixtures or a separately admitted large-profile envelope. This run does not qualify those scenarios.
The full Node suite passed 602 tests. Other Rust summaries include repeated feature, consumer, and documentation executions, not unique test counts.
Independent review confirmed unchanged captured sources, physical exit zero, absent owned processes, and an empty cgroup without resource-limit failures.
Its evidence is in `.superpowers/rom-010-native-work-idle-full-check-independent-review-20261009/execution-review.json`.

Cargo also reported a documentation output collision between the `rom` CLI binary and the `rom` library.
The verifier exited zero, but this warning remains a documentation-packaging issue to resolve before release.
Cargo tracks this class of collision in [issue 6313](https://github.com/rust-lang/cargo/issues/6313).
The next changes must also distinguish the credential mutex from the shared Runtime gate; removing one lock does not remove the other.

### Paired credential tests distinguish the two locks

Session 2107 passed two credential concurrency tests with genuine activated identity proof and the same blocked Runtime gate.
Eight callers with one shared cache held one core IO slot. Eight independent caches held eight slots.
Neither case completed an authoritative bind while the gate remained blocked.
Removing the cache mutex alone would redistribute waiting. These tests do not demonstrate higher throughput.
No production mutex change was applied.

Independent review confirmed matching captured sources, absent owned processes, and an empty cgroup without resource-limit failures.
Evidence is in `.superpowers/rom-010-auth-cache-gate-runner-independent-execution-review-20261009/execution-review.json`.
The review SHA is `cf245ff3cee35b4f30f53d2dbbf9709128e30328a53ed8ff159ed88ef7ee60de`.

### Changed Work batching reduces measured synchronization

Session 49473 passed four file-backed SQLite tests under the actual mixed-backing filesystem topology.
The tests retained WAL mode and `synchronous=FULL`. They checked ordered results, receipts, and state after reopen.
Separate failure tests confirmed rollback before commit and committed recovery after an unknown acknowledgement outcome.
The existing atomic Work API changed the same 16 Work records as the singleton path.

| Case | Operations | Native commits | WAL frames added | Traced phase sync calls | Sync wall time | Total phase time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Work singletons | 16 | 16 | 64 | 17 | 82.036 ms | 91.789002 ms |
| Work atomic batch | 16 | 1 | 6 | 2 | 6.596 ms | 12.472865 ms |
| Resource bundles | 16 | 16 | 261 | 17 | 57.586 ms | 77.521428 ms |

Independent review reproduced the timestamp join using integer nanoseconds and exact fixture paths.
The trace had 164 parsed sync calls. Of these, 128 occurred outside the measured phase windows; no rows were unknown.
These are traced debug-build fixtures. Tracing adds overhead; the values do not establish production throughput or explain every mixed-workload wait.
The next investigation concerns utilization of the existing batch API in the mixed workload. Ordering barriers and current authorization must remain intact.

The original session 25427 passed three tests and failed one cross-database snapshot assertion.
The two independent databases had different generations. Their records, roots, retry epochs, operator receipts, and limits matched exactly.
The corrected test compares those semantic fields between databases. It still compares each complete snapshot across that database's reopen.
A runner filename collision also masked the failed test exit with EEXIST. The original test exit, trace, and parent failure remain preserved.
Independent review confirmed physical closure of the failed execution. Neither failure was converted into a pass.
Session 67636 later refused admission before any owned test launch; its capacity refusal is also preserved.

The successful trace evidence is in `.superpowers/rom-010-native-work-sync-probe-retry-prep-20261009/actual-trace`.
Its independent review is `.superpowers/rom-010-native-work-sync-trace-retry-independent-review-20261009/execution-review.json`.
The review SHA is `a292e561f07b3c43d10bdc1279ef9e15b6ac0da386469e669db89da384bd052f`.
The unchanged mixed-workload acceptance remains outstanding: its last final proof had 11,511 completed Work records against 12,000 required.

The CLI manifest now sets `doc = false` on its binary target to prevent the library name collision during default Rustdoc generation.
Cargo documents this field in [the target reference](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-doc-field).
This configuration change still requires the next full verifier. It does not qualify documentation packaging by itself.

Session 88611 subsequently passed that full verification on the captured current source.
The command ran formatting, affected SQLite and Studio Host all-targets/all-features Clippy, their tests, and the unchanged `./scripts/check`.
SQLite passed 106 library tests and two integration tests. Nine library tests were intentionally ignored.
Four ignored sync cases had already passed in the separately admitted trace. The other five fixture or large-profile cases remain separately unqualified.
Studio Host passed 79 library tests. The full Node suite passed 602 tests, and OpenSpec validation passed 17 items.
The log contains 210 successful Rust summaries from repeated invocations; these are not 210 unique test suites or a unique test count.
The former Rustdoc collision warning was absent. This verifies the CLI target configuration in this local documentation run.

Independent review confirmed matching before, after, and current source captures and physical exit zero.
Owned processes and groups were absent. The cgroup was empty, without OOM or PID-limit events.
Observed target growth was 4,567,040 bytes. The growth monitor is not a disk quota.
The review is `.superpowers/rom-010-current-sync-auth-full-check-independent-review-20261009/execution-review.json`.
Its SHA is `24a668f66095623c13ea0bc350dcc684114cddb651b6503e387724e8b5df7e3c`.
This qualifies the local checks, not load, package, deployment, or release acceptance.

### Work utilization diagnostics and a failed full verification, 9 October

The latest chatbox styling requests came from another project window. They do not change the 0.1.0 release objective or its earlier accepted scope.

The current mixed diagnostic already uses native batching. Its 6,447 Work commits comprise 4,577 prefix claims, 1,138 atomic updates, and 732 delivery transitions.
The original evidence does not contain returned-width histograms. These totals do not establish that increasing the batch limit would improve the workload.

The candidate adds fixed diagnostics, without changing scheduling, authorization, durability, or queue limits:

- SQLite records commit origin, actual width, semantic change, native result, and commit wall time in at most 462 aggregate buckets.
- The load host records returned claim widths and requested atomic-update widths in 33 buckets for each boundary.
- Failed prefix calls have no confirmed returned width. They remain separate from successful empty claims.
- Native commit success remains separate from a later acknowledgement with an unknown outcome.

Semantic change compares prepared headers, records, and roots. It does not establish WAL writes or synchronization calls.
The native observer uses the same commit interval as the existing stage timer. The existing 18 stage and 10 publication categories remain unchanged.

Actual RED evidence is retained in `.superpowers/rom-010-work-utilization-actual-red-20261009`.
Five SQLite tests failed because the new observations were absent. The rollback control passed.
Only two initial host tests reached the intended assertion. The other three had fixture errors, which remain recorded as failures.
After fixture correction, all five host tests reached missing-observation assertions in `.superpowers/rom-010-work-utilization-host-red-retry-20261009`.
Both executions completed and physically closed. Independent review did not convert the initial fixture errors into valid RED evidence.

Session 16026 then passed affected formatting and Clippy checks. SQLite passed 115 library tests and two integration tests; nine library tests remained ignored.
The isolated load host passed 46 tests. OpenSpec passed 17 items, and the Node suite passed 602 tests.
The full local verifier nevertheless failed: the AI tools suite passed 84 tests and failed one test.
The failing test was `redb_concurrent_and_duplicate_tool_resume_admits_one_original_receipt_replay`.
At `crates/rom-ai/tests/tools/support.rs:2307`, the actual state was `AwaitingReconciliation`; the expected state was `Completed`.

The failed full verification remains in `.superpowers/rom-010-work-utilization-green-full-prep-20261009`.
Independent review confirmed unchanged before and after source captures, physical exit 101, absent owned processes, and an empty cgroup.
The review SHA is `36ea64b02423ffa209f2c81f292152f7152652ad17b70ba7e3c4778928d9d264`.
Observed target growth was 10,805,248 bytes. The monitor is not a disk quota.
These affected passes do not qualify the full verifier or the release.

Session 36605 subsequently ran five unchanged-source paired SQLite/redb samples. All ten focused test executions passed.
The unchanged 85-test AI tools suite also passed. This reproduction does not erase the earlier failure or establish its cause.
Evidence is retained in `.superpowers/rom-010-ai-tool-resume-failure-reproduction-20261009`.
Its independent review SHA is `ee6a4417b1fd4bebd570873646b2c62a35d7c52ce69e44afbdd9f7d0dbdebe36`.

A source-supported hypothesis concerns duplicate resumes of an existing pending operation.
A duplicate can finish known settlement while the first caller still prepares tool recovery from an earlier revision.
A controlled interleaving on both real adapters must test this hypothesis before a production correction.
Timeouts, authority checks, and the expected final state remain unchanged. Terminal utilization export and unchanged mixed-load acceptance remain outstanding.

The first controlled race fixture did not reach its semantic assertions. The redb test aborted with stack overflow; SQLite did not run.
Evidence is retained in `.superpowers/rom-010-ai-tool-pending-duplicate-actual-red-20261009`.
Cargo returned 101, but this is a fixture failure, not valid semantic RED evidence.
The wrapper completed with exit 0 because its shell checked only Cargo's status. The experiment result requires separate assertion review.
The next fixture correction stores large futures on the heap. It does not increase stack sizes or deadlines.

The corrected fixture reproduced the intended race on SQLite and redb. Both original callers returned `Conflict`; both duplicates returned success.
Neither flow reached `Completed`. The original Task remained at revision 2, without another provider lookup or attempt.
Both tests failed at the intended original-recovery assertion. Later completion and receipt assertions were not reached.
Evidence is retained in `.superpowers/rom-010-ai-tool-pending-duplicate-boxed-red-20261009`.
Independent review confirmed all 4,500 source pins and physical closure. Its SHA is `33d185db99b27f74a877e68a53f56066305208b5dc59207b2101ab75bc116ed6`.

The candidate correction preserves authorized financial settlement for a pending duplicate.
It does not finish that operation while its final tool turn remains incomplete. This preserves AiRun's revision for the original recovery.
The candidate requires GREEN tests and full verification. It does not claim protection against every other classified-snapshot interleaving.

The first candidate full verification also failed. The shared test helper introduced another inline large future in the original recovery wrapper.
`redb_current_tool_grant_revocation_blocks_recovery_without_provider_lookup` aborted with stack overflow before its semantic assertions.
Affected SQLite tests passed 115 library cases and two integration cases; nine library cases remained ignored. The load host passed 46 tests.
AI all-feature Clippy and 602 Node tests passed. These passes do not qualify the failed full verifier.
Evidence is retained in `.superpowers/rom-010-ai-tool-pending-duplicate-green-full-20261009`.
Independent review confirmed unchanged source pins and physical exit 101 with an empty cgroup.
Its SHA is `7a506d659061c5cc4e69e51902a018ab6fa5638bf6bac527e4e8a6996a7a6d72`.
The next correction boxes the shared test wrapper's future without changing production behavior, stack limits, deadlines, or assertions.

The corrected candidate passed both controlled regressions and the full local verifier.
Both original recoveries succeeded, and both flows reached `Completed`. Task revision remained 2, with zero provider lookups and two intended attempts.
The full AI tools suite passed 87 tests. The combined run also passed affected formatting and Clippy checks, 602 Node tests, and 17 OpenSpec items.
SQLite passed 115 library tests and two integration tests; nine library tests remained ignored. The load host passed 46 tests.
The log contains 213 successful Rust result summaries. Repeated suites prevent treating that number as a unique test count.

Evidence is retained in `.superpowers/rom-010-ai-tool-pending-duplicate-green-retry-20261009`.
Independent review confirmed all 4,500 current source pins, relevant cache/tool/runtime pins, physical exit 0, and an empty owned cgroup.
No OOM, PID-limit event, or documentation collision warning was observed. Maximum monitored target growth was 1,363,968 bytes.
The review SHA is `1365707af6c7dfe4f73abf7927e962c7163926063dc27f5d6f6d00d3c7a13d9b`.
Previous failures remain preserved. This result does not qualify mixed-load acceptance, release artifacts, or publication.
The next experiment exports bounded terminal Work diagnostics without changing workload criteria or native database guarantees.

Two real Host lifecycle tests subsequently failed at missing terminal Work exports. The Host and worker had closed before those assertions.
The missing files were `load-final-work-utilization.json` and `load-final-work-batches.json`.
Both failures were intended RED evidence: zero passed, two failed, and 46 filtered. Cargo returned 101; the enclosing RED wrapper returned 0.
Evidence is retained in `.superpowers/rom-010-work-terminal-export-actual-red-20261009`.
Independent review confirmed all 4,501 current source pins, physical closure, and no OOM or PID-limit event.
Its SHA is `c521a562005b62bf2baab797b2bc41abc3187eb1b62b9786810834050394a972`.

The candidate writer uses separate modules for native cells, Host boundary cells, and terminal lifecycle composition.
Both bounded files are attempted before either write result is returned. Existing lifecycle, authentication, core, and stage error priority remains unchanged.
New u64 totals use decimal strings. Missing Host snapshots remain explicitly unavailable; they do not become clean empty observations.
The writer still requires affected tests, full verification, and independent review. No current mixed-load acceptance is claimed.

The first writer candidate failed Clippy because of an unused production import. No new Host test or full verifier ran afterward.
A later candidate passed 56 Host tests but failed one reopen fixture. Its default database limits differed from the persisted limits.
The fixture now reopens through the configured `Database` adapter. Its exact ledger comparison remains unchanged.
Both failed executions and their database evidence remain preserved. A separate prepared candidate was never launched.

The corrected writer passed 57 Host tests, including 11 terminal Work tests. The full local verifier also passed.
SQLite passed 115 library tests and two integration tests; nine library tests remained ignored. The AI tools suite passed 87 tests.
The private collector passed 21 controls. These controls do not cover a blocking FIFO open.
Evidence is retained in `.superpowers/rom-010-work-terminal-export-green-retry3-20261009`.
Independent review confirmed 4,505 source pins, relevant tool and cache pins, physical exit 0, and empty owned cgroups.
Its SHA is `56749adef50fa30fe19ceb3ef698dad79fff48c8847e85ca30d3c1b9d7259160`.
Observed target growth was zero bytes. This monitor is not a disk quota.

Source review found a separate collector gap: a read-only FIFO open can block before the descriptor type check.
The next regression tests static FIFO inputs, replacement before open, and the parent projection reader.
Historical collector assets remain unchanged. A fresh derivative will test nonblocking open with the existing identity and content guards.
Regular-file reads still need an outer execution deadline. No mixed-load acceptance, artifact qualification, or release publication follows from these results.

The current terminal diagnostic build and all 70 mixed harness controls passed independent review.
The actual mixed run failed unchanged scheduling, HTTP completion, and durable Work criteria. All diagnostic receipts and owned cleanup remained available.
Idle commits cost only 10.670365 ms in total. Batch barriers and repeated query authority reads are the next investigation targets.
The [terminal diagnostic report](rom-0.1.0-work-terminal-diagnostic-2026-10-09.md) records exact source identity, results, limits, and evidence.
No mixed acceptance, selected native-engine qualification, final artifact qualification, or release publication follows from this result.
