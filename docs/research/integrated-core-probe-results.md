# Integrated typed Resource probe: results and remaining work

Date: 2026-10-02. Disposable branch `codex/prototype-integrated-core`; source in `prototypes/integrated-core`. This closes a specific composition question from the [readiness assessment](core-implementation-readiness.md) and [cohesion review](architecture-cohesion-review.md). It is an executable library experiment, not the ready production MVP requested subsequently. No production implementation task is marked complete.

## Observed result

Two distinct typed Resources, Task and Setting, now use one derived field/codec contract, fluent registration API and action pipeline. They also share a replaceable persistence trait, authorization boundary, live query implementation and work supervisor. The separate consumer crate needs no per-kind repository, controller, broadcaster or subscription implementation. Resource remains the only managed domain entity. Action/receipt/query/intent values are infrastructure. The Setting example uses the same Resource pipeline, consistent with users, identity-provider configuration and settings also being Resources. Those latter built-in kinds were not separately implemented here.

This is new integration evidence rather than the sum of earlier prototype test counts. The prior library used string fields and JSON business mutations; this consumer uses generated typed selectors and ordinary typed functions. The core has no concrete DB or transport dependency. SQLite supplies an actual atomic state/event/receipt/effect-intention transaction behind a driver-free semantic trait. SQLite remains an experimental reference adapter, not an owner-approved production selection.

The resulting API is materially closer to the premise. There is not yet evidence that humans find it pleasant. An agent wrote the example; no author walkthrough or editor study occurred.

## Reproduction and exact validation

