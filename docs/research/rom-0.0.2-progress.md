# ROM 0.0.2 execution record

Date: 2026-10-03. Status: active release goal. No 0.0.2 implementation or artifact acceptance is claimed.

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
