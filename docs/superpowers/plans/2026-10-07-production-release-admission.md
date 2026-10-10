# Production Release Admission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bind public consumer acceptance to verified 0.1.0 source artifacts without changing historical artifact contracts.

**Architecture:** Add manifest version 3 and the explicit `rom-public-consumers-v3` verification profile. Preserve the existing eight commands and append three extracted-source consumer gates. Keep the R1–R14 release decision separate from the profile's narrower consumer evidence.

**Tech Stack:** Existing Linux Node tooling, npm lockfile version 3, Playwright Chromium/WebKit, Rust Cargo, SQLite, and redb.

**Spec:** [Consumer release requirements](../../research/rom-0.1.0-consumer-research-plan.md), especially lines 54–70 and 150–159. This document proposes integration; it does not amend an accepted release specification.

Date: 2026-10-07. Status: partial candidate implementation. The candidate ledger below records execution and remaining acceptance.

## Global constraints

- Preserve `native-source-v1`, manifest version 1, source-only distribution, and the original six gates.
- Preserve `rom-studio-v2`, manifest version 2, source-and-Studio-assets distribution, and its exact eight gates.
- Do not reinterpret a historical artifact as proof of newly introduced consumer behavior.
- Keep `publication_enabled: false`. This plan does not authorize publication or deployment.
- Require one matching Rust/Studio version and a clean, committed source set for release production.
- Preserve failed stages, historical artifacts, consumer logs, and prior browser runtime installations.
- Admission forbids dependency copying, private source aliases, checkout links, vendor patches, and lock generation.
- A supplied minimal Studio candidate archive is authoring evidence. It is not the independently verified complete ROM release archive.
- Do not use the profile name as a production-readiness claim.

## Source findings and module boundaries

These references describe the inspected implementation, before this proposed integration.

| Source reference | Existing contract and integration seam |
| --- | --- |
| `scripts/release-artifacts/commands.mjs:6–35` | Explicit native and Studio profiles; six native commands plus two Studio commands. `verifyCommands` currently selects first seven or last one implicitly. Add explicit phases without changing the default historical contract. |
| `scripts/release-artifacts/produce.mjs:15–43` | Clean snapshot, first seven gates, asset preparation, eighth gate, source archive, extraction verification, manifest, independent verification, final fence, exclusive publication. Insert consumer execution after verified source extraction and before finalization. |
| `scripts/release-artifacts/verification.mjs:14–40` | Complete source inventory, modes, reconstructed Git tree, revision, selected identity, Cargo lock, skills, and Studio source are verified. Scratch extraction is deleted on return. Consumers need a retained, verified extraction lease. |
| `scripts/release-artifacts/verification.mjs:42–66` | Versions 1 and 2 have separate artifact contracts. Verification checks complete checksums and re-verifies extraction. It does not rerun browser behavior. Keep that property explicit. |
| `scripts/release-artifacts/manifest.mjs:8–13` | Version 2 and Studio profile are currently hardcoded. Add a versioned constructor with strict profile-specific validation. |
| `scripts/release-artifacts/source.mjs:19–47` | Full tracked source snapshot and clean-source fingerprint fence. Retain fences throughout the new sequence. |
| `scripts/packages/studio-source.mjs:22–36` | Full Studio input fingerprint includes fixture files and lockfiles, not only exported source. Reuse this identity. |
| `scripts/release-artifacts/command-result.mjs:7–25` | One-hour command timeout, 32MiB combined output, immutable logs, and Cargo jobs 2. Add explicit consumer limits through the shared supervisor. |
| `scripts/skills/process.mjs:17–57` | Linux process-group ownership, output abort, TERM/KILL escalation, and descendant cleanup. Reuse this contract rather than another subprocess abstraction. |
| `studio/tests/public-controls/verify.mjs:24–99` | Exclusive output, explicit source selector, frozen local dependency metadata, npm ci, physical installation, source match, type check, build, and optional browser report admission. Invoke the verifier from the verified extraction. |
| `studio/tests/public-controls/verification.mjs:57–66` | Admission requires supplied source, locked npm ci, matching installed source, and actual passing results from both engines. These checks establish a narrower package consumer contract. |
| `studio/tests/public-controls/consumer/tests/playwright.config.ts:4–10` | One worker, strict port 43281, no server reuse, Chromium, and WebKit only when explicitly configured. Missing WebKit must fail admission. |
| `scripts/check:5–7` | Full local verification includes public-controls support unit tests. It does not execute the installed external browser consumer. Add a separate explicit consumer-check command. |
| `docs/superpowers/plans/2026-10-07-mutation-recovery.md:118–129` | Forthcoming external client/recovery proof and real adapter host. Its proposed CLI lacks the source selector and admission contract required here. Coordinate that change with its owner before implementation. |

Keep implementation in named modules. Keep public CLI and JavaScript exports stable. Share source admission, lock validation, result validation, and process supervision where guarantees match. Do not combine pure artifact validation with behavior execution.

## Selected profile and command contract

Add `PUBLIC_CONSUMERS_PROFILE = 'rom-public-consumers-v3'`. Version 3 retains the same three payload artifacts and `source-and-studio-assets` distribution. It adds checksummed consumer evidence and a requirement ledger, not a fourth payload type.

