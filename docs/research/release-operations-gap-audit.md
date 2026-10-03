# Release operations gap audit

Date: 2026-10-03.

Scope: tasks 4.1–4.6 in [prepare-framework-release](../../openspec/changes/prepare-framework-release/tasks.md).
Method: inspection of maintained source, tests, scripts and documentation in `release-query-index`.
The inspected baseline is `9db4b2a`, with concurrent stage-three measurement work in the working tree.
This audit ran no tests, changed no product code and marked no release task complete.
Existing test names below identify inspected coverage, not new execution evidence.

## Summary

The repository has substantial operational foundations. Stage four is not a new core rewrite.
Its largest gaps are enforced deployment ownership, authorized operator work transitions, and one real identity/secrets deployment profile.
Packaging and recovery tooling already exist, but must accept the final combined release.

| Task | Implemented foundation | Remaining release work |
| --- | --- | --- |
| 4.1 | Runtime supervision, capacity bounds, durable claims, shutdown and process-exit fixtures | Enforce one Runtime owner and the native deployment ownership boundary. Integrate maintenance exclusion. |
| 4.2 | Generic Resource CLI, exact invocation retry, journal continuation, durable work ledger, domain compensation actions | Authorized work views and operator retry/reconciliation transitions, then transport and CLI commands. |
| 4.3 | Bounded JWT/introspection verifiers, identity Resources, explicit links, revision-bound identity gates | One documented real-provider profile, bootstrap procedure, secret loading/rotation and end-to-end deployment tests. |
| 4.4 | Public Rust traits, several versioned data contracts, maintained conformance tests and skills research | Publish an extension compatibility policy, reusable external conformance entry points and executable skills. |
| 4.5 | Public reference upgrade journey, full local verifier and extracted-package consumer checks | Run combined operator/identity/ownership acceptance from packaged sources on the final revision. |
| 4.6 | Local source archive script, checksums, MIT metadata and dependency notices | Final artifacts, current support/compatibility notes and a requirement-to-evidence completion audit. |

## 4.1 — Enforced ownership

### Existing implementation

[Storage](../../crates/rom/src/persistence.rs) states that one ROM owner controls an adapter.
[Runtime state](../../crates/rom/src/execution/state.rs) contains a commit gate, local generation, watch channel and worker semaphore.
[Lifecycle operations](../../crates/rom/src/execution/lifecycle.rs) track actual work and drain it before publishing stopped status.
[Reaction execution](../../crates/rom/src/reactions.rs) permits one started worker loop per Runtime.

SQLite serializes one connection through a per-handle mutex in [store.rs](../../crates/rom-sqlite/src/store.rs).
redb has a database handle and commit gate in [store.rs](../../crates/rom-redb/src/store.rs).
Engine transactions and work-claim generations provide different guarantees from Runtime ownership.
They do not share Runtime-local revocations, notification generation or wakeups between separately constructed Runtimes.

Inspected coverage includes:

- [runtime_reactions.rs](../../tests/persistence/tests/runtime_reactions.rs): one worker per Runtime, shutdown drain, cancelled batches, restart and lost acknowledgement.
- [shared.rs](../../tests/persistence/tests/shared.rs): rollback checkpoints, revision races, acknowledgement uncertainty and actual child-process exits.
- [consumer lifecycle tests](../../examples/consumer/tests/lifecycle.rs): public lifecycle behavior.
- [identity integration](../../crates/rom-identity/tests/integration.rs): current identity changes and an in-flight action race within the owning Runtime.

### Missing enforcement

[Builder::build](../../crates/rom/src/execution/builder.rs) accepts `Arc<dyn Storage>` without acquiring an exclusive Runtime ownership token.
SQLite's per-handle mutex does not prevent another adapter handle from opening the same database.
The redb open documentation explicitly leaves one-owner enforcement to the host.
No shared native ownership contract is represented in the inspected core port.

There is also a relevant test conflict to resolve deliberately.
[references.rs](../../tests/persistence/tests/references.rs) contains `independent_runtime_create_delete_race_preserves_integrity`.
It checks reference integrity under two Runtimes; it does not establish safe shared authorization or live observation.
A new ownership rule must retain the atomic race evidence at the appropriate adapter boundary, while rejecting unsupported Runtime sharing.
Do not simply remove the race assertion to make ownership tests pass.

### Smallest faithful implementation

1. Define an ownership acquisition contract at the storage/runtime boundary. Acquire ownership before accepting requests or changing registration state.
2. Keep its guard with the actual Runtime lifetime and accepted work. Dropping a caller future must not release ownership.
3. Add native exclusion for supported deployment paths before mutable engine setup. Cover separate handles and separate processes.
4. Make offline upgrade, migration, retention and rebuild reject an actively owned source.
5. Specify when a stopped Runtime releases its guard. Test that rule with retained clones and interrupted drain waiters.
6. Add deterministic parent/child tests for rejection, owner exit, restart, preserved pending work and subsequent successful ownership.

