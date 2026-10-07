# ROM 0.1.0 core and production investigation

Date: 2026-10-07. This report records source inspection and proposed work. No build, test, deployment, live database, or credential operation ran.
Only this report was written. Historical evidence remains unchanged. No production-readiness claim follows from this investigation.

## Source identities and evidence

Producer `/root/ROM` HEAD: `d7ef529040eec60dc869034c2d33130219db85fe`.
Its observed dirty state contained three untracked research documents: the consumer feedback, source review, and research plan.
Consumer `/root/astral-plane` HEAD: `86dca6fa514895f94fd09ea788ebf07fa21cf648`; observed working tree clean.

| File | SHA-256 |
| --- | --- |
| Producer Cargo.lock | `a080b892d06dd30e65917a9a13715918cbd2b02fe64827f0334d7bde7005b441` |
| Consumer Cargo.lock | `4dd0bb900581925cd82ada55aa60b98a9f19481ace5e1186055a4c5b70c28c82` |
| Producer crates/rom/src/resource/definition.rs | `81a7cafd16475e3d1f0f14ee2ecd0f48c89303921f48b75e15fd6a3b1843c5fb` |
| Consumer vendor/rom/crates/rom/src/resource/definition.rs | `f38fad0a69d3c0a92470ad811a060e6ece015da0d3da29436b55bf4dbd2cf546` |