The profile requires exactly eleven commands. Commands 1–8 retain their existing order, arguments, checkout working directory, and meaning. Commands 9–11 use the same recorded working directory. Their verifier programs come from the verified extracted source.

Use a strict profile context with absolute normalized execution paths and portable evidence-relative paths. Bind that context to the archived revision, tree, source inventory digest, Studio digest, and source archive SHA-256. Reject unknown or missing context fields. The context describes a completed execution; historical absolute paths need not exist during later static artifact verification.

| Gate | Proposed fixed command after context substitution |
| --- | --- |
| 9 | `node EXTRACTED/studio/tests/public-controls/verify.mjs EVIDENCE/controls --source-dir EXTRACTED/studio --admit` |
| 10 | `node EXTRACTED/studio/tests/mutation-recovery/verify.mjs EVIDENCE/recovery-sqlite --source-dir EXTRACTED/studio --adapter sqlite --host-binary HOST --port 43282 --admit` |
| 11 | `node EXTRACTED/studio/tests/mutation-recovery/verify.mjs EVIDENCE/recovery-redb --source-dir EXTRACTED/studio --adapter redb --host-binary HOST --port 43283 --admit` |

`ROM_PUBLIC_CONTROLS_BROWSER=1` is mandatory for gate 9. All browser consumers require both admitted browser executables. Serialize gates; acquire ports exclusively and reject existing listeners. Record effective environment selections without credentials.

The recovery CLI is a proposed contract, not an existing executable guarantee. Agree its host package and binary name with the recovery owner. Do not freeze the new profile until that host and both adapter scenarios exist. A controls-only interim command can produce candidate evidence, but must not emit this profile.

Build HOST once from the verified full source extraction. Use its declared fixture package with `cargo build --locked`, jobs 2, and a separate evidence-local target directory. Record the exact command, binary SHA-256, compiler, Cargo lock hash, and extracted source identity. A prebuilt host supplied from a different checkout is not admissible. Host preparation is a mandatory recorded consumer preparation step, not a twelfth top-level gate.

Recovery evidence must distinguish real state/event/receipt observations from mocked responses. It must cover lost acknowledgement, process restart, replay, changed input, stale revision, cancellation, logout, and revoked authority on each actual adapter. Browser success alone does not establish those backend invariants.

## Verified extraction and fingerprint fences

Introduce `withVerifiedExtraction(directory, source, artifacts, frontend, callback)` in a focused release module. It verifies the current extraction contract before calling the callback. Preserve `verifyExtraction` as a compatibility wrapper that returns the same old record.

Supply `{ root, identity }` only after revision, full file inventory, executable modes, tree, locks, skills, and Studio checks succeed. Keep this witness separate from the existing `archive_verification` record so versions 1 and 2 remain byte-shape compatible.

The callback may create consumer copies, binaries, and databases outside the extracted input tree. It must not install dependencies or write generated files into the verified extraction. Invoke fixture source and frozen consumer manifests from that extraction, not from the producer checkout.

Use this order:

1. Capture the clean source snapshot and explicit selected profile.
2. Run gates 1–7 and the existing asset preparation and gate 8.
3. Fence the checkout before assembling source and skills archives.
4. Verify the complete archived extraction and record its witness.
5. Prepare the host from extracted source with an external target directory.
6. Run gates 9–11 into separate exclusively created evidence directories.
7. Recheck the extracted full source fingerprint, frozen fixture locks, and source archive digest after each consumer gate.
8. Fence the producer checkout after consumer execution.
9. Validate consumer reports and the requirement ledger before manifest finalization.
10. Finalize checksums and independently validate the complete artifact directory.
11. Fence the checkout immediately before exclusive local publication.

Use temporary extraction only for immutable inputs. Retain all consumer commands, reports, runtime records, and failure summaries outside that temporary tree. Remove only producer-owned temporary inputs through the existing lifecycle. Do not remove recorded failed stages or earlier evidence.

## Candidate locks and version refresh

Keep each consumer package manifest and npm lock in its fixture source directory. Record their raw byte hashes and lockfile version. The production producer must never run `npm install` to create or refresh them.

The checked public-controls graph uses selected Svelte, Vite, Tailwind, type-checking, and Playwright dependencies. It installs the supplied ROM source as a local file dependency. The Studio runtime stylesheet dependencies remain declared by Studio. Do not clone the producer's complete development dependency set.

During a version or dependency change, generate a candidate lock through explicit `--bootstrap-lock` authoring execution. Review package identity, selected dependency versions, integrity entries, license effects, and transitive changes. Commit the refreshed fixture locks with the matching source version before producing artifacts.

Run a separate fresh `npm ci` replay after that review. Label bootstrap evidence `bootstrap_then_npm_ci`; admission requires `locked_npm_ci`. These are separate evidence classes. A consumer's arbitrary new install does not establish a frozen graph.

For release gates, compare each checked lock before and after installation. Compare realized package versions and exact installed ROM source against the verified source copy. Retain npm version, relevant platform, command flags, lock hashes, and install logs. Optional platform packages may be absent only where their lock entries permit it.

Registry fetches during npm ci are permitted under the frozen integrity graph. Record network use and failure. Do not claim an offline proof unless the installation actually used an offline command and complete cache.

## Consumer result and browser admission

Add a strict versioned consumer evidence record. Each record identifies the gate, adapter, archived source witness, fixture file hashes, lock hashes, exact commands, source mode, installed source digest, and evidence paths.

