# ROM 0.1.0 release coordination

Date: 2026-10-07. Status: active goal; investigation in progress. This is not release acceptance.

## Owner-authorized scope

Complete the [production-consumer research plan](rom-0.1.0-consumer-research-plan.md) and the accepted generic Resource contract.
Investigate source before research, selection, and implementation. Include reusable consumer usability improvements.
Add optional smart OpenRouter routing and ergonomic agent flows through existing Resource, action, event, and durable work contracts.
The core must remain independent of model providers, transports, and database drivers.

## Consumer coordination

Consumer: `/root/astral-plane`.
The [consumer thread](thread://01a111a5-617a-7800-b94f-eb82b11da39d?hostId=remote-ssh-codex-managed%3A7455bafd-bcdf-4924-b743-87f13bd5b771) was read on 2026-10-07.
The read covered the latest turn and three older pages. The last response reported `hasMore=false`.
Pagination completion does not guarantee that provider-limited messages are untruncated.
Private extracts retain turn/message identities and page metadata. Do not publish raw personal messages or credentials.

There is no callable cross-thread message-delivery tool in this session.
A shared-file handoff is placed at `/root/astral-plane/docs/rom-0.1.0-coordination.md`.
The existing [consumer feedback](astral-plane-production-feedback-2026-10-07.md) remains unchanged.
No acknowledgement from the original consumer agent is claimed.
No deployed consumer, vendor source, provider settings, or live data has been changed by this coordination.

## Feedback intake contract

Each usability observation receives a stable ID, source turn/message reference, and classification.
Classifications: shared ROM defect, public-interface friction, documentation/discoverability gap, application-specific behavior, or unresolved evidence.
Record duplicates, superseded observations, withdrawn requests, and conflicts instead of silently discarding them.
A domain-specific preference does not automatically become a core feature.

Each accepted shared item must identify its maintained source location, proposed generic contract, regression, and external-consumer acceptance.
Track states: collected, investigated, researched, selected, implemented, independently reviewed, verified, deferred, or rejected with reason.
A passing isolated fixture does not close an item requiring deployed or packaged-consumer acceptance.

## Maintained investigation and intake

The [usability intake](rom-0.1.0-usability-intake.json) initially recorded 30 stable groups from the returned conversation and inspected source. The reconciled intake contains 32 groups.
The [usability investigation](rom-0.1.0-usability-investigation.md) maps each group to a generic seam and acceptance scenario.
The [core investigation](rom-0.1.0-core-investigation.md) identifies production composition and recovery gaps.
The [AI investigation](rom-0.1.0-ai-investigation.md) maps routing and agent flows to existing durable work.

These are source investigations, not executed acceptance tests. Existing consumer test reports remain reported evidence.
Framework candidates require selection and implementation. Application-specific requests remain visible in the intake.
The withdrawn frame-rate request requires no ROM implementation.

## Investigation ownership

| Investigation | Exclusive report ownership | Source access |
| --- | --- | --- |
| Consumer usability and shared Studio behavior | `/var/tmp/rom-010-ui-investigation.md`, `/var/tmp/rom-010-usability-intake.json` | Read-only ROM/consumer source and thread extracts |
| AI routing and agent flows | `/var/tmp/rom-010-ai-investigation.md` | Read-only consumer execution and existing ROM work contracts |
| Production core and other-project lessons | `/var/tmp/rom-010-core-investigation.md` | Read-only core, conformance, deployment, and bounded project comparisons |
| Coordination, research plan, release specification | Maintained ROM documents assigned to the root agent | Integrate reports without modifying their evidence |

No investigation agent owns product code, live data, shared build targets, or deployment configuration.
Implementation file ownership will be assigned after the interfaces and investigation results are reconciled.

## Additional AI research scope

Investigate a provider-neutral invocation contract and a separate OpenRouter adapter.
Separate model eligibility, provider routing, fallback order, and an aggregate run budget.
The deadline must cover discovery, waiting, each attempt, validation, and fallback.
Retries must retain run identity and progress rather than restart a failed catalog traversal.
Track actual cost and usage separately from reservations and model-price estimates.
Do not promise a strict spend ceiling when the provider's accounting cannot establish one.

Agent flows need typed tools, authorized Resource access, validated outputs, durable step state, cancellation, and restart recovery.
Tool mutations use normal actions and stable idempotency identities. A model output does not grant authority.
Separate ephemeral token streaming from durable result and work events.
Private context disclosure must be explicit. Credentials do not belong in Resources, prompts, logs, or public progress projections.

Domain prompts, Human Design interpretation, and business knowledge remain in the consumer.
The framework should supply execution and composition machinery, not domain answers.

## Completion boundary

Investigation reports and research recommendations do not complete the active release goal.
Complete the selected implementations, native/browser checks, external-consumer acceptance, and production-profile recovery/load/upgrade trials.
Run the full release producer and independent artifact verification against clean source.
Publish the matching source and tag only after the accepted deployment and support limits are recorded.
GitHub remains source hosting; no registry publication or Actions activation is implied.

## Release triage

The intake preserves the investigation classification and adds provisional release triage.
A candidate labeled `shared_rom_defect` still requires a producer reproduction before it becomes a confirmed defect.
Model failover preferences inform the generic routing policy; application content and brand remain in the consumer.
Every candidate remains open until selection and the required regression and external acceptance are complete.

## Intake verification

The [independent intake review](rom-0.1.0-usability-intake-review.md) checked all 30 groups. Its three consistency findings were corrected.
Local checks confirmed 66 source references, 47 test-file references, and 13 local document links before the review record was added.
Whitespace checks passed for the imported documents and consumer handoff. No product tests were executed for this documentation-only intake.
The checked file references identify proposed acceptance locations; their existence does not establish passing tests.

## Foundation implementation status

Captured validation has targeted evidence and a [coordinator implementation review](rom-0.1.0-captured-validator-review.md).
The combined local verifier exited zero; its log and source limits are `/var/tmp/rom-010-foundation-evidence/`.
The public-control review found stylesheet dependency and acceptance-fixture defects after the initial targeted pass.
The corrective candidate passed seven admission tests and six locked-consumer browser tests.
Release gate integration remains open; candidate passes do not close the public packaging work package.
Mutation recovery implementation and AI scheduling design are in progress under separate ownership.
No 0.1.0 release, clean artifact admission, production readiness, or consumer-agent acknowledgement is claimed.

## Fresh consumer synchronization

The coordinator reread the latest three consumer turns on 2026-10-07.
The latest returned turn remains `01a115c2-999b-77b2-a90f-449043de2e9a`.
The thread update marker remains `1791379117`. No additional owner usability request was identified in those turns.
This fresh read does not replace the earlier paginated intake or prove that truncated messages were complete.

The UI and AI agents received the synchronization result and the existing intake closure requirements.
The consumer handoff contains the same status. No acknowledgement from the original consumer agent is claimed.

The [corrective control review](rom-0.1.0-public-controls-review.md) separates candidate verification from release admission.
Mutation recovery now has targeted adversarial tests; its external-consumer acceptance remains open.

## Public client candidate and recovery review

The package now exports client and recovery subpaths. The minimal installed consumer first exposed their missing export definitions.
After correction, locked installation, type check, build and eight Chromium/WebKit cases passed.
The evidence context is `/var/tmp/rom-010-public-client-evidence/context.md`.
That context identifies the copied checkout candidate. Its CLI admission label does not establish independently verified clean release source.

The [independent helper review](rom-0.1.0-mutation-recovery-review.md) includes a root rerun of 38 targeted recovery tests.
Actual HTTP, adapter restart, IndexedDB and navigation acceptance remain in progress.
The root App entry still requires [portable source refactoring](../superpowers/plans/2026-10-07-portable-studio-root.md).
No narrower subpath success closes the full external-consumer contract.

## Message-level feedback reconciliation

The renewed owner request requires every usability observation to remain traceable.
The fresh read returned 29 user messages in three turns. The latest turn and update marker remain unchanged.
The [synchronization audit](rom-0.1.0-feedback-sync-2026-10-07.json) records message identities without publishing their text.

The comparison found missing references within existing groups. These references now cover launch controls, sources, new conversations, and answer freshness.
AP-UX-031 adds date timeline selection. AP-UX-032 preserves the explicitly withdrawn music request.
The intake now contains 32 groups. One resume-work message contains no additional usability requirement.
All 29 observed messages map to intake references or that explicit non-usability classification.
This check does not establish complete coverage of provider-truncated messages or uninspected image attachments.

The UI, AI, and core agents received the renewed coverage requirement.
Each agent must identify uncovered expectations and distinguish generic implementation from application-owned behavior.
The shared consumer handoff requests source revisions, reproductions, and acknowledgement. No original-agent acknowledgement has arrived.

The portable root App candidate passed 12 installed-consumer browser cases across Chromium and WebKit.
The maintained Studio component suite passed 221 cases and skipped one case.
These results concern checkout candidates. Clean release admission and deployed consumer acceptance remain open.

## Current integration boundary

A repeated read returned the same consumer update marker and latest turn. No new feedback item was identified.
The three workers received explicit source ownership and the shared intake obligations.
The UI worker maps reusable behavior; the AI worker researches public UI contracts while AI source remains frozen.
The core worker investigates the actual identity-provider boundary and corrects the downstream migration fixture.

The combined verifier found a missing `not_before` field in the historical demo work initializer.
The failure log is `/var/tmp/rom-010-integrated-candidate-check-20261007.log`.
The historical fixture requires `None`, which preserves its immediate-work meaning. Focused and complete checks remain separate acceptance steps.
Source investigation also predicts Authentik key-ID and JWKS metadata incompatibilities, plus session loss during acquisition failure.
These provider findings require actual failing experiments before a compatibility claim or broad decoder change.
No open feedback group, release gate, or deployed consumer is closed by this synchronization.

## Renewed four-turn synchronization

The coordinator reread four recent turns. The thread update marker remains `1791379117`.
The comparison covers 36 user-message references and the existing 32 intake groups.
All references map to intake groups or two explicit exclusions: resume work and credential setup.
No credential value or raw message text is retained in this audit.
The UI, AI, and core workers received the renewed coverage and acceptance requirements.
No new usability group was identified. Truncated messages and uninspected images still limit coverage.
The consumer agent has not acknowledged the handoff. No deployment or vendor replacement was performed.

## Public session and expanded AI candidate review

The [public session review](rom-0.1.0-auth-lifecycle-review.md) records independent source, unit, installed-consumer, and browser checks.
A custom-driver logout credential retention defect was reproduced and corrected before candidate review.
The corrected source passed 24 targeted tests and 208 Studio unit tests. Type checking reported zero errors and warnings.
The installed consumer passed 12 cases with each tested WebKit selection and Chromium. Production identity remains open.

The combined verifier exited zero at `/var/tmp/rom-010-integrated-auth-checkpoint-check-20261007.log`.
It includes the expanded AI checkpoint candidate and the corrected session source.
The separate container Node gate passed 113 tests. The host-only attempt failed because its PATH lacked rustc.
That environment failure is retained at `/var/tmp/rom-010-integrated-node-auth-review.log`; it was not classified as a product regression.

AI Work composition may now proceed under exclusive module ownership. Legacy session integration still requires a compatible bridge.
No release gate, feedback group, deployment, or publication is closed by these candidate results.

## Feedback synchronization, 2026-10-07T18:42 UTC

The coordinator reread six recent turns from the linked consumer conversation.
The latest turn remains unchanged. The thread update marker is now 1791396483.
All 41 returned user-message references already occur in the maintained source audit.
The intake retains 32 groups. No new feedback reference was found.
Truncated text and uninspected images remain coverage limits.

The AI and production identity workers received the renewed acceptance requirements.
The release coordinator owns remaining Studio creation-draft and composition work.
The previous Studio worker completed its candidate work; it is not an active worker.

Each active group requires a generic implementation mapping, executed regression evidence, and original-consumer acceptance.
The installed App candidate passed 32 authoring executions across SQLite, redb, Chromium, and WebKit 2359.
These results cover 28 selected-resource recovery cases and four maintained-main startup diagnostics.
Evidence is in /var/tmp/rom-app-session-evidence/app-matrix-authoring.json.
The coordinator independently reran 292 unit cases and checked the frozen UI source hashes.
Creation-form persistence, reusable composition, complete release admission, and original-consumer acceptance remain open.

Please record acknowledgement or missing observations here with source IDs and minimal reproductions.
Direct cross-thread messaging remains unavailable. This entry does not establish acknowledgement or deployment acceptance.
## Renewed owner feedback handoff

The coordinator reread six recent consumer turns on 2026-10-07.
All 41 returned user-message references match the maintained audit. The intake retains 32 groups.
The AI and production identity workers received direct coordination messages.
The release coordinator owns creation recovery and remaining Studio composition.
Each active group requires implementation mapping, regression evidence, and original-consumer acceptance.
Please attach missing reproductions and acknowledgement here with the existing AP-UX identifiers.
The source audit records read time and reference coverage without publishing raw messages.
Direct cross-thread delivery and consumer acknowledgement remain unavailable or unconfirmed.
Truncated text and uninspected images remain coverage limits.

Creation workflow and application wiring now passed 313 Studio unit cases and Svelte checking.
These are authoring checks. Creation browser acceptance and release admission remain open.
The earlier 32 browser executions used an earlier candidate and do not validate the new creation source.

## Renewed owner coordination: expanded feedback coverage

The coordinator reread eight turns from the linked Astral Plane conversation on 2026-10-07.
The read covers 47 user-message references. All inspected references now have a classification.
Six older references extend the previous synchronization. The latest thread update marker remains 1791396483.
AP-UX-033 records guest invitations and optional account save. The intake now contains 33 groups.
Invitation and chart behavior remain application-owned. Shared forms, authorization, idempotency, and draft recovery remain ROM responsibilities.
The consumer reports invitation implementation and tests. Those reports are not independently reproduced ROM acceptance.

The AI and identity workers received direct coordination messages. The coordinator owns remaining Studio integration.
Each active group requires an implementation mapping, executed regression evidence, and consumer acceptance.
Withdrawn requests remain withdrawn. No candidate test result closes the release or an original feedback item.
Please record missing observations, source revisions, minimal reproductions, and acknowledgement with AP-UX identifiers.
Direct cross-thread messaging is unavailable. The shared handoff does not establish acknowledgement.
Older paginated turns, truncated text, and uninspected images remain coverage limits.

## Creation feedback candidate: descriptor renewal and navigation

AP-UX-004/005 now have candidate coverage for exact creation replay, original-draft cleanup, later invalid edits, and descriptor renewal.
Explicit discard adopts the new definition. Pending original commands retain their identity and current-authority checks.
Blocked row and page controls no longer dispatch navigation. Forced events remain guarded. Successful discard closes its confirmation dialog.
The current source passed 333 unit tests, 231 component browser tests, and 40 installed-App executions. One existing component test was skipped.
The installed matrix covers both supported database adapters and both browser engines. It uses a prior witnessed native fixture and synthetic identity.
Independent review found no remaining reported creation P2 finding. It does not establish original-consumer acceptance.
The full verifier stopped before Rust because a provider test required a host-only directory absent in the development container.
That correction and a complete rerun remain open. No release gate or consumer feedback item is closed by these candidate results.
Evidence: /var/tmp/rom-010-creation-renewal-evidence.json.

## Feedback synchronization: renewed owner request

The coordinator reread eight consumer turns on 2026-10-07.
All 47 returned user-message references match the classified intake of 33 groups.
No new unclassified reference appeared in this read. Older turns and uninspected images remain coverage limits.
The AI and production identity workers received renewed acceptance requirements.
Every active feedback group needs generic implementation, executed regression evidence, and original-consumer acceptance.
The portable Node gate passed 194 tests after the provider test-path correction. This is not a full-verifier result.
UI composition research identified synchronous abort reentrancy and exact wire-cloning requirements before implementation.
Direct cross-thread messaging remains unavailable. This shared handoff does not establish consumer acknowledgement.
Please record missing usability observations, source revisions, and minimal reproductions with AP-UX identifiers.

## Renewed owner feedback coordination

The coordinator notified the UI, core, and AI workers of the renewed owner request.
The existing verified intake contains 33 groups. Each group requires implementation, an executed regression, and original-consumer acceptance.
Application-owned behavior must remain explicit; a generic helper test cannot establish consumer acceptance.

A new `read_thread` call did not return before termination. No new conversation content was used or assumed unchanged.
The earlier verified intake remains the working baseline. Truncated text and uninspected images remain coverage limits.
No direct cross-thread messaging tool is available. This shared handoff does not establish acknowledgement by the original agent.

The observation review reproduced three callback and reentrancy defects despite 14 passing baseline tests.
See `/root/ROM/docs/research/rom-0.1.0-observation-independent-review.md`. These defects remain open pending correction and maintained regressions.
The ResponsiveDetails candidate passed 257 browser cases with one skip; installed public-entry and consumer acceptance remain open.
The native local verifier exited zero for the preceding candidate; its log is `/var/tmp/rom-010-observation-ai-preparation-full-verifier.log`.
That verifier result does not override the independent observation defects or establish release readiness.

## Verified feedback synchronization — 2026-10-08

A fresh `read_thread` returned eight turns and 47 user-message references.
The thread update marker is `1791396483`. All returned references occur in the existing intake or earlier synchronization records.
The intake has 33 groups. Presence in a synchronization record alone does not prove a classified acceptance requirement.
An independent coverage review will check every reference against a group, withdrawal, or non-usability disposition.
The sanitized read record is `/root/ROM/docs/research/rom-0.1.0-feedback-sync-2026-10-08.json`.
Older paginated coverage remains available. Truncated messages and uninspected images remain explicit limits.

Each active AP-UX group must retain its original acceptance requirement.
Track generic implementation, executed regression, installed consumer acceptance, and human feedback separately.
Application-owned requirements remain in scope for the original consumer; they are not automatically framework features.
Please record additional feedback with its source reference, reproduction, consumer revision, and affected AP-UX group.
Direct cross-thread messaging is unavailable. This shared file is a handoff, not acknowledgement from the original agent.

The observation callback defects now have maintained regressions and an independent corrective review.
See `/root/ROM/docs/research/rom-0.1.0-observation-corrective-review.md`.
History and layout validation also have a completed independent subset review.
See `/root/ROM/docs/research/rom-0.1.0-history-layout-independent-review.md`.
ReferencePicker has public component candidate evidence; original consumer acceptance remains open.
Selection cards and layout controls remain implementation work. These results do not establish release readiness.

## Feedback accounting checkpoint — 2026-10-08

The independent reviewer matched all 47 recent references by exact turn and item identifiers.
Of these, 41 map to active requirements, four map to withdrawn requests, and two have explicit non-usability dispositions.
No unmatched recent reference remains. AP-UX-002, AP-UX-003, and AP-UX-010 remain source-derived candidates, rather than direct user requests.
The full report is assigned to `/root/ROM/docs/research/rom-0.1.0-feedback-coverage-2026-10-08.md`.
Original-consumer acceptance remains open for active groups. No original-agent acknowledgement is claimed.

## Current candidate handoff — 2026-10-08, 07:14 UTC

The coordinator repeated the recent eight-turn read. All 47 references match the audited baseline; no new reference appeared.
See `/root/ROM/docs/research/rom-0.1.0-feedback-recheck-2026-10-08.json`.
The 33 intake groups remain the shared requirements ledger. Withdrawn requests remain marked as withdrawn.

SelectionCard and LayoutControls now have candidate implementations and an independent review.
The review passed seven focused tests and 12 adversarial probes without a reproduced actionable defect.
See `/root/ROM/docs/research/rom-0.1.0-selection-layout-independent-review.md`.
An installed public package consumer passed 22 browser cases across Chromium and WebKit, plus type checking and build.
See `/root/ROM/docs/research/rom-0.1.0-installed-workspace-2026-10-08.md`.
That fixture uses manual reference entry and synthetic observation. It does not establish original-application acceptance or release readiness.

The identity worker reproduced session loss after a real provider outage. Three maintained classifier regressions changed from failure to success.
The affected Host suite passed 44 tests and Clippy. Current-source provider verification remains pending a coordinated rebuild.
The AI worker is checking nullable tool results, concurrent resume, current grants, and bounded transcripts.
Workers must retain AP-UX identifiers and distinguish library regressions from acceptance in Astral Plane.

For each original-application retest, record the application revision, installed ROM package identity, AP-UX identifier, reproduction, and outcome.
Keep screenshots and human observations where available. A library fixture alone cannot close an original-application requirement.
Direct cross-thread messaging remains unavailable. This shared handoff does not claim acknowledgement by the original agent.

## Authority-loss and HTTP integration increment — 2026-10-08

Source investigation linked AP-UX-006, AP-UX-013, and AP-UX-021 to confirmed-denial handling in the legacy browser adapter.
Two maintained HTTP 401/403 tests reproduced generic errors before correction. The HTTP 503 control remained distinct.
The public `/auth` entry now exports `SessionDeniedError` alongside `SessionExpiredError` and `createBrowserAuth`.
Consumers must clear private views for either confirmed denial or expiry, while treating temporary acquisition failures separately.
See `/root/ROM/docs/research/rom-0.1.0-legacy-authority-loss-2026-10-08.md`.
Astral Plane's current source catches expiry only. Adoption and actual application/browser confirmation remain open.
No deployed consumer or vendor snapshot was changed, and no original-agent acknowledgement is claimed.

The independent maintenance application passed six tests, including two actual HTTP/database-close/reopen journeys.
See `/root/ROM/docs/research/rom-0.1.0-maintenance-http-2026-10-08.md`.
Fixed fixture credentials are not production authentication. Installed Svelte integration and release admission remain open.

## Renewed consumer coordination — 2026-10-08

The coordinator reread the linked thread after the owner's renewed request.
The update marker changed to `1791445000`. The consumer's final message reports commit `86dca6f`.
The local consumer HEAD matches that commit. Deployment and test statements remain consumer-reported evidence, not coordinator-reproduced results.

The returned page contains five user-message references. All five occur in the earlier audited intake.
Some historical turns now contain no items. This read does not replace the earlier 47-reference coverage audit.
The sanitized read record is `/root/ROM/docs/research/rom-0.1.0-feedback-coordination-refresh-2026-10-08.json`.
No raw private messages or credentials were published.

The consumer report is `/root/ROM/docs/research/astral-plane-production-feedback-2026-10-07.md`.
Its eight API-friction rows remain requirements, not automatic acceptance:

| Consumer feedback | ROM requirement or review |
| --- | --- |
| Captured transition validator | Review runtime-scoped validation and isolation. Keep function-pointer callers compatible. |
| Public controls | Verify supported controls from an installed public package without source aliases. |
| Wrapper and control styling | Verify the documented styled-node contract, including width, disabled, invalid and focus states. |
| Shared-install license notices | Retain resolved-install ownership checks, complete license text and negative admission tests. |
| Compact forms | Test composer geometry, native values, date/time and select bindings in a consumer. |
| Unknown mutation outcomes | Test original command recovery, reload, permission changes and safe navigation. |
| Durable work composition | Provide visible stages, attempts, receipts and actionable recovery through public APIs. |
| Public projections | Test authorized public data, redaction, bounded queries and cache revisions without exposing private jobs. |

The UI worker received the full-feedback requirement and the actual visual-layout acceptance gap.
The identity worker received confirmed-denial, natural-expiry and temporary-outage distinctions.
The AI worker received practical conversation, lazy research, typed tools, immutable sources and provider-policy requirements.
These worker messages were delivered through the collaboration tools.

Each active AP-UX group needs separate implementation, regression, installed-consumer and original-application evidence.
Withdrawn requests stay withdrawn. Application-owned behavior remains visible in the ledger.
The consumer still uses vendored ROM and local Studio source aliases. This does not establish packaged-release acceptance.

Direct messaging to the original chat is unavailable in this environment.
This shared document is a handoff. It does not prove acknowledgement by the original agent.
For a consumer retest, record its revision, installed package identity, AP-UX group, reproduction and result.

## Supervised calculation and portal review increment — 2026-10-08

Runtime::calculate now uses the existing tracked I/O lifecycle and the exact configured Rayon pool.
Three external tests passed, each on SQLite and redb. Core and consumer all-target Clippy checks passed.
Independent source review confirmed admission and shutdown ownership, and required explicit nested-work and detached-child limits.
See `/root/ROM/docs/research/rom-0.1.0-supervised-calculation-2026-10-08.md`.
The AI ReadContext integration and combined full verifier remain open.

The connected portal's frozen matrices passed 26 cases per database across Chromium and WebKit.
Actual panel geometry follows confirmed stored layout, including movement, resize and reload.
These matrices use the preserved earlier native fixture. They do not verify the newly added calculation API.
Evidence: `/var/tmp/rom-010-portal-grid-matrix-witness.json`.

Independent review identified two candidate defects for maintained reproduction.
WorkOrder has no explicit recovery controls after lost acknowledgement or reload.
Proxy bind failure can leave its previously launched backend running.
The passing matrices do not override these findings. Correction and independent acceptance remain required.

## Full-feedback coordination checkpoint — 2026-10-08

The coordinator reread the original chat after the owner's renewed coordination request.
Its update marker remains 1791445000. The latest consumer-reported commit remains 86dca6f.
The single returned turn does not replace the historical 47-reference coverage audit.
The 33 AP-UX groups remain the requirements ledger. Withdrawn requests remain withdrawn.

The coordinator sent this requirement to the active UI, identity, and AI workers through collaboration messages.
Each worker must retain the feedback mapping and distinguish implementation, regression, installed-consumer, and original-application acceptance.
The tool set still has no direct message operation for the original chat. This handoff does not establish its acknowledgement.

Independent review accepted the two portal corrections for the witnessed candidate.
WorkOrder recovery now retains the original command after lost acknowledgement and reload.
Proxy bind failure now drains its owned backend before it returns the original error.
The actual SQLite/redb browser matrices passed 60 cases across Chromium and WebKit.
Evidence: /var/tmp/rom-010-portal-recovery-matrix-witness.json.
These results do not establish original Astral Plane adoption.

The next public-projection consumer uses a separate, explicitly authorized guest Resource.
The guest cannot read complete private values, invoke mutations, or discover private fields.
Current HTTP regressions passed on both database adapters. Its connected browser acceptance remains in progress.
The final combined verifier, original-application retest, and release admission remain open.

## Renewed full-feedback assignment — 2026-10-08

The coordinator read the linked consumer thread again after the owner's latest request.
The thread update marker remains 1791445000. The consumer reports commit 86dca6f.
See /root/ROM/docs/research/rom-0.1.0-feedback-owner-refresh-2026-10-08.json.
This recent-page read does not replace the historical 47-reference audit or its 33 feedback groups.

The coordinator delivered renewed assignments to the active AI, identity, and operations workers.
The coordinator owns the remaining UI and original-consumer acceptance mapping.
AP-UX-030 and AP-UX-032 remain withdrawn. Domain-specific requirements remain assigned to the consumer.

The current portal matrices passed 80 browser executions across two database adapters and two browser engines.
They include live-result invalidation and rejection of an export after the selection changes.
See /root/ROM/docs/research/rom-0.1.0-public-result-invalidation-2026-10-08.md.
These results do not prove adoption in the original application.

The combined local verifier exited zero. Its log is /var/tmp/rom-010-invalidation-ai131-full-verifier.log.
That verifier includes the feature-enabled OpenRouter adapter tests and the external AI consumer.
Application recovery, observability implementation, bounded load, original-consumer acceptance, and release admission remain open.

For each active feedback group, record its public API, regression, installed consumer, original application, and human observations separately.
For an application retest, record the consumer revision, installed ROM identity, AP-UX group, reproduction, and result.
Direct messaging to the original conversation remains unavailable. This handoff does not establish acknowledgement by its agent.

## Actual predecessor writer compatibility increment — 2026-10-08

A separately witnessed executable compiled against accepted 0.0.3 source populated both database adapters.
Current native upgrades preserved logical data and fenced predecessor claims and cursors.
Four predecessor archive-reader controls accepted archive 6 and rejected archive 7 without changing archive bytes.
See [the executed subset report](rom-0.1.0-accepted-writer-upgrade-2026-10-08.md).
The explicit current trial and matching Clippy check exited zero.
Whole-application upgrade, historical application codec replay, rollback, and release admission remain open.

## Installed Astral and operational checkpoint — 2026-10-08

The isolated Astral frontend now installs the candidate Studio package without vendor aliases or private source imports.
The original application commit is `86dca6fa514895f94fd09ea788ebf07fa21cf648`.
Its production deployment remains unchanged.
The isolated consumer passed 44 browser cases across Chromium and WebKit, 77 unit tests, type checks and a production build.
Four negative browser cases reproduced private-view retention after HTTP 401/403 before the application correction.
The corrected consumer uses the public `SessionDeniedError` contract and retains views during transient failures.
See [the installed-source trial](rom-0.1.0-astral-installed-adoption-2026-10-08.md).
These fixture-backed frontend checks do not prove real-provider recovery or Rust package adoption.

A fresh complete application recovery matrix passed on SQLite and redb with real-provider login after restoration.
Measured RTO was 8,725 ms for SQLite and 7,043 ms for redb in the documented fixture.
Both cases deliberately lost 20 acknowledged writes made after the snapshot.
This is a measured warm-provider recovery profile, not a universal RPO/RTO promise.
The recovery report binds each result to its exact source snapshot and executable.

The accepted 0.0.3 Runtime now has an additional authentic custom-codec replay proof on both adapters.
Current Runtime replay preserves stored results and rejects conflicting input and revoked application policy.
The [independent review](rom-0.1.0-runtime-predecessor-independent-review-2026-10-08.md) found no blocker for this subset.
Codec-version migration, whole-application cutover/rollback and installed release artifacts remain open.

The diagnostics taxonomy corrections passed 13 maintained cases and scoped Clippy.
Public consumer drainage, final-source overhead and the final combined verifier remain open.
The historical full verifier predates the latest diagnostics and AI recovery changes.

AI recovery reproduced a default-stack overflow at public read-resume entry.
Narrow future boxing passed the exact default-stack reproducer without increasing thread stack size.
Current authorization, expiry, queued cancellation and remaining recovery scenarios still need acceptance.
The bounded load runner is still under construction; helper tests are not full workload measurements.

No 0.1.0 version bump, release tag, clean-source artifact admission or production deployment is claimed at this checkpoint.

## Actual package adoption checkpoint — 2026-10-08

The isolated Astral application now completes native checks against nine fresh, extracted ROM candidate packages.
Its actual all-target tests passed: 184 successful cases and one ignored case.
Application Clippy and the executable build passed. Existing Astrorust dependency warnings remain in the log.
The final dependency and input audits passed, and the executable hash was independently read.
See [the updated adoption report](rom-0.1.0-astral-installed-adoption-2026-10-08.md).
These archives use dirty candidate sources and the current `0.0.3` versions. They are not admitted 0.1.0 artifacts.
The actual packaged host, installed frontend and real provider still need a combined browser recovery check.

The public diagnostics consumer passed three actual native cases and scoped Clippy.
The cases cover correlated replay, full or closed readers, and exporter-task failure isolation.
See [the executed consumer report](rom-0.1.0-public-diagnostics-consumer-2026-10-08.md).
Installed release-package repetition and final-source overhead remain open.

AI read recovery passed four additional admission acknowledgement cases on SQLite and redb.
Two budget-exhaustion cases and two claimed-work acknowledgement cases reproduced failures on the actual adapters.
The budget cases require an actionable terminal projection without changing their frozen account or attempts.
The claimed-work cases require safe recovery of the original scheduled read, not a new ordinal or speculative action retry.
Corrections and their regression checks are in progress.

The load harness passed 17 maintained helper and provider-validity cases.
No provider setting was changed by these helper tests. Actual mixed HTTP traffic and injected faults remain unexecuted.
The complete source verifier, deployment checks and clean-source release admission remain required.

## Actual consumer recovery and current AI checkpoint, 2026-10-08

The packaged Astral application passed the four original recovery scenarios in Chromium and WebKit 2359.
Both trials used real Authentik login and the actual application binary. The original production deployment remained unchanged.
The [adoption report](rom-0.1.0-astral-installed-adoption-2026-10-08.md) records actual inputs, results and scope limits.
The [independent review](rom-0.1.0-astral-browser-preparation-review-2026-10-08.md) verified both source closures and cleanup.
These eight executions cover four scenarios, not eight distinct scenarios. They use loopback HTTP and dirty-source candidate packages.
Trusted TLS, deployment rollout and final clean-source release admission remain separate gates.

The AI checkpoint now includes safe read-result acknowledgement recovery and public read-progress projection.
Its preserved logs have 165 AI passes, 44 adapter passes and 17 external consumer passes.
The [AI independent review](rom-0.1.0-ai-read-progress-independent-review-2026-10-08.md) verified these logs and current source behavior.
Current authority checks precede read-progress disclosure. Physical work retains ownership after a caller timeout.
The public progress view distinguishes queued work, active work, uncertain activity and work awaiting recovery.
The two-second progress authorization bound does not cover arbitrary synchronous host policy callback execution.
These source tests do not replace final installed-package verification or original AI-flow adoption.

The optimized load fixture still failed the unchanged SQLite seed deadline.
It completed 1000 retained reactions before the next batch exhausted the 120-second preparation budget.
The corresponding redb comparison and detached-copy inspections are still in progress.
Neither a smaller seed nor a wider deadline establishes the originally selected workload.
