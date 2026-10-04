# ROM 0.0.2 plan acceptance map

Date: 2026-10-04 UTC. Inspected worktree HEAD: `cc05983b506873c5d655254477a63bbe946f77e4`.
The ordinary checkout remains at `91954b66a1d2808a8747ec79700e3c41196daf21`.

This report maps all 51 checkboxes in the [Studio plan](../superpowers/plans/2026-10-03-rom-0.0.2-studio.md).
IDs identify the task number and checkbox position. The coordinator owns checkbox updates.
This was source and retained-evidence inspection. No build, browser run, deployment, or producer ran for this report.

| Status | Meaning |
| --- | --- |
| Complete | The named implementation and recorded focused evidence satisfy the item. This does not accept the final release. |
| Partial | Some required behavior has evidence. The row identifies the remaining scenario. |
| Missing evidence | No executed evidence for the named requirement was found in the inspected source and maintained records. |
| Pending producer | The fixed gate exists. Final clean-source execution or artifact acceptance remains pending. |
| Pending preview | Deployment or external access acceptance remains pending. |
| Pending integration | Final accepted source has not been integrated and pushed to main. |

New multi-tab and table measurement tests were being added during inspection.
Their status below describes the retained results at this cutoff, rather than an assumed future result.
Earlier reports retain their original scope and counts; later evidence supersedes only the corresponding acceptance boundary.
The coordinator also added a post-adoption wire benchmark while this report was prepared.

## Evidence index

Paths in the tables are relative to the repository root. Each evidence key links a maintained report with raw logs and source identity.

| Key | Source and executed evidence |
| --- | --- |
| M | [Metadata contract](rom-0.0.2-client-contract.md); `crates/rom/src/resource/{input_descriptor,input_descriptor_tests,fields,definition,schema}.rs`, `crates/rom/src/discovery.rs`, `crates/rom-derive/src/expansion.rs`, `tests/persistence/tests/studio_discovery.rs`. [Affected crate suites](evidence/rom-0.0.2/task1/affected-final.log), [actual HTTP fixtures](evidence/rom-0.0.2/task1/http-wire-fixtures.log), [compile-lock correction](rom-0.0.2-compile-lock-results.md). |
| C | [Client results](rom-0.0.2-client-results.md), [SDK corrections](rom-0.0.2-sdk-resilience-results.md), [anchor review](rom-0.0.2-sdk-review.md), [blob review](rom-0.0.2-sdk-blob-review.md), [independent descriptor review](rom-0.0.2-discovery-independent-review.md). `studio/src/lib/client/` and `studio/tests/unit/`; latest recorded descriptor unit suite has 70 passing cases. |
| W | [Actual captured wire responses](evidence/rom-0.0.2/wire-parser/capture.json) and [finite parser benchmark](evidence/rom-0.0.2/wire-parser/benchmark.json); `scripts/research/{capture,benchmark}-studio-wire.mjs`. Twelve provider-authenticated native responses from both stores; parser/serializer source and executable hashes are recorded. |
| F | [Locked component trial](rom-0.0.2-frontend-compatibility-results.md), [controls](rom-0.0.2-studio-controls-results.md), [WebKit runtime](rom-0.0.2-webkit-runtime-results.md), [pagination](rom-0.0.2-studio-pagination-results.md), [custom wrappers](rom-0.0.2-wrapped-codec-results.md). `studio/tests/components/` and `studio/upstream-components.json`. |
| O | [Authentication comparison](rom-0.0.2-browser-auth-research.md), [sealed OIDC proof results](rom-0.0.2-oidc-proof-results.md). `crates/rom-auth/src/oidc/`, `crates/rom-identity/src/`, their tests and feature verifiers. |
| H | [Host contract and tests](rom-0.0.2-host-contract.md), [trusted HTTPS profile](rom-0.0.2-trusted-studio-profile-results.md), [shared file admission](rom-0.0.2-secure-host-files-results.md). `crates/rom-studio-host/src/`, `crates/rom-studio-host/tests/{configuration,backchannel,host,human,coherence}.rs`, `demo/provider-fixture/`. |
| B | [Actual Studio host](rom-0.0.2-actual-studio-host-results.md), [attachments and process recovery](rom-0.0.2-attachment-lifecycle-results.md). `studio/tests/runtime/{host.spec.ts,host-fixture.mjs}`, `demo/verify-studio`; [16-case actual-host regression](evidence/rom-0.0.2/attachments/full-host-results.log), [exact recovered-byte checks](evidence/rom-0.0.2/recovery-bytes/download-green.log). |
| T | [Shared-session logout and table measurements](rom-0.0.2-multi-tab-and-renderer-results.md); [both-engine/both-store logout run](evidence/rom-0.0.2/multi-tab/browser-results.log), [100/500-row samples](evidence/rom-0.0.2/multi-tab/table-frame-results.log). These reuse the recorded native binary rather than claiming a new source build. |
| N | [Native resilience](rom-0.0.2-resilience-results.md); `tests/persistence/tests/studio_resilience.rs` and named `authority`, `query`, `migration`, `support` children. [Both-store final run](evidence/rom-0.0.2/task6/native-final.log) and [strict Clippy](evidence/rom-0.0.2/task6/clippy-final.log). |
| A | [Packaging](rom-0.0.2-studio-packaging-results.md), [runtime notices](rom-0.0.2-runtime-notices-results.md), [independent notice review](rom-0.0.2-runtime-notices-independent-review.md). `scripts/release-artifacts/`, `scripts/packages/`, `scripts/check-packages.mjs`, `scripts/release`, and their Node tests. |
| R | [Integration review](rom-0.0.2-integration-review.md), [execution record](rom-0.0.2-progress.md), [dependency checks](rom-0.0.2-dependency-check-results.md), [prepared support contract](../release-support.md). |
| P | [Preview deployment plan](rom-0.0.2-preview-deployment-plan.md), [prepared acceptance](rom-0.0.2-preview-deployment-acceptance.md), [host preflight](rom-0.0.2-preview-host-preflight.md), [scoped activation](rom-0.0.2-scoped-preview-activation.md). These are not a deployed 0.0.2 acceptance record. |