For browser evidence, retain the original machine-readable Playwright report. Validate expected test identities and counts, not only nonempty project arrays. Freeze the current controls scenarios in the profile schema. The expanded fixture has six scenarios per engine, including public client/recovery preparation and desktop/mobile root App composition. Admit every required scenario in both Chromium and WebKit, with no skipped, absent, failed, interrupted, or timed-out case. Unexpected retries and global report errors must remain visible and fail the selected no-retry contract.

The current required controls test titles are:

- `installed public controls preserve bound values, native refs and keyboard operations`
- `installed stylesheet and wrapper contracts support a compact composer at 1280x900`
- `installed stylesheet and wrapper contracts support a compact composer at 390x500`
- `installed public client and recovery entries preserve exact typed mutation draft`
- `installed root App and renderer compose resources settings and work at 1440px`
- `installed root App and renderer compose resources settings and work at 390px`

Require twelve matching executions across the two engines. Review scenario changes explicitly before updating the profile contract. Earlier six- or eight-execution candidate evidence does not satisfy this expanded contract.

Record actual executable paths, versions or admitted runtime revision, executable/runtime digest, and launch arguments. Validate these records against the selected runtime admission before launch. Environment variable presence alone does not establish engine execution. Retain crashed-engine reports and logs.

Public controls evidence covers supported control exports, CSS, focus, sizing, class/ref/value behavior, and required failure scenarios. Recovery evidence also exercises public client and recovery subpaths. The expanded fixture exercises root App and an explicitly registered custom renderer without private aliases. SSR support and broader visual variants remain outside that browser-source proof.

Static `verifyArtifacts` validates complete checksums, source extraction, profile commands, preparation commands, strict consumer records, and original browser reports. It does not silently install packages or execute browser code from an artifact. A separately named replay command creates a fresh output directory and repeats admission from newly verified extraction. Record fresh replay evidence separately from the producer's historical records.

Checksums detect changes relative to the supplied manifest; they are not signatures or independent attestations of execution. Keep this trust boundary explicit.

## Finite execution envelope

Retain the existing one-hour/32MiB limits for gates 1–8. Set a 20-minute wall-clock limit and 32MiB combined output for each consumer gate and host preparation. Use the existing process-group supervisor.

Keep the public consumer's 180000ms child timeout and 8MiB child output ceiling. Bound preview readiness to 20000ms. Use one Playwright worker and no retries. Run no native producer or consumer compilation concurrently. Native compilation uses jobs 2 and disabled incremental compilation for evidence runs.

Nested process ownership requires a specific test. An outer group timeout must also terminate the fixture's npm, host, browser, and preview descendants. Avoid detached inner groups that escape outer ownership. A direct-child timeout alone is insufficient.

Before each gate, record available disk, selected directories, adapter, and engine configuration. Use fresh bounded fixture data, loopback hosts, isolated databases, and reserved ports. Refuse occupied output paths before any write. Do not overwrite a successful result with a later run.

Retain the existing minimal Studio archive bounds: 32MiB compressed, 64MiB decompressed, 20000 members, bounded listings, fixed prefix, and no links. Full release extraction has a different inventory contract; do not apply the Studio-only allowed-member list to it. Add explicit full-source archive byte/member limits after measuring the committed inventory. Reject excessive archives before extraction writes.

A timeout or output overflow leaves incomplete evidence and prevents local publication. These limits bound subprocess work and captured logs. They do not constitute a machine memory quota or an R10 load measurement.

## R1–R14 evidence ledger and release decision

Add `release_requirements` with exactly R1 through R14. Each entry records requirement identity, state, source identity, evidence paths and hashes, measured scope, limitations, and review reference. Candidate, inspected, mocked, or deferred evidence cannot satisfy an executed acceptance requirement.

| Requirements | Required evidence outside profile naming |
| --- | --- |
| R1 | Two real Runtime instances, captured snapshot isolation, compatibility, rejection/panic invariants, and authorized replay on SQLite and redb. |
| R2–R3 | New package consumer gates plus packaging layout and negative license/owner tests. Controls passes do not replace shared-install notice checks. |
| R4 | Public recovery/client consumers with current authority and real backend receipt/state/event evidence on both adapters. |
| R5 | Durable prepare/publish/follow-up restart boundaries, retained input, attempt/budget state, and receiver deduplication limits. |
| R6 | Anonymous projection boundaries, private Resource denial, permission invalidation, and principal-safe conditional caching. |
| R7 | Isolated real-provider restart, key rotation, expiry/logout, denied grants, CSRF, and issuer/audience failures. Mock OIDC tests are insufficient. |
| R8 | Fresh-host application restore covering database, blobs, configuration references, unfinished work, receipts, tombstones, journal fences, and measured RPO/RTO. |
| R9 | Correlated action/work/external-attempt diagnosis, redaction, bounded labels, and exporter-failure isolation. |
| R10 | Measured adapter workload envelope, latency/memory/disk/queue age, disk-full, slow consumers, provider loss, and interrupted recovery. |
| R11 | Populated 0.0.3 upgrade with pending work/custom codecs/receipts, matching package versions, explicit incompatible downgrade behavior, and separate rollback/restore evidence. |
| R12 | Owner-authored feature and a second non-AI consumer; record effort, private imports, patches, duplicate orchestration, and diagnostic clarity. |
| R13–R14 | Provider routing and aggregate budgets; validated tools and durable flows in two unrelated consumers, restart, denied/invalid outcomes, and unknown external effects. |

