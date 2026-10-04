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