## Task 1: contracts and discovery

| ID | Requirement | Status and exact boundary |
| --- | --- | --- |
| 1.1 | Failure-first input and disclosure cases | Complete. M: `input_descriptor_tests.rs`, `studio_discovery.rs`, metadata RED/GREEN logs cover scalar/unit/manual/derived inputs, aliases, invalid declarations, hidden references and bounds. |
| 1.2 | Intended compile diagnostics | Complete. M: `tests/compile/verify` and the corrected independent fixture locks retain diagnostic checks. Fixture failures are distinct from product failures. |
| 1.3 | Callback-free discovery registration | Complete. M: `resource/input_descriptor.rs`, `resource/definition.rs`, `discovery.rs`; registered metadata supplies disclosure without row decoding or action execution. |
| 1.4 | rom/derive, compile and native checks | Complete for this slice. M: `affected-final.log` contains rom and rom-derive suites plus three both-store discovery tests. Later compile-lock verification passed. |
| 1.5 | Exact real wire fixtures | Complete. `crates/rom-http/tests/loopback/discovery.rs::studio_wire_fixtures_use_actual_http_values_and_live_frames` emits discovery, invocation, projection, query, snapshot, live, error and action fixtures. M retains the executed output. This is not a parser benchmark. |
| 1.6 | Contract review and metadata commit | Complete. Contract and independent SDK/native correspondence review exist; metadata was committed as `ba96fe7`. R records subsequent accepted corrections. |

## Task 2: browser client