Profile completion proves its eleven command contracts and bound consumer evidence. The 0.1.0 release decision additionally requires the complete release ledger and documented measured deployment limits. Pending required evidence blocks that decision. An accepted scope exception needs an explicit owner decision and must remain visible; do not infer it from a profile pass.

## Review focus

- Historical manifests must retain exact validation behavior after the new profile is added.
- A fixture invoked from one checkout must not attest a different artifact's fixture or lock.
- Browser configuration must not admit missing engines, changed tests, or report-level failures.
- Successful static validation must not execute untrusted artifact programs.
- Failed gates must retain evidence without orphan processes or contradictory completed records.

## Task 1: Versioned profile and historical compatibility

**Files:** Modify `scripts/release-artifacts/{commands,manifest,verification}.mjs`. Test `commands.test.mjs`, `studio.test.mjs`, and new `profiles.test.mjs` in the same directory.

- [ ] Add failing tests for version 3/profile pairing and exactly eleven ordered commands.
- [ ] Add mutations for missing, extra, reordered, wrong-source, wrong-adapter, and wrong-port consumer commands.
- [ ] Pin historical version 1/six-gate and version 2/eight-gate fixtures unchanged.
- [ ] Add a negative test preventing a version 2 artifact from acquiring version 3 claims by changing its profile field.
- [ ] Implement explicit phase selection and strict context without changing historical defaults.
- [ ] Run `node --test scripts/release-artifacts/commands.test.mjs scripts/release-artifacts/studio.test.mjs scripts/release-artifacts/profiles.test.mjs`.

## Task 2: Verified extraction lease and consumer orchestration

**Files:** Create `scripts/release-artifacts/consumer-source.mjs` and `consumers.mjs`. Modify `produce.mjs` and the focused extraction seam in `verification.mjs`. Test new `consumer-source.test.mjs` and `consumers.test.mjs`.

- [ ] Add failing tests proving no consumer executes before full archive admission.
- [ ] Reject mismatched archive revision/tree, mutated extracted source, checkout-sourced fixture, and changed lock bytes.
- [ ] Test source fences before assembly, after consumers, and immediately before publication.
- [ ] Test source binary provenance and a preparation command that fails or uses a different lock/source.
- [ ] Add immutable input leasing and serialized orchestration using the shared process supervisor.
- [ ] Add timeout/output-overflow tests that spawn descendants and verify process-group termination.
- [ ] Test occupied output/port refusal, immutable evidence, and failure-summary storage errors.
- [ ] Run `node --test scripts/release-artifacts/consumer-source.test.mjs scripts/release-artifacts/consumers.test.mjs`.

## Task 3: Frozen consumer contract and engine evidence

**Files:** Coordinate changes to `studio/tests/public-controls/{verify,verification}.mjs` with its current owner. Coordinate `studio/tests/mutation-recovery/` with the recovery owner. Create `scripts/release-artifacts/consumer-evidence.mjs` and `consumer-evidence.test.mjs`.

- [ ] Add failing tests for npm install/bootstrap/copying admission, source aliases/links, missing lock, changed metadata, and realized dependency drift.
- [ ] Add report mutations for missing engine, skipped/missing test, altered scenario identity, wrong count, global error, retry, and crash.
- [ ] Reject missing/mismatched executable provenance and a forged result inconsistent with its original report.
- [ ] Add the recovery source selector and strict admission contract before profile enablement.
- [ ] Preserve controls-only authoring execution and bootstrap labeling.
- [ ] Run focused fixture support tests and `node --test scripts/release-artifacts/consumer-evidence.test.mjs`.
- [ ] Run fresh candidate locked-ci admission separately for controls and both recovery adapters after review.

## Task 4: Static validation, fresh replay, and release ledger

**Files:** Modify `verification.mjs` and `manifest.mjs`. Create `scripts/release-artifacts/requirements.mjs`, `requirements.test.mjs`, and an explicit replay CLI. Update release-artifact README and the existing CLI facade only after ownership assignment.

- [ ] Add checksum-valid manifest mutations that remove or substitute consumer reports, preparation records, locks, source witnesses, and ledger entries.
- [ ] Reject duplicate/unknown requirements and passing statuses backed only by inspected or candidate evidence.
- [ ] Prove versions 1 and 2 require no new ledger and launch no consumer programs.
- [ ] Prove static version 3 verification launches no npm/browser/host commands.
- [ ] Implement fresh replay with exclusive evidence and verified extraction input.
- [ ] Require explicit release-decision evidence independently of profile completion.
- [ ] Run all `scripts/release-artifacts/*.test.mjs` and affected package/fixture support tests.

## Task 5: Integration and acceptance order

**Files:** Add an explicit `scripts/check-public-consumers` command. Modify `scripts/check` only to include new support unit tests, not hidden network/browser deployment work. Update operator/release documentation after writing-rule review.