The accepted release remains the documented 0.0.3 source `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
The inspected producer revision is a different identity. Retained test reports do not verify that revision automatically.

Read foundations: `/root/ROM/docs/quality.md`, `writing.md`, `ai-development.md`, `release-support.md`, and the three 0.1.0 consumer research documents.
Read relevant Beskid lessons through `docs/research/beskid-compiler-lessons.md` and `beskid-quality-baseline.md`.
Those bounded earlier audits identify exact Beskid revisions and source files. Their builds were not reproduced here.
Read deployment implementations only in Astral Plane packaging, operations documentation, and ROM's existing NixOS preview module.
No remote deployment or production endpoint was contacted.

## Genuine missing behavior and existing capabilities

| Boundary | Source-backed finding | Recommended disposition |
| --- | --- | --- |
| Captured validation | `crates/rom/src/resource/definition.rs` stores a function pointer. The consumer stores an Arc callable. | Genuine authoring restriction. Upstream the narrow callable change after compatibility and two-Runtime acceptance. |
| Unknown mutation outcome | `execution/command.rs` persists a shared bundle and invalidates on success or Unknown. Receipt replay precedes new transition validation. | Preserve existing semantics. Add external composition evidence; do not add another receipt repository. |
| Client recovery | Studio `client/session.ts` freezes a command in a WeakMap and tracks pending/unknown/terminal state. `application/controller.ts` provides same-object retry. | Existing in-session recovery. Reload persistence and navigation-independent autosave composition need a public recipe or small helper. |
| Authorized projections | `projection.rs` checks current and historical field disclosure. `LiveProjected::changed` re-queries before acknowledgement. | Existing core capability. Add external public-view example and cache/invalidation acceptance. |
| Durable orchestration | Core reactions, channels, delivery profiles, and operator controls already persist work and budgets. | Consumer scheduler duplication is evidence of composition friction, not missing durable-work primitives. |
| Database recovery | rom-backup plus both native adapters already support bounded logical archives, fresh restore, fencing, and receipt preservation. | Complete application recovery is missing acceptance/composition; database backup is present. |
| Migration | Resource migration preserves historical receipt codecs and pending obligations; native/archive upgrades exist. | Exercise a populated external deployment. Keep application image rollback distinct from data restoration. |
| Operations | RuntimeStatus and authorized Work views provide bounded, payload-free state. | Reuse current surfaces. Improve author/operator entry points before expanding Work disclosure. |
| Correlation telemetry | No direct tracing instrumentation appeared in maintained ROM crate source/manifests examined. Tracing appears transitively and in earlier recommendations. | Correct R9's phrase “existing tracing.” Correlated timing and safe exporter composition are genuine implementation work. |

The public Studio control and packaging restrictions are separately confirmed in the existing source review.
They remain release work, although another work package should own their files.

## Exact seams and API risks

1. **Validator ownership:** change only the private callable alias/storage and the public `Definition::validate_transition` parameter in `resource/definition.rs`. Borrow the stored callable during invocation. Keep `rom::Definition` exports stable. Keep the registration contract and `execution/command.rs` ordering unchanged. Existing function pointers and non-capturing closures should compile through a generic Fn argument; method-function type assumptions need an external compatibility check. Do not generalize action or policy callbacks merely because their signatures also use function pointers.

2. **Validator contract:** execution invokes the validator under the commit gate, inside bounded supervised blocking work. It catches panic locally. A captured value does not establish determinism, termination, or non-reentrancy. Specify that validation reads one immutable dependency snapshot for each evaluation. Mutable catalog publication may change later admission; matching receipt replay must not revalidate an obsolete transition. A callback that takes an application lock can create lock-order deadlock if that application holds the lock while calling ROM. Document this explicitly. Do not add asynchronous I/O inside this boundary.

3. **Mutation recovery:** reusable code belongs beside `studio/src/lib/client/session.ts` and `mutation.ts`, with a public facade export if accepted. The application controller is navigation-aware; its `submit` writes `selected` after an epoch check, so it is not automatically a reusable autosave controller. Its prepared mutation is recognized only by a WeakMap entry. Persisting that object alone cannot survive reload. A public recipe can persist the exact Invocation using the wire codec, then call `prepare` with the unchanged request after authenticated recovery. Persistence must bind the saved request to the original principal; client generation is not a durable principal identifier. Clear sensitive state on logout without falsely claiming the server canceled accepted work. Avoid implicit localStorage adoption or persisting credentials.

4. **Projection:** retain generic `Runtime::{read_projected,invoke_projected,query_spec_projected,live_spec_projected}`. HTTP ETags/304 decisions belong in the HTTP/application adapter. A computed public progress Resource is ordinary application data; raw Work and operator receipts remain infrastructure metadata. Do not treat a catalog cache as authority. Test policy updates caused by related identity/configuration Resources, not only edits to the projected row.

5. **Workflow:** use `reactions/{worker,receipt}` and `channels/{registration,execution,supervision}` for durable orchestration. Keep application prepare/publish/follow-up stages as actions and Resources. Keep provider identity, payload, profile, and budgets frozen as current contracts require. Expose progress through a separately authorized Resource projection. Do not introduce an AI job entity, parallel scheduler, or universal multi-Resource transaction.

6. **Recovery:** rom-backup owns logical archive validation; native adapters own engine export/import; the host owns application manifests, blob inventory, configuration references, secret references, and deployment fencing. Preserve public `backup_to`, `restore_from`, `migrate_from`, and `replay_from` paths. Keep core driver-free. Archive encryption, retention, transfer, and key management need explicit host policy rather than an accidental core dependency.

7. **Telemetry:** instrument lifecycle admission/wait, command proposal/commit/replay, reaction attempt, channel delivery/reconciliation, and shutdown boundaries in named modules. Keep Runtime status exports stable. Prefer an optional host-installed subscriber/exporter or narrow observer boundary after measurement. Do not install global subscribers from ROM. Correlation identifiers belong in access-controlled spans/logs, not metric labels. Never log actor credentials, Resource values, frozen payloads, evidence references, or full fingerprints.

## Shared tests before release acceptance

Extend existing shared tests rather than create SQLite-only proofs. `tests/persistence/tests/transitions.rs` already covers every mutation path, old receipt replay, authority, panic, and normalized values.
Use its real SQLite/redb fixture or shared native fixture for these added cases:

- Build two distinct Runtime instances with separate databases and conflicting captured catalogs. Both register the same Resource kind. Mutate one catalog and assert independent acceptance and unchanged data/events in the other runtime.
- Compile named functions, function pointers, and non-capturing closures through public APIs. Compile-fail fixtures reject Rc capture, non-Sync state, and borrowed non-static captures for their intended reason.
- Exercise create, replace, patch, action, and delete with captured validation. Rejection and panic leave row revision, receipt count, journal, effects, and work unchanged. A later valid operation still succeeds.
- Commit, lose acknowledgement, change catalog, and replay the exact identity. Return the historical authorized outcome without rerunning transition validation. Changed input returns IdentityMismatch. Current revocation denies disclosure without representing the earlier commit as absent.
- Through actual HTTP and the public Studio client, preserve exact expected revision/key/input across lost acknowledgement, cancel, reload, view switch, stale revision, session expiry, and principal switch. Verify one event bundle on both adapters.
- Public projection fixtures cover anonymous field subsets, raw-job denial, related-policy changes, live invalidation, deletion, stale cached disclosure, and cache isolation across principals.
- Workflow fixtures restart before preparation commit, after it, before send, after provider acceptance, and before acknowledgement. Record stable IDs, payloads, attempts, age, and budgets. Assert a deduplicating receiver sees one publication; unresolved reconciliation never grants another send.

Existing anchor suites: `backup.rs`, `migration_runtime.rs`, `receipt_versions.rs`, `operator_runtime/*`, `operator_reconciliation/*`, `channels.rs`, and `studio_resilience/*`.
Source inspection of these suites establishes intended cases only. Every new record must identify executed revision, dirty patch hashes, lock hash, toolchain, command, and result.
Beskid lessons require negative fixtures to fail for the named reason, complete codec-value comparisons, and downstream feature isolation.

## Non-AI second consumer

Use a small equipment-maintenance portal. Its Resources represent Asset, Inspection, WorkOrder, Publication, and configuration.
A runtime-specific captured standards catalog validates inspection transitions. An inspector autosaves a form with photos.
A supervisor approves a report; a durable chain prepares an immutable public report, publishes it through a deduplicating receiver, and records follow-up.
Anonymous readers receive safe public fields. Internal repair notes and raw infrastructure Work remain denied.

This repeats validation, controls, mutation recovery, projections, attachments, durable delivery, and operations without Astral Plane concepts.
Use public Rust/Studio entries only, extracted source packages, separate SQLite/redb databases, and independent source identity.
Record every private import, vendor patch, hand-written retry/scheduler branch, and unclear diagnostic. The owner performs one bounded authoring task.

## Finite recovery and load experiments

These are proposed experiment envelopes, not service commitments. Record accepted workload/RPO/RTO targets before interpreting results as acceptance.

**Restore drill:** build 1,000 synthetic Resources, 100 retained receipts, 20 tombstones, 20 attachments, and pending/reconciling/stopped work.
Back up at a recorded time, perform ten later mutations, then restore into a fresh isolated host directory with matching binaries/assets.
Recover database, blob inventory/bytes, configuration and identity references. Reprovision isolated identity without copying fixture credentials into production.
Verify login, field authority, replay, attachment digests, journal/claim fencing, and resumption. Measure lost accepted changes and elapsed recovery.
Keep source, archive, manifest, restored database, logs, and failure artifacts. Repeat on both adapters. Reject a live or existing destination.

**Upgrade drill:** populate 0.0.3 with custom fields, old receipts, tombstones and unfinished work. Upgrade one representation version offline.
Retain historical codecs and validate every unfinished consumer contract. Start matching Rust/Studio versions and perform public-client acceptance.
Test incompatible old binaries as explicit refusals. Test prior-image rollback only where the data contract permits it; otherwise restore the retained backup.

**Load envelope:** 10,000 synthetic Resources, 32 concurrent clients, 32 live subscriptions, bounded queries of 50 rows, and fixed-size attachments.
Use two minutes warm-up, ten minutes mixed load, and two minutes drain/recovery for each adapter/profile. Run one host at a time.
Set request/work counts and deadlines in advance. Record p50/p95/p99 separately for reads, commits, caller wait, replay, stream lag, and delivery.
Record RSS, queue age, disk/WAL growth, denied/overloaded counts, and retained obligations. Abort at declared memory/disk/latency ceilings.

Use fresh isolated runs for a slow subscriber, unavailable receiver, full bounded disk image, and process interruption.
Limit injection to one condition per run and preserve the unperturbed baseline. Require bounded queues/admission and preserved exact recovery identities.
Process-exit evidence does not establish power-loss durability or prolonged soak behavior.

## Deployment lessons and official references

Astral Plane `packaging/check-backup.py` uses SQLite's online backup API and a fresh temporary integrity check.
It checks tables, not ROM replay, current authority, blobs, identity restoration, or host loss.
Its `docs/operations.md` also describes stopped config/data archives, separate Authentik backup, and preserving an image for rollback.
The container runs without root and with a read-only root filesystem. These are useful bounded deployment patterns, not ROM acceptance evidence.
ROM `infra/nixos/studio-host.nix` is a VPN preview with existing listeners/certificates. It does not establish a public production profile.

After source inspection, official documentation was read on 2026-10-07:

- [Rust Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html#thread-safety): shared ownership does not supply thread safety for captured data. Send/Sync bounds remain necessary.
- [SQLite backup API](https://sqlite.org/backup.html): online database copy is a database primitive. Inference: it cannot establish restoration of application components outside that database.
- [OpenTelemetry overview](https://opentelemetry.io/docs/specs/otel/overview/): instrumentation APIs and SDK/exporter implementation are separate layers. Inference: keep exporters optional and host-owned.

These references do not prove locked-version ROM behavior or select new dependencies.

## Work packages and acceptance order

1. Freeze inspected sources, patch identities, and requirement-to-evidence matrix. Correct telemetry's current-state description.
2. Implement captured validation in its narrow seam. Run real shared transition cases and downstream compatibility fixtures.
3. Implement and accept public Studio/notice changes under separate ownership. In parallel, specify exact mutation recovery composition.
4. Evaluate public recovery recipe, durable workflow, and projections with Astral Plane fixtures and the maintenance portal. Add only helpers demonstrated necessary.
5. Add safe timing/correlation instrumentation; verify exporter failure isolation and redaction before load/recovery evidence uses it.
6. Run isolated provider identity, full application restoration, and populated upgrade drills. Preserve evidence of failures and recovery.
7. Run finite load/fault envelopes on both supported adapters, then the owner's authoring task.
8. Perform independent standards/spec review, affected checks, downstream extracted-consumer checks, and the full local verifier.
9. Produce clean-source matching Rust/Studio artifacts and compatibility/support records. Publication and production deployment are separate actions.

No new multiwriter, shared-tenancy, power-loss, registry-distribution, universal identity-provider, or exactly-once external guarantee is established here.
