# ROM 0.0.2 resilience and integration audit

Date: 2026-10-03.
Status: source review of maintained release evidence. Proposed acceptance work follows below.

This audit does not rerun tests. Historical test results belong to their linked reports.
The accepted native release is `0.1.0-alpha.1`, from commit `37c4f87cb4bd5d43975ed6ce1e049630a41d4f55`.
The owner names the next release `0.0.2`. Its plan must state this version transition explicitly.

## Preserve the implemented contract

Resource remains the application entity. Actions mutate Resources through the shared core, and successful commits record events.
Studio consumes the same contract. It must not introduce private repositories, direct database writes, or alternative authorization rules.
HTTP stays an optional adapter. Studio does not move HTTP assumptions into the domain core.

| Guarantee already implemented | Source and evidence | Release implication |
| --- | --- | --- |
| Generic mutations enforce Resource transition rules | `crates/rom/src/resource/definition.rs`; `tests/persistence/tests/transitions.rs`; [transition results](resource-transition-validation-results.md) | Reuse the validator for patch, replace, delete, and custom actions. Do not repeat validation only in forms. |
| Receipt replay uses current disclosure authority | `crates/rom/src/replay.rs::replay_outcome`; [migration evidence](resource-migration-results.md) | Browser retry retains request identity. Revoked authority denies historical response disclosure. |
| Metadata requires explicit discovery grants | `crates/rom/src/discovery.rs`; `crates/rom-http/tests/loopback/discovery.rs` | Discovery is not a mutation or read grant. Hidden references remain hidden. |
| Live observations retain capacity and recheck actors | `crates/rom/src/query/subscription.rs`; [read contract](query-read-contract-results.md) | Navigation releases observations. Subsequent disclosure must use current authority. |
| ROM owns accepted work after caller cancellation | [Ownership](runtime-ownership-results.md); [provider deployment](provider-deployment-results.md) | Cancellation stops waiting. It does not authorize a second mutation or release unfinished execution capacity. |
| Unknown outcomes preserve exact retry identity | [Operator recovery](operator-recovery-results.md) | UI states distinguish confirmed failure, unknown outcome, and resolved receipt. |
| Query cost cannot grant semantic eligibility | `crates/rom/src/query_storage`; [measurements](maintained-query-cost-results.md) | Filters, sort, anchors, and authorization retain common semantics. |
| Maintenance preserves unfinished obligations | [Scenario map](framework-release-scenarios.md) | Close native ownership before migration, retention, or restore. |
| Artifacts bind complete profile and source | `scripts/release-artifacts/verification.mjs`; [completion audit](framework-release-completion.md) | Add Studio assets to the verified distribution. Preserve complete inventory validation. |

The shared transition hook already fixes the discovered generic-patch bypass.
Current authority checks already cover replay and discovery. These are regression obligations, not missing implementations.

## Remaining integration gaps

### P0: Authorized descriptors for generic action forms

`DiscoveredResource` returns field shapes and action names. Its source explicitly states that action input codecs remain opaque.
`Input` exposes encoding, decoding, and diagnostic member names. It has no public action input schema method.
`Definition::action` erases each typed action into a callback without a discoverable input descriptor.

Generic Resource forms can use current field shapes. Generic custom-action forms cannot infer inputs safely from action names.
The release needs a versioned, authorized action-input description from the same typed contract.
Manual implementations need an explicit route. Opaque inputs need an explicit UI state; Studio must not invent a schema.

Relevant existing files:

- `crates/rom/src/resource/fields.rs`: `Input` contract.
- `crates/rom/src/resource/definition.rs`: action registration and erased definitions.
- `crates/rom-derive/src/expansion.rs`: shared typed field generation.
- `crates/rom/src/discovery.rs`: bounded authorized metadata.
- `crates/rom-http/tests/loopback/discovery.rs`: HTTP metadata assertions.

Proposed tests compare derived and manual input descriptors with actual codecs.
Cover wire names, nullable and optional members, scalar inputs, no-input actions, custom fields, and opaque inputs.
Hidden fields and action schemas must remain undisclosed. Oversized metadata must fail without a partial catalog.
Metadata does not grant execution permission. Invalid values still fail in the core.

### P0: Browser identity is not the existing service-token profile

The real-provider fixture proves opaque client-credentials service tokens. It does not establish browser login or human sessions.
[Provider results](provider-deployment-results.md) explicitly preserve this limit.

Studio needs a host-owned browser session boundary or a separately accepted browser identity profile.
Introspection credentials and client secrets must never enter JavaScript, browser storage, generated assets, or client-visible metadata.
User, provider configuration, and application settings remain Resources. Their runtime implementation is unchanged.

Reuse host authentication where its contract applies.
Research and acceptance must establish session issuance, logout, expiration, current binding, and request-forgery protection.
These are transport-host obligations, not new domain entities.
Relevant files include `demo/src/provider_profile/authentication.rs`, `demo/src/provider_profile/serving.rs`, and `demo/provider-fixture/journey.mjs`.
New browser acceptance belongs beside the Studio host and browser tests, with separate file ownership.

### P0: Active work through actual process shutdown

Current binary tests deliver SIGINT and SIGTERM at readiness on both stores.
The attachment upload finishes before readiness. Provider signal tests have no accepted authentication job.
Paused authentication drain and paused blob publication have separate component tests.
These checks do not prove the combined active-operation signal journey.