- [ ] Run affected fast unit tests before any expensive producer run.
- [ ] Run the complete local verifier after source ownership is frozen.
- [ ] Refresh candidate locks for the final matching 0.1.0 version and review their changes.
- [ ] Commit the reviewed source and locks through the root coordinator's authorized integration process.
- [ ] Run a clean-source version 3 producer with exclusive evidence, admitted engines, and isolated databases.
- [ ] Run independent static artifact verification from the resulting output.
- [ ] Run fresh extracted-source consumer replay into a different evidence directory.
- [ ] Complete the separate R1–R14 requirement review and measured deployment documentation.
- [ ] Report exact source/archive/lock identities, commands, limitations, and unresolved requirements before any release decision.

Execution requires coordination before edits to the shared release modules, fixture source, CLI, or manifests. Initial design ownership covered this plan alone.

## Candidate implementation ledger

The root coordinator now owns the release-admission modules and their tests.
The historical producer still emits version 2. Version 3 artifact admission remains disabled until its complete evidence contract is implemented.

Task 1 has an eleven-command candidate contract and strict consumer path/source context.
Explicit prepare, asset, and consumer phases preserve the original eight gate identities.
Versioned manifest integration and strict consumer evidence validation remain open. Do not mark Task 1 complete from command tests alone.

Task 2 has a verified extraction lease. Consumer callbacks run only after source inventory, modes, tree, profile, skills, and Studio verification.
The lease requires declared payload digests. It remains available through asynchronous execution, then checks source and archive fingerprints before disposal.
Historical verifyExtraction returns its original evidence shape. Producer orchestration, host preparation, process limits, and complete source admission remain open.

The command-contract RED result contains four failures for the unsupported new profile.
The subsequent digest RED result contains four passes and one failure because consumer execution accepted a missing payload digest.
The complete artifact test suite now passes 42 tests in rom-dev. These are isolated artifact fixtures, not real production release gates.
Evidence is retained in `/var/tmp/rom-010-release-lease-final.log` and the preceding RED and corrective logs.

The broad test run also found a stale negative test: its format mutation assigned 9, which is now the candidate format.
The negative case now changes the current value. It still requires rejection of a checksum-valid manifest with the wrong source profile.
The initial host run lacked rustc; the retained log records an environment failure, not a product regression.

No clean release producer, complete version 3 manifest, independent artifact replay, deployment, or release publication has run for this candidate.

The strict browser-report checker now binds six control scenarios and five recovery scenarios to both engine projects.
It rejects missing, extra, renamed, wrong-file, skipped, failed, interrupted, retried, and duplicate results.
The recovery RED retained three passing control tests and three failures for the missing recovery interface.
The report checker accepts both existing ten-case recovery reports only as authoring-candidate browser evidence.
It does not establish native provenance, correct server effects, or clean release source.

An explicit native execution test exposed omission of the sixth native gate under the new profile option.
The correction executes all six native gates without changing the historical Studio phase behavior.
All 49 artifact module tests pass in rom-dev. Evidence is `/var/tmp/rom-010-release-report-and-native-final.log`.
The corresponding RED logs are `/var/tmp/rom-010-recovery-report-red.log` and `/var/tmp/rom-010-native-profile-phase-red.log`.

Source inspection found that the recovery verifier records HEAD through git in its own source root.
A verified extraction has no `.git` directory. Host preparation and consumer evidence must use the verified source witness instead.
Do not enable version 3 admission until that source-bound execution and complete native provenance are implemented and tested.

Task 3 now validates the six frozen control scenarios in both report projects.
It rejects missing, duplicate, renamed, wrong-file, skipped, failed, interrupted, retried, and globally erroneous reports.
It also rejects malformed report collections. The original twelve-execution checkout-candidate report passes this narrow validation.
Executable provenance, strict recovery scenario binding, complete consumer records, and extracted-source producer integration remain open.

The expanded artifact test suite passes 45 tests. Its terminal log is `/var/tmp/rom-010-release-report-final.log`.
Browser-report RED logs remain separate from artifact-fixture and runtime browser evidence.

## Native preparation candidate

The recovery source-metadata helper now reports null checkout HEAD for packaged source without Git metadata.
Three tests cover real checkout HEAD, extracted source without Git metadata, and a nested directory in an unrelated checkout.
The logs are `/var/tmp/rom-010-recovery-source-metadata-red.log` and `/var/tmp/rom-010-recovery-source-metadata-green.log`.
A null diagnostic HEAD is not archive provenance. The verified extraction witness must still bind release identity.

The internal `native-preparation.mjs` helper now accepts a verified lease and a fresh external evidence directory.
It fences source before and after execution. It checks Cargo.lock against the witness at both boundaries.
It records rustc, Cargo, and `cargo build --locked -p rom-recovery-host` through the shared process supervisor.
The target directory is external, with two build jobs and incremental compilation disabled.
Each command has a one-hour timeout and a 32 MiB combined output limit. These limits do not bound build-cache disk growth.
An actual producer must obtain a separate finite disk allocation before compilation.
The helper rejects a symlink, hard-linked, empty, or nonexecutable host binary before writing successful provenance.
Failure logs and failure metadata remain in the fresh evidence directory.

Six helper tests pass. Their runner writes fixture executables; they do not prove compilation or runtime behavior.
The missing-interface RED and lock-mutation RED remain in `/var/tmp/rom-010-native-preparation-contract-red.log` and `/var/tmp/rom-010-native-preparation-lock-red.log`.
The final helper log is `/var/tmp/rom-010-native-preparation-final-green.log`.
All 55 artifact module tests pass in rom-dev at `/var/tmp/rom-010-native-preparation-artifacts-green.log`.

