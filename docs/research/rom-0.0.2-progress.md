# ROM 0.0.2 execution record

Started: 2026-10-03. Updated: 2026-10-04. Status: active release goal.
Implemented slices have executed tests. Complete 0.0.2 artifact acceptance remains pending.

The owner authorized the full release without further approvals. Studio is now in scope.
The accepted native alpha and its evidence remain unchanged.

## Initial evidence

- Current frontend manifests and primary docs: `rom-0.0.2-studio-stack-research.md`.
- Browser authentication and dependency tradeoffs: `rom-0.0.2-browser-auth-research.md`.
- Existing guarantees and actual missing journeys: `rom-0.0.2-resilience-gap-audit.md`.
- Written design: `../superpowers/specs/2026-10-03-rom-0.0.2-studio-design.md`.
- Seven-task implementation plan: `../superpowers/plans/2026-10-03-rom-0.0.2-studio.md`.
- OpenSpec: `../../openspec/changes/release-rom-0-0-2-studio`.

Strict validation of the new OpenSpec change passed. `git diff --check` passed.
This is document validation, not a successful frontend build or browser acceptance.
The existing container reports Rust 1.99.0 and Node 22.16.0. The selected frontend graph still needs a locked executable trial.

## Design review and decisions

An independent source reviewer found five interface gaps in the first plan.
The amended design specifies sealed human identity proof, session stream cancellation, Resource-owned custom codec metadata, supervised upload intake, and bounded revocation detection.

Ruling: select static Svelte/Vite assets with an optional Rust host. This avoids a second backend runtime; complex routing could require later rework.
Ruling: preserve existing current-authority, mutation, receipt, ownership, and artifact fixes as regression requirements. Reimplementing them would add inconsistent paths.
Ruling: use the requested identifier 0.0.2 despite its ordering before the existing alpha. Consumers must use one exact matching package set.
Ruling: do not relax the advisory gate for an OIDC library. Trial typed OAuth exchange plus the current cryptographic backend before adoption.
Ruling: custom codec identity remains Resource-owned metadata. Independent Field registration remains outside this release.

## Immediate execution

1. Preserve the design/research snapshot in Git and establish an isolated release worktree.
2. Run the unchanged native baseline with the existing shared build target.
3. Implement and review Task 1 metadata before freezing browser fixtures.
4. Run frontend compatibility and authentication trials against the selected contract.
5. Continue through all seven tasks and independent release acceptance.

Parallel workers must have separate file ownership. Keep native and frontend build concurrency bounded.
Do not create redundant Cargo targets while shared disk capacity remains constrained.

## Implemented slices, before release acceptance

The isolated branch is `codex/rom-0.0.2-studio` in `.worktrees/release-query-index`.
The unchanged native baseline passed `./scripts/check` in `rom-dev` with the shared measured-verification target.

Task 1 is committed as `ba96fe7`. It supplies authorized action-input discovery and separate codec presentation metadata.
Six metadata cases, three native cases, actual HTTP fixtures, CLI compatibility, compile fixtures, and scoped Clippy passed.
A metadata-only reopen preserves catalogs, values, revisions, journal heads, and receipt replay on both stores.
The combined full verifier still belongs to the coordinator after concurrent source stabilizes.

The locked frontend trial passed installation, type checking, production build, Chromium controls, and dependency audit.
The product control worker reports eight Chromium cases; its final scoped verification and review remain pending.
WebKit needs its NixOS runtime libraries before it can be a successful browser gate.

The coordinator's first SDK slice passes 21 unit tests and zero-error Studio type checking.
It includes lossless wire values, descriptor validation, request-bound replies, explicit mutation retry, session generations, and bounded live snapshots.
It does not yet establish journal validation or real browser-to-host acceptance.
The human-provider fixture passes three real code/PKCE flow tests; Rust signature acceptance remains separate.
Independent review found malformed intent/shape, getter, signed-zero, and Unicode issues. The SDK now has regression tests for those findings.

The human OIDC proof worker has compiling sealed-profile integration and passing initial real-signature tests.
It is extending negative and bounded key-cache cases before its scoped commit.
The host and application workers will use these stable interfaces rather than a fabricated browser Actor.

These are tested slices. No 0.0.2 archive, durable preview, or completed release is claimed.

The OIDC proof implementation is committed as `0f3af31`.
Its scoped verifiers passed 38 tests, seven doctests, strict Clippy, docs, formatting, and feature isolation.
The generic control implementation is committed as `9871266` with eight Chromium cases.
The client journal extension is committed as `d1b564d`; the current client suite has 22 passing cases.