Avoid a PID file as the authority. A stale file does not prove a process still owns the database.
A process-released OS lock is a candidate implementation, subject to supported-platform and path-identity tests.
Symlink aliases, hard links and sidecar names need explicit treatment; path spelling alone is not a complete identity rule.

This is single-owner deployment enforcement, not distributed consensus or protection against arbitrary external SQL writes.
Keep the core independent of filesystem paths and database drivers.

## 4.2 — Operator work inspection and recovery

### Existing implementation

The [CLI command enum](../../crates/rom-cli/src/args.rs) exposes discovery, reads, queries, mutations, live queries and journal operations.
`invoke --request-file` can submit the original envelope with its identity, revision and retry epoch.
The CLI has no work inspection, work retry or reconciliation command.

[HTTP routes](../../crates/rom-http/src/server.rs) match that Resource/query/journal surface.
The adapter has no corresponding operator work endpoint.
[RuntimeStatus](../../crates/rom/src/execution/lifecycle.rs) is already a payload-free host snapshot, but is not a general operator work API.

[WorkRecord and WorkUpdate](../../crates/rom/src/reaction_work.rs) retain claims, attempts, generations, due times and delivery outcomes.
The current updates are claim, materialize, finish, delivery-started and delivery-finished operations.
There is no explicit operator transition for a stopped record or externally reconciled delivery.
`Storage::reaction_records` is trusted host inspection. It does not apply caller authorization or redact payloads.

Compensation already uses ordinary domain actions in [demo compensation](../../demo/src/compensation.rs).
The [reference journey](../../demo/src/reference.rs) and [upgrade tests](../../demo/tests/upgrade.rs) exercise pending compensation and recovery.
These examples do not provide a general operator reconciliation interface.

### Smallest faithful implementation

Implement the core operator contract before adding CLI syntax:

1. Add a bounded, authorized work view. Default output includes identifiers, state, attempts, cause and safe failure categories, not raw payloads or credentials.
2. Add explicit operator transitions with current authority and an expected work generation. Reject stale views atomically.
3. Define which stopped states permit retry. Preserve causal identity, original retry epoch and attempt history.
4. Resolve unknown Resource mutation outcomes through the existing receipt before any repeat attempt.
5. Give unknown external delivery a distinct reconciliation path. Require provider evidence or an explicit recorded operator decision; do not equate timeout with failure.
6. Record each accepted operator operation durably, with an idempotency key and attributable actor.
7. Expose these same core operations through an optional transport and the CLI. Keep HTTP out of the core contract.

A retry must not silently reset exhausted root, age, depth or attempt budgets.
If the operator may grant a new bounded allowance, specify that separately and record it without erasing history.
A worker holding an old claim must not finish a newly reconciled generation.

Compensation remains a registered Resource action chosen by application semantics.
The CLI can submit that declared action with explicit revision and identity.
Do not add a generic command that restores an old snapshot or claims to roll back an external effect.
Internal work records remain framework machinery around Resource operations, not a competing application entity model.

Suggested operator flow:

```text
inspect work -> inspect current Resource and receipt state
             -> retry an eligible unchanged intent
             -> reconcile an uncertain external delivery
             -> invoke a declared compensation action when required
             -> inspect the resulting work and Resource state
```

Required new tests cover denied inspection, redacted output, stale generations, duplicate operator requests, live worker races and restart after operator commit.
Reuse the existing [CLI failure](../../crates/rom-cli/tests/failures.rs) and [streaming](../../crates/rom-cli/tests/streaming.rs) fixtures for uncertain output and blocked terminals.
Do not expose `Storage::reaction_update` directly to remote callers.

## 4.3 — One real identity, bootstrap and secrets profile

[rom-auth](../../crates/rom-auth/README.md) provides bounded JWT and introspection verification profiles.
[rom-identity](../../crates/rom-identity/README.md) provides User, IdentityProvider and IdentityLink Resources, explicit bootstrap allow-lists and current revision checks.
Its inspected tests use signed synthetic tokens and local fixtures.

The demo [resolver](../../demo/src/identity.rs) explicitly accepts a synthetic local marker.
It is not real-provider authentication.
`IdentityProvider.credential_ref` names a secret; the host still owns resolution, rotation and redaction.
The current documentation explicitly leaves real-provider interoperability and first-administrator provisioning outside these tested profiles.

The smallest release addition is one pinned deployment profile:

- Select an actual provider and token profile compatible with the verifier, or add a separately reviewed bounded verifier profile.
- Define issuer, audience, trusted key acquisition or introspection endpoint, host time and revision-keyed verifier refresh.
- Provision the first administrator through an explicit host operation. Reject unauthenticated first-caller enrollment.
- Resolve secret references through one documented host mechanism. Test missing secrets and rotation without logging values.
- Show how external proof becomes an explicit User link and how disablement invalidates active access.
- Run real-provider acceptance for login evidence, expired proof, rotated configuration, disabled User, bootstrap replay and forbidden operator access.