The helper is not yet called by the producer. Version 3 admission remains disabled.
Static preparation-record verification, complete consumer provenance, disk admission, orchestration, and independent replay remain open.
The most recent complete verifier predates this helper and Task 3 AI source. A fresh full verifier is required before integration.

A seventh native-preparation regression checks failure metadata refusal.
The helper reuses the existing fallback diagnostic and preserves the original command failure when failure.json cannot be written.
Its initial RED and correction logs are `/var/tmp/rom-010-native-preparation-evidence-red.log` and `/var/tmp/rom-010-native-preparation-evidence-green.log`.
The first corrected fixture expected EISDIR, but exclusive file creation correctly returned EEXIST. That fixture expectation was corrected separately.
All 56 artifact module tests then passed in `/var/tmp/rom-010-native-preparation-artifacts-corrected-final.log`.
These fixture tests still do not establish actual native compilation or complete producer admission.

Static native preparation validation now checks the complete source witness, exact commands, lock, target, binary digest, and six retained log digests.
Four initial validation tests and seven preparation tests passed in `/var/tmp/rom-010-native-evidence-green.log`.
The complete artifact suite then passed 60 tests in `/var/tmp/rom-010-artifacts-native-evidence-root.log`.
The static validator does not establish source trust or execute a compiler or consumer.
Producer integration and complete provenance admission remain open.

An additional regression reproduced acceptance of a command record above the one-hour limit.
It also checks calendar timestamps and acceptance at the exact one-hour boundary.
Execution and evidence validation now use shared timeout and log-size constants.
The initial RED is `/var/tmp/rom-010-native-evidence-time-red.log`.
The corrected 12 focused tests passed in `/var/tmp/rom-010-native-evidence-time-green.log`.
These checks do not bound actual build-cache growth. Finite disk admission remains mandatory before a producer run.
The complete artifact suite passed 61 tests in `/var/tmp/rom-010-artifacts-native-time-final.log` after this correction.
The full current verifier and release producer have not been run for this source state.

### Requirement ledger validation contract

The candidate ledger has `schema_version`, `source_identity`, and `requirements` only.
Its requirements contain exactly R1 through R14. Each entry has `id`, `state`, `source_identity`, `evidence`, `measured_scope`, `limitations`, and `review`.
States are `pending`, `failed`, or `accepted`. Pending and failed entries remain valid diagnostic records but block release completion.
Evidence references contain `path`, `sha256`, `bytes`, and `kind`. Kinds distinguish `executed`, `inspected`, `candidate`, and `review` evidence.
An accepted entry needs executed evidence, a separate review reference, and a nonempty measured scope.
Inspected or candidate evidence cannot satisfy an accepted entry. All references bind retained bytes and the expected source identity.
Each entry has at most 32 evidence references. Each referenced file has a 32 MiB bound.
Scope and limitations each have a 16 KiB text bound. Paths are relative and cannot traverse links.
Duplicate references, hard-linked files, digest mismatches, unknown fields, and substituted source identities fail validation.
The static validator executes no artifact program. Metadata consistency alone does not prove that a report covers its requirement.
The independent R1–R14 completion audit must inspect actual evidence scope before acceptance is recorded.
This validator remains separate from consumer profile completion and version 3 producer admission.

The candidate `requirements.mjs` validator implements this metadata and retained-byte contract.
Five tests cover all fourteen entries, pending and failed admission, substitutions, links, and changed report bytes.
The intended missing-module RED is `/var/tmp/rom-010-requirements-contract-red.log`.
The focused GREEN is `/var/tmp/rom-010-requirements-first-green.log`.
All 66 artifact tests passed in `/var/tmp/rom-010-requirements-artifacts-green.log`.
The accepted records in these tests are synthetic fixtures, not ROM release acceptance.
No real R1–R14 ledger has been admitted. Manifest integration, independent replay, and completion audit remain open.

Common release command execution now shares the native preparation timeout and output bounds through `command-limits.mjs`.
The preceding 61-test suite passed after that refactor in `/var/tmp/rom-010-release-shared-command-limits.log`.
No producer or native compiler was executed by these module tests.

### Serialized consumer execution candidate

`consumers.mjs` adds the internal `runConsumerGates` seam after the eight Studio profile gates.
It requires a matching extraction witness and a validated native preparation record before any consumer command.
The binary digest, retained compiler logs, provenance bytes, and complete source fence are checked before and after each command.
The fixed commands execute in controls, SQLite recovery, and redb recovery order.
Exclusive output creation prevents reuse of earlier consumer results. A failed command stops subsequent execution and retains its command logs.
Canonical output parents reject symlink paths into protected source. Consumer log staging cannot occur inside extracted or producer source.

The missing-module RED is `/var/tmp/rom-010-consumer-orchestration-red.log`.
A separate regression reproduced a linked output parent being accepted: `/var/tmp/rom-010-consumer-output-links-red.log`.
Five focused tests then passed in `/var/tmp/rom-010-consumer-orchestration-output-green.log`.
All 71 artifact module tests passed in `/var/tmp/rom-010-consumer-orchestration-artifacts-green.log`.
These tests use simulated compiler and consumer commands. They prove helper sequencing and rejection paths, not actual browser or native acceptance.