The first combined native verifier found one obsolete exact discovery fixture in the consumer example.
The expected catalog omitted the new authorized unit-action input descriptor.
The fixture now includes that descriptor; all 12 consumer discovery tests pass.
The failure is retained as `client/native-combined-red.log`, and the focused result as `client/native-discovery-green.log`.
The second full verifier is running. It is not yet accepted.

The next host contract is `rom-0.0.2-host-contract.md`.
The host must reverify finite server-side ID tokens through current Resource gates.
It cannot extend an existing Actor proof. A live observer closes at its proof lease;
the client revalidates the session and opens a fresh authorized snapshot.
The application and resilience workers continue in separate file scopes.

The second combined native `./scripts/check` completed with exit zero.
Its complete log is retained as `client/native-combined-final.log`.
This accepts the combined metadata and OIDC slices before the new host workspace member.
The host and persistence resilience test dependencies now resolve offline from the existing locked package versions.
Their implementations and final combined verifier remain pending.

## SDK, native anchors, and browser gates

Native query anchors are committed as `c86e9cb`. The actual TCP test preserves canonical `FiniteF64` values and verifies token rejection.
The route delegates to the existing runtime contract. It does not make the core depend on HTTP.

SDK resilience corrections are committed as `6a5fcc4`. The coordinator retained failing regression runs and a successful 48-test run.
The independent anchor worker subsequently reports 51 passing SDK tests and zero type errors or warnings.
Its scoped anchor commit and evidence remain separate from the coordinator's request fixes.

The repeated frontend gate passed 16 model/auth cases and 28 browser cases across Chromium and WebKit.
These browser cases use API fixtures. They do not establish browser-to-native-host acceptance.
The successful gate ran on the NixOS host, where its pinned browser downloads exist.
The initial container invocation failed because the container did not contain that browser download; its failure log is retained.

The native resilience worker completed both-store cases for current identity, reentrant policies, migration, operator receipt replay, and maintenance ordering.
Its Task 6 evidence remains distinct from accepted-work interruption through an actual serving process.

The optional host worker reports real upstream OIDC journeys on both stores, finite proof renewal, stream expiry, callback replay rejection, and logout closure.
Final scoped host verification and supervised BlobService routes remain in progress.
A separate worker is building the actual demo launcher and browser acceptance against production Studio assets.
It will register existing Resources plus a held-out custom-codec Resource without Resource-specific controllers.

No 0.0.2 release artifact, actual durable preview, or complete release acceptance exists yet.
The remaining gates include actual host/browser integration, accepted-work shutdown, package alignment, extracted artifacts, and persistent protected preview.

## Actual native Studio acceptance

The actual application slice is committed as `0034f07`.
Its four production-asset cases passed on SQLite and redb in Chromium and WebKit.
The gate built the current native source and preserved source inventories, the binary hash, and the exact supplied asset inventory.
It tested real upstream human OIDC, generic Resources, durable retry, moving pagination, authority changes, proof expiry, restart, and logout.
The held-out Resource uses the shared pipeline. It has no separate controller or repository.

The custom codec wrapper correction is committed as `6dcf27c`.
Native tests, discovery integration tests, and strict Clippy passed.
The latest coordinator SDK run passed 53 tests.
The worker's shared component/application suite passed 30 browser cases.
Registered optional and list custom renderers also passed actual-host browser cases.
Unknown custom codecs remain read-only, including optional values.

The supervised authenticated blob host slice is committed as `4817d14`.
It passed 26 host tests, 13 blob integration tests, one blob doctest, three provider tests, strict Clippy, and rustdoc.
A regression demonstrated that an authentication panic could skip the blob drain. The correction drains each accepted-work service before returning the terminal error.
Its real TCP shutdown test is not an operating-system SIGTERM test.
That process-level gate and generic attachment controls remain active work.

Evidence and explanations:

- [Actual native Studio](rom-0.0.2-actual-studio-host-results.md)
- [Custom codec wrappers](rom-0.0.2-wrapped-codec-results.md)
- [SDK acceptance](evidence/rom-0.0.2/client/wrapped-codec-sdk-green.log)
- [Host and blob acceptance](evidence/rom-0.0.2/task4/host-blob-final-4.log)
- [Protected preview plan](rom-0.0.2-preview-deployment-plan.md)

The package worker is adding extracted-asset acceptance and backward verification of the accepted historical manifest.
The root coordinator still owns version alignment, full-source acceptance, artifact production, deployment, and final source integration.

## Integrated preview profile and final client admission

The trusted HTTPS backchannel and provider deployment profile are committed as `882e7a8`.
The host admits explicit internal endpoints bound to the public issuer. It does not disable TLS verification.
The private demo launcher and query wrapper correction are committed as `5889905`.
The launcher keeps credentials outside source and shares the same Resource runtime on both stores.