Add finite subprocess tests that pause an accepted operation, then send SIGINT or SIGTERM.
Assert stopped intake, retained capacity, bounded completion, final native-owner release, and coherent reopen.
For uploads, assert committed readable content or the documented incomplete outcome. Never report incomplete bytes as a complete attachment.
For authentication, assert no stale binding or new admission after closure.
Use fixture-controlled pauses. Do not depend on timing sleeps or external provider delay.

Relevant existing files:

- `demo/src/host_signals.rs` and `demo/src/serving.rs`.
- `demo/src/provider_profile/serving.rs`.
- `demo/tests/server_lifecycle.rs` and `demo/tests/server_lifecycle/support.rs`.
- `demo/tests/provider_profile/serving.rs` and `demo/tests/provider_profile/commands.rs`.
- `crates/rom-blob/tests/lifecycle.rs`.

The [signal report](evidence/framework-release-2026-10-03/signal-results.md) is the precise baseline.
Process shutdown remains distinct from power loss and arbitrary Rust callback preemption.

### P1: One browser journey for authority, replay, and live results

Core and adapter tests establish many existing rules.
Studio adds browser state, cancellation, reconnects, and cached metadata. These need combined acceptance.

Use two different declared Resources without per-kind client controllers.
Discover authorized metadata, create data, invoke an action, and observe a filtered list update.
Lose the mutation response after commit. Recover with the same key and identical request.
Revoke User or provider binding. Require denial of replay, refreshed metadata, and subsequent live disclosure.
Stale browser content cannot become an authorized server response.
Change row ownership. Require the observed result to remove data that the actor can no longer read.

Reuse existing routes and wire correspondence checks. Browser memory is not the authoritative receipt store.
Relevant fixtures include `tests/persistence/tests/query_read.rs`, `tests/persistence/tests/transitions.rs`, and `crates/rom-http/tests/loopback/discovery.rs`.
Add browser tests to the selected Studio runner. Separate synthetic fixture identities from real-provider acceptance.

### P1: Provider identity through maintenance and recovery

The actual-provider journey and synthetic upgrade/operator journey are separate.
The completion audit rejects describing them as one provider-to-migration experiment.

Add one accepted journey on both stores: authenticate, mutate, close ownership, perform declared maintenance, reopen, and replay.
Preserve pending work, identity configuration, historical receipt interpretation, and current authority.
Revoke the link after reopen. Require denial of receipt disclosure.
Reuse the pinned fixture. This does not establish compatibility with every provider.

Relevant files include `demo/provider-fixture/journey.mjs`, `demo/tests/upgrade.rs`, and `demo/src/provider_profile/verification.rs`.
Use shared named helpers. Do not copy the complete provider fixture into another harness.

### P1: Direct tests for two qualified evidence gaps

The scenario map records no dedicated reentrant Storage-read policy test.
Add a bounded policy callback that reads native Storage. Run reference and eligible query paths on both stores.
Assert completion and unchanged authorization order. This directly tests the callback-free native lock boundary.
Relevant files are `tests/persistence/tests/query_read.rs` and `crates/rom/src/query_eval/read.rs`.

Native migration preservation of operator receipts combines shared-state inspection with direct archive, restore, and retention tests.
Add a native migration case seeded with operator receipts and reconciliation holds.
Assert exact replay, profile preservation, original budgets, and rejection of changed identities after reopen.
Relevant files are `tests/persistence/tests/schema_migration.rs`, `tests/persistence/tests/operator.rs`, and `crates/rom-backup/src/epoch_upgrade_tests.rs`.

### P0 release gate: Real Studio distribution

The previous release found missing copied configuration, excessive metadata output, incomplete manifest admission, and large-archive EPIPE.
The fixes already exist. Preserve their tests.

Extend extracted-consumer acceptance to include the real Studio build and host launch.
Serve assets under the configured path, including nested routes and browser refresh.
Verify complete asset inventory, production configuration, and source/lock identity.
Compiled assets must contain no credentials or development source-path dependencies.
Run browser smoke against the extracted distribution, not only the development server.

Relevant files include `scripts/packages/application.mjs`, `scripts/packages/application.test.mjs`, and `scripts/release-artifacts/verification.mjs`.
Release profiles must bind the Studio capability and complete packaged source/asset inventories.

## Minimal release order

1. Specify the authorized Studio descriptor and browser session boundary.
2. Build shared field renderers, Resource forms, query controls, and generic action forms.
3. Accept two Resources through one browser workflow on SQLite and redb.
4. Add active-work shutdown and the direct qualified-gap regressions.
5. Accept the combined identity, maintenance, and receipt-recovery journey.
6. Run full local gates and extracted Studio acceptance from clean source.

This order addresses demonstrated gaps. It needs no Studio-specific repositories or new query engine.
Keep CLI and optional HTTP consumers working. Keep module facades free of implementation logic.
Preserve historical reports, prototypes, failed runs, and release artifacts.

## Limits

This document reviews local source and accepted evidence. It introduces no external library recommendation.
The web research owner must compare browser authentication and Svelte strategies against primary sources.
The Studio owner must test accessibility, unsupported field behavior, and author extensibility.
Agent execution does not establish human usability or a measured productivity gain.

ASD-STE100 is used as a writing guide. This document does not claim certified compliance.