The producer does not yet invoke this seam. Manifest version 3, semantic consumer report binding, finite disk admission, and fresh replay remain open.
The full local verifier has not run for this candidate state. No release publication or deployment occurred.

A further regression reproduced a source-fence failure being mislabeled as a subprocess spawn failure.
Shared gate execution now runs source fences outside command result recording.
The successful subprocess result remains intact when subsequent source admission fails; a separate failure summary blocks further execution.
Evidence: `/var/tmp/rom-010-consumer-fence-result-red.log` and `/var/tmp/rom-010-consumer-fence-result-green.log`.
Failure-summary storage errors retain earlier evidence and emit a bounded fallback diagnostic without replacing the original gate failure.
All 72 artifact tests passed in `/var/tmp/rom-010-consumer-fence-artifacts-final.log` after this change.

A separate regression reproduced command logs being written through a linked staging directory.
The helper now rejects a linked command log directory before any consumer executes.
The initial RED is `/var/tmp/rom-010-consumer-linked-logs-red.log`.
All 73 artifact tests passed in `/var/tmp/rom-010-consumer-orchestration-final-artifacts.log`.
Seven orchestration tests use simulated subprocesses; no actual compiler, consumer browser, complete producer, or publication was run by this suite.

### Consumer deadline correction

The earlier helper used the historical one-hour command bound. It did not meet the planned consumer envelope.
Three intended regression failures are retained in `/var/tmp/rom-010-consumer-time-budget-red.log`.
They cover preparation command limits, the complete preparation duration, and consumer command limits.

Preparation now has one monotonic twenty-minute deadline across all three commands.
Each command receives only the remaining duration. An exhausted deadline prevents the next subprocess from starting.
The failure record preserves completed command results and the bounded-abort diagnostic.
Static preparation validation rejects a complete record that spans more than twenty minutes.
Each consumer gate now receives a twenty-minute timeout through the shared process supervisor.
Historical gates 1–8 retain their existing one-hour bound. The command output bound remains 32 MiB.

The complete artifact module suite passed 74 tests in the development container.
Evidence: `/var/tmp/rom-010-consumer-time-budget-container-artifacts-green.log`.
A host attempt failed 21 tests because `rustc` was absent from PATH; that log is retained separately.
Evidence: `/var/tmp/rom-010-consumer-time-budget-artifacts-green.log`.
These are module tests with simulated compiler and consumer commands. They do not prove complete producer admission or descendant shutdown.
No native compilation, installed browser consumer, or release publication occurred in this increment.

### Package identity comparison

`consumer-package.mjs` compares a report with independently supplied package expectations.
The comparison binds the source input, complete package file inventory, source lock, consumer lock, installed path, and minimal archive digest.
Controls require locked installation and declared browser execution. Recovery records also require the selected adapter and no unrelated checkout head.
The helper rejects changed files, missing notices, extra files, substituted locks, and authoring-checkout reports.
Malformed expectations cannot reduce the comparison requirements.

The missing-module RED is retained in `/var/tmp/rom-010-consumer-package-binding-red.log`.
Three focused tests passed in `/var/tmp/rom-010-consumer-package-binding-green.log`.
The full artifact module suite passed 77 cases in the development container.
Evidence: `/var/tmp/rom-010-consumer-package-artifacts-green.log`.

This helper does not establish trusted source expectations. It does not read the installed package or execute browsers.
The producer must derive expectations from the verified extraction and retained bytes before using this comparison.
Complete browser reports, native preparation, fixture identity, and immutable evidence still require their separate checks.
The helper is not yet connected to complete v3 producer admission. No release was published by these tests.

### Retained package bytes

`consumer-files.mjs` derives package expectations from a verified extraction lease.
It reads the installed package and compares every source file and both required notice files.
It compares the retained consumer lock with the frozen fixture lock in the extraction.
It then binds the report to those files, the source lock, installed path, and retained minimal archive digest.
The lease fence runs before and after the read.

Four focused tests passed in `/var/tmp/rom-010-consumer-files-green.log`.
The missing-module RED is retained in `/var/tmp/rom-010-consumer-files-red.log`.
The tests reject rehashed changed source, changed locks, extra files, changed archives, symlinks, hardlinks, and final source drift.
The complete artifact module suite passed 81 cases in the development container.
Evidence: `/var/tmp/rom-010-consumer-files-artifacts-green.log`.

The read bounds are 64 MiB per file, 256 MiB per source inventory, 100000 visited entries, and depth 32.
The helper reads no package programs. It does not certify browser execution or native compilation.
It compares the minimal archive digest; it does not inspect that archive's members in this increment.
Fixtures use representative archive bytes, not an executed tar archive or installed browser.
Complete admission must combine member validation, browser evidence, native provenance, and fixture identity.
The producer integration remains open. No release was published by these tests.

### Consumer report checks in execution

`runConsumerGates` now checks retained consumer evidence after the three fixed commands complete.
A successful subprocess status without retained reports is an admission failure.
The comparison reads package files and frozen locks through the live verified extraction lease.
Strict browser checks require all controls and recovery scenarios in Chromium and WebKit.
Each recovery report must name the prepared host binary, its digest, and the same native compile lock.
The retained native provenance copy must match the already validated preparation bytes.
A final source and native fence runs after these checks.