From the host:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/integrated-core-probe/prototypes/integrated-core && ./verify'
```

The final verifier passed on native `rom-dev`: Rust **1.99.0** (`b940084d7`), Cargo **1.99.0**, Tokio **1.53.1**, Rayon **1.12.0**, rusqlite **0.40.2**, serde **1.0.229**, serde_json **1.0.151**, Syn **3.0.6**, Quote **1.0.47**, proc-macro2 **1.0.107**. Workspace and downstream fixture lockfiles retain transitive resolution. `CARGO_BUILD_JOBS=2` and a probe-owned target directory avoid interference with other experiments. This is the toolchain actually tested; no older compiler compatibility is claimed.

The command runs formatting, Clippy with warnings denied, **18 integration tests**, rustdoc with warnings denied, the application, a core normal-dependency isolation check, **five intentional compile failures**, and a separately locked renamed-dependency consumer. Compile-failure fixtures assert diagnostic text and the primary user-source line for unsupported field type, independent Serde annotation, duplicate public field name, wrong action input type and wrong selector value type. The crate documentation builds, but there are no executable documentation examples yet. The consumer is a real separate Cargo package with path dependencies, not a crates.io packaging/publishing proof.

The runnable application reports open membership `1`, then `0` after completion; the second Resource reaches revision `2`; final state/event/receipt/effect counts are `[2, 4, 4, 1]`.

| Executed scenario | Observation | What it establishes |
| --- | --- | --- |
| Task and Setting plus actions/live reads | Both use the same runtime, storage adapter and generated contract; Task completion removes it from the open query; Setting enable adds it to its query. | Resource-specific infrastructure is unnecessary for this bounded subset. |
| Codec agreement | Public rename is used by descriptor and codec; explicit false, zero, empty string and null survive; missing nullable field, wrong value type and unknown field are rejected. | One generated field list supplies actual executable normalization. No PATCH semantics are implied. |
| External field/manual definition | Consumer implements a validated Label newtype; invalid label fails before commit. A manual Resource with the same descriptor reads the persisted value through the same contract. | Native extensions can use public interfaces without macro internals; no second storage engine is introduced. |
| Concurrent revisions/identities | Different commands at one revision have one winner; same identity concurrently returns one durable outcome; changed input under an existing identity is rejected. | Shared runtime and native transaction preserve optimistic concurrency and durable idempotency. |
| No-op | Unchanged replacement keeps revision/event count and persists its receipt. | Retrying a confirmed no-op has a durable answer without fabricating a state transition. |
| Four actual transaction fault points | Failure after state, event, receipt or intention writes leaves none of the attempted bundle visible; retry succeeds. | SQLite transaction composition covers all four persistence parts. |
| Lost acknowledgment | Actual commit succeeds, adapter returns Unknown, close/reopen plus same-key retry resolves the original revision and intention. | Unknown is not flattened into rollback; durable identity resolves this injected ambiguity. |
| Authoritative policy race | A gated CPU proposal reads Alice-owned state; admin changes owner before it resumes; proposal is denied and does not overwrite the new state. | Pre-computation authorization cannot authorize a later authoritative revision. |
| Stale live/replay authority | Revocation before an initial live poll denies disclosure; row-owner change removes membership; old receipt replay is denied. | Current authorization applies to live delivery and stored outcomes in the single-owner topology. |
| Negative stale-buffer control | A previously authorized query snapshot remains readable if a deliberately flawed cache hands it out after revocation; a fresh protected query denies it. | Buffering old payload alone cannot enforce new authority; actual already-delivered data cannot be recalled. |
| Cancellation/drain | Cancelled caller leaves running CPU capacity occupied; another command is overloaded; shutdown remains pending until the gated worker exits, then the committed bundle is visible. | Actual work owns capacity and supervision; response waiter lifetime is independent. |
| CPU panic | Intentional business-function panic yields an error and no commit; capacity is restored after drain. | This limited unwind boundary works outside the authority lock; arbitrary poisoned adapter/policy state and process abort are not recovered. |
| Downstream failure/retry | Upstream Task and intent commit; downstream Setting write rolls back; upstream remains completed; same downstream identity retries and replays with one resulting Setting transition. | Atomic intent plus ordinary action retry can support the accepted chain failure default without multi-resource rollback. |

No throughput, latency, memory plateau, native power failure, distributed writer, broker or HTTP benchmark was performed. Reopen tests are normal close/reopen, not subprocess crash or power-loss evidence. Earlier independent storage probes retain their broader failure coverage; this report does not silently inherit it.

## Responsibility split and provisional implementation decisions

The derive produces descriptor, field encoding/decoding and typed field references. It rejects unsupported declarations at the source field. A separate Serde configuration cannot silently redefine the schema. The accepted descriptor is frozen with the definition; runtime normalization checks codec output against accepted names and shapes. Manual definitions traverse the same registration/capability checks. The current trait allows trusted manual code, so this is not proof against malicious native implementations.

Fluent registration combines policies and typed action tokens, rejects duplicate kind/action bindings and requires an adapter claiming atomic bundles, snapshots and effect intentions. Action tokens hold ordinary function pointers. Actions compute proposals, then ROM normalizes, reauthorizes against authoritative state, checks expected revision and commits. Admission and response delivery are separate stages. Each actual Rayon operation owns its permit; a Tokio supervisor retains completion even after caller cancellation.

The persistence trait transports semantic keys/rows/bundles rather than a SQL connection or transaction. The reference adapter protects conditional revision and durable identity checks with a native transaction. Effect intentions are in the same transaction as resource state, event and receipt. No separate outbox write can create a gap in this slice.

The **provisional topology is one runtime owning all writes**. Its authority gate serializes current reads, revocation, commit and live refresh. The SQLite adapter still conditionally arbitrates revisions, but another process writing the same file would bypass the runtime's local authorization/live freshness protocol. No multi-process claim is made.

Live subscriptions retain a coalescing generation signal, not old protected rows. Registration subscribes before the initial snapshot. Delivery recomputes current authorized rows. This is conservative and easy to reason about, but unbounded synchronous full scans are unsuitable as the final execution contract. The probe intentionally omits Salsa and completed-result caching; neither is necessary to prove the typed pipeline.

## Authoring critique and manual work still exposed

The external consumer declares field names once, uses typed business state, registers ordinary functions and asks for generated typed equality selectors. Adding Setting needs its declaration, policy and action registration; it needs no repository or event code. A validated custom field needs only its public Field implementation. Explicit idempotency identity and expected revision remain visible because they express caller intent, not hidden plumbing.

The biggest unfinished pieces are concrete:

- Action declaration repeats a Rust constant and a string operation name; inputs beyond scalars/unit require an Input implementation. A generated typed action/input layer remains untested. The derive does not build fields using Bon or typed-builder; prior builder research is not integrated just because the runtime API is fluent.
- Effects expose string channels and JSON payloads, and the continuation test manually translates an intention to a downstream command. A typed registered continuation should replace that glue. The eventual worker must submit the same action path under current service authority.
- Create/replace structs require all fields. There is no field patch builder, default initialization, absent/null/delete algebra, field-level write permission or friendly missing-revision/idempotency builder diagnostic. Those missing inputs currently become runtime errors.
- Public errors have categories and safe field/kind context for normalization, but storage errors lose useful diagnostic cause, operation context is incomplete, and typed input failures do not yet identify nested source paths.
- Field references internally contain a public name and are constructible manually; query input validation and descriptor-bound field identity need strengthening before public transport exposure.
- Host setup explicitly wires storage, shared pool and authorizer. This is appropriate once per host, but read/query APIs and shutdown semantics still expose rough implementation choices.

An initial compile run exposed a macro hygiene bug: generated `Result` resolved to the consumer's alias rather than `std::result::Result`. Fully qualifying generated result types fixed it. Clippy exposed generated selector dead-code warnings for a private consumer Resource; generated selectors now carry a narrowly scoped allowance. These are useful examples of work ROM must absorb before claiming a smooth author experience.

## Gaps that block direct promotion to ready MVP

1. **Bounded execution and I/O.** Mutation admission and payload/effect limits exist. Queries, snapshots, number of live subscriptions, retained durable receipts/events and row result bytes are unbounded. Public read/query and final response-authorization reads are synchronous and can block an async caller. A real async persistence boundary or bounded blocking-I/O service is required; CPU workers should not double as the whole database execution service. Shutdown needs cancellation safety, concurrent-caller semantics and explicit live termination. Hosts currently must call and await shutdown; dropping the runtime is not a complete drain contract.
2. **Authorization completeness.** Policies are whole-row function pointers plus host revocation. There is no actor expiry, tenant/isolation key, field projection, query-field permission, public discovery policy, external verifier or cross-process policy epoch. Descriptor inspection is trusted-host introspection. Delete replay returns a scalar tombstone outcome based on actor-scoped receipt plus host revocation; generalized retained authorization metadata remains open. Stored events/intents lack field-privacy filtering and journal delivery authorization.
3. **Durable chains.** The owner confirmed: preserve already committed upstream changes and retry the failed downstream action; bound chains to prevent loops/round trips; any compensation is a separate explicitly authorized action. The test covers only the lower-level transition and dedup semantics. There is no automatic worker, durable claim/ack, restart resumption, backoff, attempt/hop/elapsed budget, cycle handling, dead-letter inspection or provider dispatch. No unbounded recursion or multi-resource transaction is implied.
4. **Query/event contract.** Only scalar equality, implicit adapter id ordering and full live snapshots are implemented. No query AST version, stable pagination, ordering contract, joins, relation semantics, aggregates, journal cursor/retention or replay interface exists. Whole snapshots coalesce; they are not committed-event streams.
5. **Resource/codec surface.** Only String/bool/u64/nullable/custom scalar shape is covered. No schema migration, public compatibility negotiation, independent byte decoder with duplicate-key rejection, nested structures, default semantics or PATCH operation exists. JSON `Value` is an internal/provisional transport choice, not a selected universal wire protocol. Canonical identity fingerprints are whole normalized JSON strings, retained indefinitely; privacy and retention policy must be designed.
6. **Delivery certainty and proposals.** Concurrent same-key calls may compute the same pure proposal twice. Durable arbitration prevents duplicate commits, but native action functions must avoid direct external effects. No per-key single-flight is implemented. A write may commit while result disclosure fails; production responses should express committed-but-undisclosable outcomes clearly rather than leaving this as a plain Denied result.
7. **Packaging.** The facade depends on derive unconditionally. There is no independent macro-free core package acceptance, automatic dependency rename discovery, reexport fixture, feature matrix, published package build or MSRV policy. The tested explicit `crate` override handles the renamed downstream dependency. No unsafe is allowed in the core; this does not claim all dependencies contain no unsafe.

These are implementation gaps, not grounds for changing Resource-only authoring, Tokio/Rayon or provider-separated capability decisions.

## Focused remaining design inventory and recommendations

The following are proposals for the next milestone, not silently accepted final policies. They are grounded in this probe, existing OpenSpec, and the cited primary standards.

| Topic | Proposed next scoped contract | Decision/evidence still needed |
| --- | --- | --- |
| Initial fields | Freeze String, bool, signed/unsigned bounded integers, nullable values and validated scalar newtypes first; separately specify IDs, bytes, time/decimal and collection families. | Defaults, range/size limits, public codecs, comparison semantics, cross-language integer precision, secrecy annotations, custom-field compatibility. Probe only verifies String/bool/u64/nullable/custom String shape. |
| Missing/null/delete | Use a typed update algebra with absent = unchanged, explicit set(null) = nullable null, and a distinct removal/reset operation if fields can be absent/defaulted. | Decide whether persisted fields are always present and what reset means; implement vectors through derive/manual and actual transport decoding. |
| Relationships | Begin with a typed Resource reference whose target kind and key are explicit; define existence checks and delete behavior independently of foreign-key availability. | Restrict, retain tombstone/reference, explicit authorized cleanup action, or cascade policy; cross-resource consistency and privacy of target existence. No implicit multi-resource atomicity follows from callback chains. |
| Query AST | Start with validated typed equality/conjunction over permitted fields, explicit deterministic ordering with identity tie-breaker, and bounded pages. Live scope can initially be bounded unpaginated equality results. | Cursor scope and invalidation, snapshot consistency, null ordering, field permissions, bytes/rows limits. Defer joins and arbitrary user predicates until dependency and authorization semantics are defined. |
| Derive/manual/native plugins | One versioned accepted descriptor plus codecs/action bindings, frozen at registration. Keep extensions as trusted in-process Rust interfaces. | Versioned field capabilities, input-schema identity, codec conformance vectors, source-local diagnostics, manual malformed definition controls, feature negotiation and extension compatibility. A stable dynamic binary ABI is not established. |
| Rust/package compatibility | Explicitly choose an initial MSRV and test that exact compiler plus current stable; keep derive optional and adapters separate; add packaged consumer/renamed/reexport/feature checks. | This probe's Rust1.99 floor is evidence, not automatic owner selection. Declare API-breaking policy before a stable public facade. |

JSON Merge Patch assigns deletion semantics to null and specifically notes the limitation for documents using explicit nulls. Therefore it cannot be silently equated with ROM's required null-versus-removal distinction. JSON Patch gives remove and replace separate operations; ROM can borrow that distinction without adopting its entire path/operation surface. These are proposed consequences for ROM, not claims that either format was implemented here. [RFC 7396](https://www.rfc-editor.org/rfc/rfc7396), [RFC 6902](https://www.rfc-editor.org/rfc/rfc6902).

PostgreSQL's LIMIT/OFFSET documentation warns that limited subsets are unpredictable without a unique ordering. The proposed identity tie-breaker is a backend-neutral inference from that requirement, not a selection of PostgreSQL or a proof of cursor consistency under concurrent mutation. [PostgreSQL query limit documentation](https://www.postgresql.org/docs/current/queries-limit.html).

Cargo describes `rust-version` as the minimum supported compiler and gives guidance for compatibility expectations; its SemVer guide catalogs changes that can break downstream code. Thus the development compiler pin, tested minimum and promised public compatibility policy should be separate explicit records. [Cargo rust-version](https://doc.rust-lang.org/cargo/reference/rust-version.html), [Cargo SemVer compatibility](https://doc.rust-lang.org/cargo/reference/semver.html).

## Documentation alignment and estimate uncertainty

The [readiness assessment](core-implementation-readiness.md) and [cohesion review](architecture-cohesion-review.md) correctly describe the state before this experiment as separate typed-authoring/runtime probes. Their historical claim is now superseded by this specific integrated evidence, not by a production-complete core. The minimum scenario in the cohesion review also requested HTTP parity; that is still absent here. Existing [boundary requirements](../../openspec/changes/establish-rom/specs/boundary-adapters/spec.md) and the [transport review](transport-layer-review.md) remain broader than this library slice.

OpenSpec requires full standard operations, presence-aware updates, recoverable reactions and bounded lifecycle behavior; this experiment's replace-only updates, retained-but-undispatched intentions and unbounded reads do not fulfill those requirements. There is no contradiction in the fixed Resource premise; the mismatch is incomplete implementation scope. The new owner-confirmed chain failure and loop-bound decisions should be recorded by the coordinator without inventing numeric budgets or atomic rollback behavior.

The integrated slice reduces uncertainty around joining structural code generation, the shared mutation engine, atomic effect intentions and local live authorization. It does not provide a defensible calendar estimate for the ready MVP. The highest uncertainty is concentrated in bounded asynchronous reads, public policy/field projection and durable chain lifecycle. Derive/package hardening and a finite scalar PATCH contract are more bounded, but their human-facing ergonomics still need an author walkthrough. Estimate these as separate acceptance packages and measure review/fix effort during promotion; do not extrapolate from 18 tiny tests or agent wall-clock time.

A useful promotion sequence is: define the MVP query/field/topology limits; separate bounded read/I/O execution and robust shutdown; implement current actor/projection policy; implement durable bounded chain claims and downstream idempotency; connect the generic transport contract; then verify a real packaged downstream authoring walkthrough and restart/recovery scenarios. This report supplies evidence for that plan, not permission to omit any accepted MVP requirement.

## Coordinator verification

Independently rerun offline in native rom-dev on 2026-10-02 at source commit `86780041a9824c434054fae25509dff0e56a7212`: full verifier passed (18 tests, five compile-failure fixtures, renamed consumer, fmt, Clippy, rustdoc, runnable demo and dependency isolation). [Retained source](https://github.com/pmikstacki/ROM/tree/codex/prototype-integrated-core/prototypes/integrated-core). Independent promotion review is in progress; this does not complete the MVP.