Do not label a generated JWT fixture as real-provider interoperability.
A documented single profile is sufficient for this milestone; universal OIDC/provider compatibility is not required by the task.

## 4.4 — Extension contracts and executable skills

Several contracts already carry versions:

- Resource descriptors and legacy replay codecs in [resource.rs](../../crates/rom/src/resource.rs) and [replay.rs](../../crates/rom/src/replay.rs).
- Reaction definitions and Channel payloads in [reactions.rs](../../crates/rom/src/reactions.rs) and [channels.rs](../../crates/rom/src/channels.rs).
- Query semantics and encoding profiles in [query protocol](../../crates/rom/src/query_storage/protocol.rs).
- Native/archive format compatibility in the [maintenance documentation](../native-upgrade.md).

These do not constitute one published extension compatibility policy.
Native Rust modules use source composition; no stable dynamic Rust ABI is established.
The [skills report](rom-skills-library-research.md) proposes a catalog, but the inspected worktree contains no `SKILL.md` package.
The [storage conformance package](../../tests/persistence/Cargo.toml) and [external consumer](../../examples/consumer/Cargo.toml) are useful foundations, not yet a documented third-party certification interface.

The smallest completion path is:

1. Publish a compatibility table for Resource/codec, action input, reaction, channel, storage, blob, identity and transport extensions.
2. Distinguish package SemVer, persisted payload version and wire/profile version. Document rejection and upgrade behavior for each.
3. Extract genuinely shared public conformance fixtures from existing tests. Provide one executable external adapter/module example.
4. Ship a small initial skill set: Resource/action authoring, module/adapter authoring, operational diagnosis and release verification.
5. Bind each skill to a runnable public-API template and a verifier. Test an agent following it, including a deliberate invalid case.

Do not reproduce core contracts as independent skill prose when a stable source link suffices.
WASM research does not require a production WASM loader to complete native extension documentation unless the release scope explicitly adopts one.

## 4.5 — Combined acceptance and package verification

The repository already has:

- [Reference upgrade acceptance](../../demo/tests/upgrade.rs), including real process exit and migration/backup/restore.
- [scripts/check](../../scripts/check) and [check-rust](../../scripts/check-rust), including OpenSpec, formatting, Clippy, tests, docs, feature isolation and verifier scripts.
- [check-packages.mjs](../../scripts/check-packages.mjs), which extracts crate archives, checks their license files and builds a consumer from packaged paths.
- A packaged CLI help check and lockfile identity check in that package script.

These scripts are implementation evidence. This audit did not execute them.
Historical release reports identify older revisions and cannot certify the new stage-four integration.

After tasks 4.1–4.4, add one acceptance journey from extracted packages:

```text
explicit bootstrap -> real verified identity -> start owner -> create pending work
-> terminate process -> reject competing owner -> restart
-> inspect and recover through CLI -> migrate/backup/restore -> recheck authority
```

Preserve the existing external attachment and retained-receipt assertions.
Run the complete local verifier and package checks on the final reviewed tree.
Record source revision, lockfile identity, toolchain, commands and limitations.

## 4.6 — Artifacts, support notes and completion audit

[scripts/release](../../scripts/release) requires a clean checkout, runs checks/build/package verification and creates a no-overwrite source archive with a SHA-256 file.
It performs no registry or GitHub publication.
[Workspace metadata](../../Cargo.toml) still sets `publish = false` and an alpha version.
This is consistent with source distribution, but does not prove registry readiness or a stable release promise.

Complete this stage by producing artifacts from the accepted revision and documenting:

- Supported runtime, target platforms, single-owner deployment and storage/transport/provider profiles.
- Native/archive/Resource compatibility, upgrade paths, backup obligations and external blob handling.
- Current advisory review, dependency notices, known limits and support boundaries.
- A mapping from every release requirement and scenario to implementation, acceptance evidence or an explicit approved deferral.

Preserve historical reports. Add a current support summary instead of rewriting earlier limitations as if they never existed.
One concrete documentation drift is the `Redb::open` comment saying format four while current native storage uses format seven.
Resolve current API-documentation drift during the final compatibility pass.

## Recommended execution order

Start ownership and the operator core contract first. They constrain safe recovery and the real-provider operator profile.
Identity deployment work and extension/skills packaging can then proceed in parallel with the CLI adapter.
Finish with one combined packaged acceptance run, followed by artifacts and the requirement audit.

The previous instrumentation changes comply with the facade rule: their crate root and index facade contain declarations and exports only.
The shared Row reader removes duplication between observed and normal reads; no further instrumentation refactor is required by this audit.
Keep new ownership, administration, transport and CLI responsibilities in separate cohesive modules.