| ID | Requirement | Status and exact boundary |
| --- | --- | --- |
| 2.1 | Lossless and presence codec tests | Complete. C: `codec.test.ts`, `codec.ts`, `serialization.ts`, `normalization.ts`; boundary integers, unsafe keys, finite numbers, omission/null/removal and false/zero/empty values. |
| 2.2 | Reply, frame, byte and session correspondence | Complete. C: `client.test.ts`, `request-resilience.test.ts`, `discovery-stream.test.ts`, `anchor.test.ts`, `journal.test.ts`; journal cursors and native query anchors remain separate. |
| 2.3 | Mutation uncertainty and recovery | Complete. C: `mutation.ts`, unit correspondence/replay tests. B demonstrates a dropped committed Task patch followed by exact explicit retry. F tests retained stale drafts. |
| 2.4 | Parser implementation and wire benchmark | Partial ordering compliance. `serialization.ts` implements the narrow bounded lossless parser. W now records 12 native discovery/query/read fixtures, 31 batches × 100 iterations, round-trip checks and median/p95 timings. Measurement occurred after adoption. The original before-adoption ordering was not met. The 2–2,981-byte samples are not a universal performance or boundary-integer benchmark. |
| 2.5 | Bounded streaming and lifecycle | Complete. C: `request.ts`, `deadline.ts`, `stream.ts`, `session.ts`; byte/row/observer limits, cancellation and generation arbitration. `application/controller.ts` reopens a fresh authorized query after finite lease renewal. |
| 2.6 | Unit, both-store real HTTP and reviewed commit | Complete for the exercised client contract. C records unit/type acceptance; B uses the actual client, production assets, provider and both native stores. Final producer acceptance remains separate. |

## Task 3: stack and controls

| ID | Requirement | Status and exact boundary |
| --- | --- | --- |
| 3.1 | Actual Button/Input/Dialog/Table trial | Complete. F retains exact imported sources, locked install, type/build and three Chromium trial cases. The Dialog trial is historical evidence, not a current full-application Dialog test. |
| 3.2 | Compatibility selection, advisories and licenses | Complete for the selected graph. Stack research rejects incompatible alternatives; F executes the pinned graph. R records current audits. Original `svelte-toolbelt` LICENSE is retained despite missing package metadata. |
| 3.3 | Shared shape and custom fallback tests | Complete. F: `controls.spec.ts`, `Harness.svelte`; optional/nullable, enums, references, nested collections, exact integers, registered wrappers and read-only unknown codecs. |
| 3.4 | Shared renderers/forms/query controls | Complete. `studio/src/lib/renderers/`, `resources/`, `application/controller.ts`; F covers independent filter/sort fields, native anchor pagination and shared action inputs. |
| 3.5 | Labels, keyboard, focus and row removal | Partial. F tests labels, Enter activation and axe scans. The earlier Dialog trial tests containment and focus restoration. No current validation-focus assertion or live single-row removal scenario was found. Clearing all rows at logout is different evidence. |
| 3.6 | Type/component/build, reviewed provenance commit | Complete for recorded slices. F supplies passing checks and `upstream-components.json`; controls were committed as `9871266`, with later reviewed wrapper corrections. |

## Task 4: host and human sessions

| ID | Requirement | Status and exact boundary |
| --- | --- | --- |
| 4.1 | Verifier/library comparison and executable code flow | Complete. O compares current libraries using primary sources. H and B execute the chosen oauth2/current cryptographic verifier against provider-generated codes, tokens and JWKS. No execution of rejected libraries is claimed. |
| 4.2 | Negative credentials and host approval | Complete. O supplies real-signature nonce/issuer/audience/signature negatives. H supplies state/callback replay, exact endpoint/origin, redirect and conflicting configuration tests. |
| 4.3 | Session, cookie, CSRF, identity and concurrent tabs | Complete for recorded cases. H covers expiry, logout, current User/link/provider changes, cookies, Origin/CSRF and bounded admission. T adds real same-session concurrent tabs on both engines and stores. |
| 4.4 | Second-tab logout closes active delivery | Complete for the recorded run. T has four passing cases using `context.newPage()`, one real cookie session, an active live request, network termination and clearing in both tabs. The initial `Response.finished()` harness timeout remains separately recorded. A later timing-strengthened test remains subject to its own source freeze and checks. |
| 4.5 | Separate ID-token profile and current activation | Complete. O: old JWT tests remain passing; sealed profiles reject cross-profile proof binding and stale Provider activation. No public proof constructor or browser Actor bridge was added. |
| 4.6 | Real login and protected opaque sessions | Complete. H/B execute code+PKCE login and current Resource binding. Host-only token material stays outside assets; browser cookies are opaque and HttpOnly. |
| 4.7 | Base paths, generic API and finite supervision | Complete. H: assets/router/configuration/login/lifecycle modules and real TCP tests. No per-kind transport route replaces the generic pipeline. Deployed proxy acceptance remains pending. |
| 4.8 | Bounded blobs and observable shutdown barriers | Complete. H tests supervised authenticated intake. B's separate process test observes accepted OIDC exchange and provider publication before SIGTERM, then releases both barriers. |
| 4.9 | Provider/browser/both-store acceptance and commit | Complete for recorded host slices. B includes Chromium and WebKit on SQLite and redb; host and later supervised blob/profile slices are committed. The added multi-tab case has its own unresolved acceptance boundary. |