The final loopback browser regression passed 16 actual-host cases on SQLite and redb across Chromium and WebKit.
This gate includes attachments, accepted-work SIGTERM drain, restart, and finite cost samples.
It does not prove the deployed HTTPS proxy or remote Mac access.
The final deployment gate remains pending.

The current SDK admission run passed 70 tests. Its type check reported zero errors and zero warnings.
The client accepts authorized input-descriptor subsets and rejects six native-impossible metadata forms.
Independent review of this final admission slice remains required.

Maintained package versions are 0.0.2. Known-advisory checks are recorded in `rom-0.0.2-dependency-check-results.md`.
The fixed eight-gate release producer is implemented, but the complete clean-source run remains pending.
The frontend notice collector and final integration review are active.
All native work reuses the shared measured-verification target. No new target is planned.

The integrated native run passed workspace tests, strict Clippy, rustdoc and the consumer example, then stopped at stale independent compile-fixture locks.
Both local lockfiles are now aligned. The focused compile verifier passed without changing external dependency versions.
See `rom-0.0.2-compile-lock-results.md`. The earlier complete command remains a failed run, not accepted evidence.

The actual SIGTERM restart test now verifies exact recovered attachment bytes on both stores and browser engines.
A controlled byte-corruption probe failed all four cases. Normal responses passed all four.
This source slice is committed as `8915f1d`. See `rom-0.0.2-attachment-lifecycle-results.md`.

The native provider-to-maintenance test retains both-store closed-owner, fresh-identity and current-replay evidence in `task6/native-final.log`.
The optional real-host login and session boundary is implemented and tested. These obligations do not prove deployment or MacBook reachability.

## Later acceptance closure: 2026-10-04

The full generic Task and Inventory browser flow passed on SQLite and redb in Chromium and WebKit.
Each Resource has declared actions, CRUD, queries and live observations through shared controls.
The observer receives changes from a second real page without extra query requests.
Task completion removes only that Task from the open list. The connected keyboard and accessibility scans passed under recorded settled-frame conditions.

Two-tab logout terminates live delivery within 34–38 milliseconds. Both views clear within the recorded periodic session interval.
The component suite now has 42 passing cases, including descriptor-version changes with retained drafts and explicit reopening.
The actual-provider maintenance journey passed backup, restore, schema migration, disk reopen, fresh identity and durable replay on both stores.

The native parser experiment captures twelve actual seeded responses. Its Node timings and small response sizes do not establish a universal performance bound.
This experiment ran after adoption. Independent review accepted a recorded plan-order deviation; the original wording and unmet chronology remain in the plan. Future replacements require comparative measurements before adoption.

An independent extracted-source Studio author example builds through the public source entry and registers a custom renderer without generic view changes.
Its four actual-provider browser cases passed. Independent review found verifier process ownership and copied-author fence gaps. The corrections passed another four-case run; an actual extracted-source mutation was then rejected after browser success. No findings remain in the scoped review.

These proofs are separate from the final producer. The complete high-growth release phase remains voluntarily deferred at the shared capacity checkpoint.
No clean final 0.0.2 artifact, persistent protected preview acceptance or main integration is claimed here.

The final package/artifact Node suite passed all 49 cases after the recorded host toolchain PATH was restored. The first run lacked rustc and failed environment admission; no product change resolved those failures. OpenSpec passed all ten strict items using the existing pinned official tool. An earlier incorrect npx invocation is retained separately. These are prerequisite checks, not a completed producer.

## Field editor and cross-browser follow-up: 2026-10-04

The selected Resource editor now shows descriptor-driven controls before a
mutation. Its field menu retains null, removal, and unchanged as separate
intents. Unsupported versioned codecs remain read-only. The mobile Sheet uses
the full viewport. The [field report](rom-studio-field-ux-proposal.md) contains
the mock, current screenshots, and exact patch checks.

The current frontend passed 49 component tests and 83 unit tests. A real-host
diagnostic passed 12 WebKit scenarios on SQLite and redb. Earlier Chromium
diagnostics passed 12 scenarios after the collection-control change. These
tests used the maintained native binary. The [new browser evidence](evidence/rom-0.0.2/field-ux/README.md)
records that limit. It does not replace the fresh native build in gate 8.

The [release completion record](rom-0.0.2-release-completion.md) now maps
current evidence to every remaining release obligation. The last bounded
producer failed gate 8; no accepted 0.0.2 artifact exists. The temporary VPN
preview serves the current frontend with an older native executable. A fresh
bounded producer and release-bound preview still require capacity admission.
