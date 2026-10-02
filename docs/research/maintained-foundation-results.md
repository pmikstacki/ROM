# Maintained Resource foundation verification

Date: 2026-10-02. Runtime source after review correction: `6a7a19f7a211c1e2e59739382d0850ddb8425b58` on `codex/integrated-mvp`. This implements the first three packages of the [foundation plan](../../openspec/changes/integrate-resource-mvp/implementation-plan.md). It does not complete the broader integrated MVP, its authentication/field-projection requirements, durable worker, providers or transports.

## Source and verification boundaries

The maintained workspace starts from the tracked [integrated probe](integrated-core-probe-results.md), promoted as `4d3f10d`. Review fixes are separate in `042c4d1`; bounded I/O and runtime-owned drain are `c8c0381`; expiry is `d206876`; the live-delivery review correction is `6a7a19f`. These are source-branch identities, which may differ from later coordinator cherry-picks. All changes in this worker were confined to `/root/ROM/.worktrees/mvp-library`; no push or main-branch merge was performed.

The final native command passed against `6a7a19f`'s runtime source immediately before its commit:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/mvp-library && ./scripts/check'
```

Executed environment: `rustc 1.99.0 (b940084d7 2026-09-28)`, Cargo 1.99, `CARGO_BUILD_JOBS=2`, target directory `/workspace/ROM/.worktrees/mvp-library/target`, locked dependencies. The verifier explicitly checks Rust **1.99.0**, the declared initial tested floor; no older compiler claim follows.

The command passed four strict OpenSpec change validations, formatting, Clippy with warnings denied, **37 integration tests**, **one runnable doctest**, rustdoc with warnings denied, core `--no-default-features`, the two-Resource example, a driver/HTTP/derive-free normal core graph with derive disabled, five intended compiler failures checked at their primary source lines, and the separately locked renamed-dependency fixture. The 37 integration cases comprise 17 original shared-flow tests, one external custom-field/manual-definition test, three promotion regressions, four bounds tests and twelve lifecycle tests.

This worker did not perform the coordinator's separate advisory/license audit or extracted-archive packaged-consumer smoke. The maintained manifests now have versioned path dependencies, SPDX MIT metadata, descriptions and an actual copied MIT license in each library package. The example is a distinct downstream Cargo package; artifact packaging and human usability remain separately evidenced gates.

## Maintained layout and public interfaces

- `crates/rom`: public authoring facade and driver-free core. `resource.rs` owns field/codec/descriptors/actions/commands, `persistence.rs` owns semantic storage values, `execution.rs` owns admission/supervision, `query.rs` owns bounded reads/live handles, and `policy.rs` owns trusted actor/clock semantics. Core forbids unsafe code and denies unused must-use results.
- `crates/rom-derive`: descriptor, codec and typed field generation. Reexported through the default `derive` feature; manual definitions work with that feature disabled.
- `crates/rom-sqlite`: host-configured reference adapter. SQLite remains an implementation vehicle, not the only permitted production database.
- `examples/consumer`: Task and Setting, ordinary typed custom actions and a custom external Field implementation. No per-kind repository, controller or subscription infrastructure.
- `tests/compile`: isolated compiler diagnostics and renamed-dependency consumer; `scripts/check-rust` runs all Rust gates.

Existing `Resource`, `Field`, `Definition<R>`, `Action<R,I>`, `Command<R>`, `Storage`, `Runtime` and `Snapshot<R>` remain the semantic entry points. Intentional changes:

```rust,ignore
Runtime::read::<R>(&actor, id).await
Runtime::query(&actor, &query).await
Runtime::live(&actor, query).await
Live::changed().await
Runtime::shutdown().await

Builder::limits(Limits { ..Limits::default() })
Builder::clock(Arc<dyn Clock>)
Actor::trusted(authority, subject).expires_at(exclusive_unix_seconds)

