# ROM 0.1.0: research plan from production consumer feedback

Date: 2026-10-07. Status: proposed research and acceptance plan; no new experiments have run for this plan.

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