The absent-report RED is retained in `/var/tmp/rom-010-consumer-integrated-reports-red.log`.
The native-substitution RED is retained in `/var/tmp/rom-010-consumer-recovery-host-binding-red.log`.
Negative scenarios also reject wrong browser files and rewritten installed-source hashes.
Successful subprocess logs remain recorded when report admission fails.
The shared simulated package fixture prevents separate tests from maintaining different package layouts.

The complete artifact module suite passed 84 tests in the development container.
Evidence: `/var/tmp/rom-010-consumer-integrated-host-artifacts-green.log`.
The compiler, consumer commands, and browser reports in these tests are simulated.
This increment does not prove actual installed consumer execution or original application acceptance.
Minimal archive member validation and complete native/consumer fixture identity still require integration.
The v3 production entrypoint and final requirement ledger remain open. No release was published.

### Minimal archive members

The shared Studio archive admission now lives in `studio-source-archive.mjs`.
The existing consumer import remains a compatibility facade with unchanged exported names.
The checker reuses the bounded listing and extraction rules before inspecting package members.
It compares archived source and notices with the verified extraction. It compares the archived source lock separately.
The archive digest is checked again after member validation.
Only the checker-owned temporary extraction is removed. Retained consumer artifacts and failure evidence remain unchanged.

A rewritten archive digest previously concealed changed archive members.
That intended failure is retained in `/var/tmp/rom-010-consumer-archive-members-red.log`.
The correction passed seven focused file and archive admission tests.
Evidence: `/var/tmp/rom-010-consumer-archive-members-green.log`.
The complete artifact module suite passed 85 tests in the development container.
Evidence: `/var/tmp/rom-010-consumer-archive-members-artifacts-green.log`.

These tests now use real generated tar archives and fixed tar/gzip inspection tools.
Compiler, provider, and browser execution remain simulated in the orchestration fixtures.
This correction completes minimal archive member comparison, not complete release admission.
Native/consumer fixture identity, complete v3 production integration, and requirement acceptance remain open.

### Recovery fixture identity

`consumer-fixture-evidence.mjs` compares recovery fixture inputs with the verified source lease.
It binds native host sources, the verifier, consumer source and tests, manifests, and proxy/runtime scripts.
The retained output inventory must include generated browser and HTTP bundles with matching byte digests.
Rewriting a report cannot conceal substituted input files or runtime scripts.
The lease fence runs before and after the comparison.

Four focused tests passed in `/var/tmp/rom-010-consumer-fixture-identity-corrected-green.log`.
The missing-module RED is retained in `/var/tmp/rom-010-consumer-fixture-identity-red.log`.
An initial positive fixture wrote different input bytes instead of copying them; its failure is retained separately.
Evidence: `/var/tmp/rom-010-consumer-fixture-identity-green.log`.
The corrected fixture copies the intended bytes before comparison.
The complete artifact module suite passed 89 cases in the development container.
Evidence: `/var/tmp/rom-010-consumer-fixture-identity-artifacts-green.log`.

Generated output digest consistency does not prove compilation. Recorded execution and independently tested source remain necessary.
The helper does not execute a fixture or inspect private authentication data.
It is not yet connected to the complete producer path. Complete release admission remains open.

### Connected recovery fixture identity

The consumer report gate now invokes the recovery fixture identity check for SQLite and redb.
A new regression rejected missing native fixture identity after three successful simulated consumer command exits.
Its intended RED is `/var/tmp/rom-010-consumer-fixture-integrated-red.log`.
The shared fixture helper avoids duplicate input/output inventory setup in these tests.
Fifteen focused cases passed in `/var/tmp/rom-010-consumer-fixture-integrated-dry-green.log`.
The development-container artifact suite passed 90 cases before that test-helper refactor.
Evidence: `/var/tmp/rom-010-consumer-fixture-integrated-artifacts-green.log`.
These orchestration tests use simulated compiler and browser results. They do not establish consumer runtime acceptance.
Full v3 producer integration, HTTP result validation, and requirement acceptance remain open.

### Connected retained HTTP recovery assertions

Recovery admission now compares the retained HTTP report with the result record.
The report must identify the selected adapter and the documented single-process file-store durability limit.
Checks cover unchanged commit counts on replay, original receipt identities, tombstones, and exact large-integer request bytes.
The original expected revision remains visible in the accepted and replayed wire body.
A report cannot turn an unknown acknowledgement into evidence of an additional commit.

The missing-module RED is `/var/tmp/rom-010-consumer-http-evidence-red.log`.
The integrated RED accepted omitted HTTP evidence after successful simulated commands.
Evidence: `/var/tmp/rom-010-consumer-http-integrated-red.log`.
Eighteen focused tests passed in `/var/tmp/rom-010-consumer-http-integrated-green.log`.
Representative report fixtures do not execute HTTP or certify runtime provenance.
Actual HTTP execution remains part of the fixed installed-consumer commands.
Complete producer integration, recorded command validation, and final R1–R14 acceptance remain open.

The complete artifact module suite passed 93 cases after HTTP evidence integration.
Evidence: `/var/tmp/rom-010-consumer-http-integrated-artifacts-green.log`.
This includes the shared fixture refactor. No complete producer execution or release publication occurred in this increment.