trait Clock { fn now(&self) -> u64; } // Unix seconds; trusted, cheap host source
Storage::snapshot(kind, max_rows, max_bytes) -> Result<Vec<Row>>
```

Actor deliberately has no Deserialize implementation. Its optional expiry is host-established and does not change principal identity or durable idempotency scope. `SystemClock` is the default; a deterministic test clock controls all expiry cases. The clock is a trusted synchronous time source, not an authentication provider. A clock panic produces a terminal runtime error rather than granting authority.

## Executed corrections and guarantees

| Boundary | Evidence and result |
| --- | --- |
| Standard/custom action identity | A custom action named `delete` previously collided with the built-in delete's durable identity and returned its live row as a supposed deletion result. The regression failed on the promoted baseline, then passed using tagged `standard`/`custom` operation namespaces. |
| Revoked existence probe | Revoked reads previously returned Missing for absent ids but Denied for present ids. The regression failed, then passed after actor authority was checked before storage lookup. |
| Nested nullable codec | `Option<Option<bool>>` could lose `Some(None)` when encoded as null. The regression failed, then passed with explicit registration rejection of ambiguous nested nullable field shapes. This is an unsupported capability, not invented silent normalization. |
| Snapshot limits | SQLite caps candidate rows and counts serialized full Row bytes, with checked arithmetic before decoding/appending the next row. Exactly-at-limit succeeds; one extra row or byte fails with TooLarge. No truncated success is returned. Core also checks the adapter's returned rows defensively. |
| Live capacity | Every live handle owns a subscription permit; saturation rejects another handle and dropping a handle restores capacity. Shutdown invalidates/wakes observers; subsequent delivery returns Closed. |
| Tokio responsiveness | A synchronous adapter load pauses behind a gate while the single Tokio worker advances a heartbeat. Reads, queries and final response-authorization database reads are executed on bounded blocking jobs. |
| Cancellation accounting | Cancelling an observer of a blocked read leaves its I/O permit occupied; another read is overloaded until actual storage completion. Existing CPU caller-cancellation accounting still passes. |
| Shutdown ownership | Both cancelled-first-shutdown and two-simultaneous-shutdown tests failed against the old JoinSet-transfer implementation. They pass with runtime-owned active work and one shared drain condition. Admission and closure share a lifecycle lock. A separate admission/shutdown race leaves no orphaned accepted writes. |
| Expiry | Tests first failed with the Clock/Actor seam present but unenforced, then passed after checks at admission, pre-proposal, immediately before commit, and protected response/live delivery. An admitted write cannot commit after the tested expiry boundary; an expired response/live reader receives no protected rows. |
| Panic classification | A pure action panic returns an error without mutation. An adapter or policy panic produces terminal runtime failure, drains existing work, and never returns success. This does not prove rollback after a driver panic occurring after physical commit. |

The original atomic state/event/receipt/effect tests, lost-acknowledgment/reopen recovery, revision/idempotency races, no-op behavior, owner-change revocation, codec conformance and downstream-failure preservation all remain green after the asynchronous API change.

## Bounds, ownership and freshness profile

Defaults are host policy, not throughput claims: 8 actions, 8 blocking-I/O jobs, 64 subscriptions, 1,024 candidate snapshot rows, 1 MiB serialized snapshot bytes and 16 KiB command bytes. All must be nonzero. Input size includes encoded action/resource input plus resource and command identities. Existing effect limits remain eight intentions and a command-sized total payload budget. Durable history/receipt retention is still unbounded and requires the separate persistence package.

A blocking job owns its I/O permit and runtime active-work guard. Cancelling the caller drops interest, not that work. Each custom business function computes on the host's shared Rayon pool. In this first maintained implementation, the blocking action keeps its I/O permit while awaiting that CPU proposal; this is deliberately conservative accounting and can reduce I/O concurrency during long computations. It avoids SQL on Rayon workers but is not a measured scheduling optimum. A future split into individually admitted phases must preserve the same supervision and drain guarantees.

Every shutdown observer waits on runtime state, so cancelling one does not transfer or destroy ownership. Hosts still must explicitly await shutdown when they require a drain; dropping a handle is not a promise to wait. Once closed, new observations and actions are rejected. If closure occurs after a mutation committed but before its caller obtains disclosure, the observer can receive Closed instead of the protected result. Such observation errors are not proof of rollback; durable same-key resolution remains authoritative. A richer committed-but-undisclosable/unresolved outcome surface remains integration work.

The supported profile remains **one runtime owning writes and invalidation**. Protected reads capture a generation while holding the local authority/commit gate; if a resource transition or host revocation occurred before the async caller resumed, they refresh. After eight continually invalidated attempts the operation returns Overloaded rather than retrying without bound. Live handles retain only change signals and recompute an authorized bounded snapshot at delivery. Data already delivered to caller-owned values cannot be revoked retroactively.

Policy callback dependencies are limited by that profile: row state and explicit host revocation participate in the runtime's freshness mechanism. Arbitrary mutable external policy state read by native code is not automatically tracked; a future provider policy adapter needs an explicit freshness/invalidation contract. Cross-process writes likewise require a new tested protocol. Whole-row visibility is implemented; field projection and query-field authorization are not supplied by these foundation commits.

## Plan rulings and remaining work

Two execution decisions were recorded rather than hidden:

1. Cancellation-safe/concurrent drain was implemented together with the bounded I/O scheduler in Task 2, although listed under Task 3 as well. Both require the same ownership mechanism; splitting them would retain a known lost-drain defect temporarily. Task 3 then added expiry and remaining race/panic coverage.
2. I/O permits remain occupied during the associated Rayon proposal. This deliberately conservative policy preserves hard accounting bounds; its cost is potential throughput loss. No performance claim or permanent scheduler choice is made.

The coordinator owns the fresh whole-branch review before integrating these stages. No requirement is waived merely because the tests pass. Still required by the broader MVP are field-level policy/projection and trusted provider/User mapping; versioned inputs, PATCH/presence and wider fields/queries; durable bounded continuation workers and callback registration; receipt/journal retention and recovery budgets; shared SQLite/redb conformance; configuration/blob/notification integration; generic HTTP and journal transport profiles; completed dependency/packaging evidence; and a human author walkthrough. This foundation supplies the common library boundary for those packages rather than implementing competing per-resource engines.

## Fresh review correction

The coordinator’s independent review reproduced a P1 live-read bug in `d206876`: `Live::changed` consumed the initial/invalidation marker before awaiting its query, so Overloaded or cancellation could make the next call wait forever for a new mutation instead of delivering existing unobserved rows. Both new public-API regressions failed with timeouts on that source.

Commit `6a7a19f` separates watch notification tracking from the handle’s delivered generation. The latter advances only after successful query return, to the generation captured before that query. Failure or cancellation leaves an undelivered generation available for retry; an event during the query remains pending, allowing a harmless conservative refresh. The two reproductions now pass, and a third test protects the event-during-query edge. The full verifier passes with the revised 37-test count above. Coordinator reproduction of the corrected immutable source remains a separate review gate.