## Task 5: screens and browser acceptance

| ID | Requirement | Status and exact boundary |
| --- | --- | --- |
| 5.1 | Tasks and Inventory full generic workflows | Partial. B covers Task read/patch/retry/query/live; Inventory is read-only in the actual-host journey. Shared create/delete/action controls have fixture evidence. Both-kind actual-browser create/delete/action journeys were not found. |
| 5.2 | Held-out Resource and public custom codec | Complete for the demonstrated extension. `demo/src/studio_model.rs::MaintenanceTicket` and `TicketCode`, `demo/studio/{main.ts,TicketCode.svelte}` use public registration. B's registered-renderer run needs no generic view edit. This is agent-authored evidence. |
| 5.3 | Login/admin/shared views and recovery states | Complete. `studio/src/lib/application/` composes discovered Resource pages, auth, Work and attachments. F/B exercise primary-provider selection, uncertain mutation recovery and stale draft/conflict behavior. |
| 5.4 | Faults, reconnect and descriptor change during draft | Partial. C/F/B cover dropped replies, stale navigation, revocation, finite lease reconnect and row-revision conflict. `ResourceForm.svelte` guards definition-version changes, but no executed open-draft descriptor-change scenario was found. |
| 5.5 | Two engines/stores plus keyboard/a11y | Partial. B has both-engine/both-store real-host acceptance. F has both-engine assembled-control keyboard and axe checks. No scan or complete keyboard workflow of the connected human-session application was found. Neither suite certifies human usability. |
| 5.6 | Bounded rows, frames, teardown and request measurement | Partial. B records real-host request counts and timings on three Tasks. New F table samples pass both engines at 100/500 rows: 1,600/8,000 cells, then zero after clear. `multi-tab/table-frame-results.log` measures DOM retention, not heap or sustained live-frame retention. C/F test byte, row, observer, history and teardown bounds; measured repeated live-frame retention remains absent. |
| 5.7 | External author workflow and integrated commit | Partial. R reviews public seams; B demonstrates the held-out demo extension. `examples/studio-consumer` and an independently extracted Studio author-consumer journey were not found. Existing external Rust custom-field evidence does not establish the complete Studio author path. |

## Task 6: native resilience

| ID | Requirement | Status and exact boundary |
| --- | --- | --- |
| 6.1 | Actual process interruption during accepted work | Complete. B: `host.spec.ts` SIGTERM case pauses admitted authentication and upload before requesting process stop. Tests do not merely stop an idle server. |
| 6.2 | Drain, ownership, reopen and recovery | Complete for graceful SIGTERM. B checks retained ownership, delayed exit, fresh login, Ready revision and exact recovered attachment bytes on both stores/engines. It does not certify forced-exit or power-loss behavior. |
| 6.3 | Actual human-provider backup/upgrade journey | Partial, new execution pending. B covers actual login/mutation/restart/new login. N covers maintenance using test-signed proofs. The coordinator added `crates/rom-studio-host/tests/support/maintenance.rs` with actual provider, backup/restore and schema migration. `task4/maintenance-provider-focused.log` was still compiling; no passing result was available at this cutoff. |
| 6.4 | Reentrant public Storage query policy | Complete. N: `studio_resilience/query.rs`, four bounded child processes, both stores and reference/uniform eligibility. No core change was needed. |
| 6.5 | Seeded operator obligation through migration | Complete. N: `studio_resilience/migration.rs::seeded_strict_hold_and_operator_receipt_survive_native_migration_and_replay`; stable identity/profile/evidence/receipt, changed storage generation/revision, stale control rejection and exact replay. Provider evidence at this trusted storage port is synthetic. |
| 6.6 | Focused fixes for demonstrated defects | Complete for reported findings. C/H/F/R retain failure-first corrections for correspondence, cancellation, draft errors, shutdown drain and file admission. N explicitly reports no reproduced core defect. |
| 6.7 | Affected checks, full verifier and independent review | Pending producer. N/H/B and R supply scoped checks and reviews. The latest integrated native run stopped at fixture locks; their focused correction passed. No complete final-source `scripts/check`/eight-gate result was found. |

## Task 7: package and release

| ID | Requirement | Status and exact boundary |
| --- | --- | --- |
| 7.1 | Exact 0.0.2 package set and format notes | Complete in source. Workspace/package manifests and frontend lock align; `extensions/native-alpha-v1.json` declares 0.0.2. R documents intentional ordering and unchanged native format 8/archive format 6. |
| 7.2 | Fixed native/frontend/extracted-asset gates | Complete in implementation. `scripts/release-artifacts/commands.mjs` preserves six native gates, then fixed browser gate 7 and actual-host gate 8. Fresh copied asset production/extraction precedes gate 8. A's fixtures test command correspondence. |
| 7.3 | Packaging negatives and exclusive publication | Complete in focused tests. A covers missing/tampered assets/notices, lock/profile/inventory changes, large Git archives and no-replace publication. Recorded Node acceptance is separate from actual release execution. |
| 7.4 | Extracted consumers and feature isolation | Pending producer for final Rust set. `scripts/check-packages.mjs` checks each extracted crate and an external consumer without workspace paths. O separately executed no-default/jwt/oidc/introspection builds and core isolation. A executed fresh copied offline frontend build/extraction, later including notices. Final clean 0.0.2 Rust-package plus extracted-asset host acceptance remains pending. |
| 7.5 | Advisory/license inventories and corrected defects | Complete for recorded locks. R records zero matching Cargo/npm advisories. A retains original emitted-runtime notices and rejects unclassified emitted JavaScript. Historical failures remain. Recheck if final dependency identities change. This is engineering evidence, not legal certification. |
| 7.6 | Clean final archives and independent verification | Pending producer. Implemented source/tree/lock/profile/inventory/checksum gates exist in A. No completed clean 0.0.2 output exists. Historical release and private fixture artifacts do not substitute. |
| 7.7 | Persistent protected VPN HTTPS preview | Pending preview. P defines supervised Rust/provider services and approved credential/data boundaries. No activation acceptance was found. |
| 7.8 | Preview restart/routes/callback/proxy/stream and Mac access | Pending preview. P lists these probes. Loopback browser success and host preflight do not establish deployed gateway or Mac VPN routing. |
| 7.9 | Accepted main integration and push | Pending integration. Implemented slices are committed on the release branch. The ordinary checkout remains at the baseline. No Actions or registry publication is proposed. |
| 7.10 | Final completion after artifacts and preview | Pending producer and preview. This mapping is not release completion evidence. Artifacts, final acceptance mapping and persistent preview results must all exist. |

## Concrete remaining evidence

1. Record the parser benchmark's ordering deviation explicitly. W now supplies finite post-adoption native-fixture measurements. Its small seeded samples and separate unsafe-integer correctness tests have different scopes.
2. Freeze the new two-tab timing-strengthened test and retain its separate result. The first both-engine/both-store run now passes. Its harness timeout is not a demonstrated product defect.
3. Exercise Task and Inventory create/delete/actions through the shared actual-browser interface. Preserve both stores and engines.
4. Change the descriptor version while a draft is open. Check retained draft, blocked submission and explicit reconciliation. A changed row revision is a separate case.
5. Add validation-focus and live single-row-removal assertions. Run the connected-page keyboard and accessibility checks. Existing axe results apply only to their assembled fixtures.
6. Measure repeated live snapshots and subscription teardown under a declared finite sample. Distinguish current DOM rows, retained frames, request counts and heap measurements.
7. Run actual-provider login, mutation, explicit backup/upgrade/reopen and fresh current identity as one both-store journey. Keep the test-signed native migration proof attributed separately.
8. Supply the external Studio author-consumer artifact, or record an explicit reviewed plan change. Public demo registration is useful evidence, but is not independent package consumption or human usability.
9. After source and capacity acceptance, execute the unchanged complete producer. Verify its final extracted Rust consumers and own production assets, then accept the persistent preview and separate external access probes.

These are acceptance gaps, not demonstrated implementation defects unless an executed scenario establishes one.
The capacity checkpoint defers the producer; it does not waive any gate or certify a hard disk-allocation bound.
