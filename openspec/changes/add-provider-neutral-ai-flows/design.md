# Optional AI routing and durable flow design

Status: partial implementation. Task 1 passes 15 offline tests. Task 2 Resources, reservations, checkpoints and cancellation pass 14 shared SQLite/redb tests and coordinator integration review.
The latest complete AI check passes 131 cases, including typed tool flows and the supervised calculation wrapper.
The optional adapter passes 44 local HTTP, TLS and database flow cases with test-support enabled.
The earlier default-feature rerun executed only three wire cases; its log remains preserved as separate evidence.
Both AI crates and the standalone public consumers pass targeted all-target Clippy with warnings denied.
The standalone public consumers pass 17 cases for declarations, custom Field codecs, complete tool flows and interrupted recovery.
Current-grant, positive-cost settlement, actionable read failure and generation-limit cases use both actual database adapters.
The full verifier and original-consumer acceptance remain separate gates. Typed tool acceptance and external consumer recovery remain incomplete.
Queued discovery/grant failures now show sanitized failure; further admission and post-Prepare failure diagnosis remains under review.

## Intent and source basis

The owner wants reusable OpenRouter routing and ergonomic agent flows, while retaining one Resource model.
ROM core remains provider-neutral and driver-free. AI execution belongs to optional composition and adapters.

The [investigation](../../../docs/research/rom-0.1.0-ai-investigation.md) records inspected source identities and consumer defects.
The initial source basis is ROM `d7ef529040eec60dc869034c2d33130219db85fe` and Astral `86dca6fa514895f94fd09ea788ebf07fa21cf648`.
Recheck current dirty files and lock identity before implementation.

## Alternatives and choice

| Approach | Trade-off | Decision |
| --- | --- | --- |
| Document consumer code only | Few producer changes; every author still owns routing continuation and recovery. | Retain as migration reference, but insufficient for reusable flows. |
| Optional contracts and a narrow Resource facade | Provider adapter stays replaceable; existing Work supervises every tick. | Selected for offline implementation. |
| New general agent scheduler or adopted agent SDK | Adds another lifecycle, effect and authority model. | Rejected for this increment. |

Two crates are sufficient. `rom-ai` owns neutral types and `flow`, `tools`, and `routing` modules.
`rom-openrouter` depends on `rom-ai`, never the reverse.
Neither crate depends on SQLite or redb outside development fixtures.
`rom` does not depend on either crate.

## Module map

| Proposed file | Responsibility |
| --- | --- |
| `crates/rom-ai/src/lib.rs` | Module facade and public exports. |
| `request.rs`, `response.rs`, `error.rs` | Bounded messages, schema requirements, typed output and sanitized failures. |
| `provider.rs`, `clock.rs` | Object-safe async adapter and trusted time boundary. |
| `routing/policy.rs`, `routing/cursor.rs`, `routing/select.rs` | Frozen policy, run-local continuation and pure eligibility decisions. |
| `flow/resource.rs`, `flow/actions.rs`, `flow/codec.rs` | Resource storage, pure transitions, bounded versioned JSON codecs. |
| `flow/budget.rs`, `flow/reservation.rs` | Exact money and conservative account/run reservation actions. |
| `flow/host.rs`, `flow/builder.rs`, `flow/worker.rs` | Explicit Runtime binding, bounded host profile and one supervised tick. |
| `flow/projection.rs`, `flow/client.rs` | Authorized result view and original-command recovery. |
| `flow/operation.rs` | Bounded immutable operation admission facts; existing Runtime remains the receipt authority. |
| `tools/registry.rs`, `tools/read.rs`, `tools/action.rs` | Typed trusted registrations and effect-specific execution. |
| `telemetry.rs` | Bounded sanitized observation callback, disabled by default. |
| `crates/rom-openrouter/src/lib.rs` | Public facade. |
| `config.rs`, `credentials.rs`, `transport.rs` | Trusted endpoint/key boundary and bounded async HTTP. |
| `catalog.rs`, `request.rs`, `response.rs`, `errors.rs`, `reconcile.rs` | Catalog mapping, wire adaptation, typed errors and metadata lookup. |

Unit tests use separate named test modules or integration tests. No logic belongs in `lib.rs` or `mod.rs`.

## Neutral request and provider interfaces

The following signatures are proposed public contracts. `AiResult<T>` means `Result<T, AiError>`.
`AiFuture<'a,T>` is `Pin<Box<dyn Future<Output=AiResult<T>> + Send + 'a>>`.
`Value` is `serde_json::Value`; it does not imply trusted or validated content.

```rust
pub trait AiClock: Send + Sync {
    fn now_unix_ms(&self) -> u64;
}
pub trait Provider: Send + Sync {
    fn catalog<'a>(&'a self, deadline: Deadline) -> AiFuture<'a, CatalogSnapshot>;
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion>;
    fn reconcile<'a>(&'a self, attempt: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation>;
}
pub trait OutputValidator: Send + Sync {
    fn validate(&self, value: &Value) -> AiResult<Value>;
}
pub fn choose(policy: &RoutingPolicy, catalog: &CatalogSnapshot,
              cursor: &RouteCursor, request: &CompletionRequest) -> AiResult<RouteDecision>;
```

`CompletionRequest` contains ordered `Message` values, optional `OutputSchema`, registered tool descriptors, and input/output limits.
`Message` uses explicit roles: system, user, assistant, tool. `Message::assistant_calls(Vec<ToolCall>) -> AiResult<Message>` retains native call IDs, names and arguments; `Message::tool_result(id, content) -> Message` pairs each result with its call. Request validation rejects missing, duplicate or forged result identities and interleaved unfinished tool batches. Debug omits their contents.
`OutputSchema` contains version/name/schema; application validation remains decisive.
`Completion` is `Output { value, usage, evidence }` or `ToolCalls { calls, usage, evidence }`.
`ToolCall` has bounded call ID, registered name and JSON arguments. It cannot supply an actor or endpoint.
`Usage` has optional input/output/reasoning token counts and optional exact cost.
`AttemptEvidence` has opaque provider request/generation IDs when available, plus the durable attempt identity.
`Reconciliation` is `Accepted { completion }`, `NotAccepted`, or `Unresolved`. A lookup miss is unresolved.

`PreparedAttempt::new(...)` checks neutral contract coherence. `PreparedAttempt::validate() -> AiResult<()>` repeats these checks after decoding.
The selected model must occupy the cursor's preceding candidate index in the selected policy tier.
The cursor must share the policy version and tier, and must not reject the selected model.
The reservation estimate must equal the frozen tier ceiling for the exact request.
Paid attempts require the policy's budget reference. Remaining time must be positive and fit the expiry and policy age limit.
These checks cannot prove catalog capability, current time, Resource authority, or an actual budget reservation.
The flow and adapter must enforce those conditions before dispatch. Serde decoding alone permits no dispatch.

`AiError` includes `InvalidRequest`, `UnsupportedCapability`, `Denied`, `DeadlineExceeded`, `BudgetExhausted`, `InvalidOutput`, `RateLimited { retry_after_ms }`, `ProviderUnavailable`, `Conflict`, `UnknownOutcome`, `Closed`, and `Storage`.
Provider text, URLs and bodies are private diagnostics. Public failures contain sanitized categories and bounded opaque correlation IDs.

Default limits: prompt 32 KiB, output 64 KiB, tools 16, tool calls per response 8, tool arguments/results 16 KiB each.
Host policy may lower them. Raising them requires an explicit supported-profile fixture.
Reject invalid values before reservation or dispatch.

## Policy, time and routing

`RoutingPolicy` freezes version, ordered free/paid candidates, allowed providers, parameter requirements, disclosure policy and money limits.
Its initial finite limits are 8 generation attempts, 16 tool calls, 32 ticks, 300 seconds run age, and zero paid authority.
Paid work requires explicit positive caps and an account budget reference. Never infer paid permission from an available credential.

`RouteCursor` contains policy version, catalog identity, tier, next candidate index and rejected model IDs.
It belongs to one run and survives restart. Shared health caches cannot move another run's cursor.
A successful run never leaves a cursor for a later run to inherit.

`Deadline` contains absolute expiry in Unix milliseconds and the current remaining attempt duration.
`AiRun` retains expiry, last observed time, and remaining generation budget.
If time moves backwards below the last observation, fail closed with `DeadlineExceeded`.
Convert the current remaining duration to a monotonic Tokio deadline once per tick.
Include admission, discovery, credential resolution, connect, response body, validation and checkpoint persistence in the tick budget.
Do not divide useful generation time by the number of discovered models.

`choose` never performs I/O. It enforces schema/tool/modality/context requirements and exact rate ceilings.
Adapter-native model fallbacks are permitted only inside the prepared candidate envelope.
Application output rejection excludes the offending model within this run; native routing cannot validate domain claims.

## Resources, reservation and dispatch

Internal storage Resources are `AiRun` (`ai_runs`) and `AiBudget` (`ai_budgets`).
Their public definitions reject arbitrary create/replace/delete by non-service actors.
The facade performs host-authorized submission, cancellation and redacted reads.
Persisted private fields use bounded canonical JSON strings where ROM field types do not encode nested structures.
Typed public methods hide that codec; version 1 is strict and rejects unknown fields.

Implemented initial Resource APIs are `AiRun::queued`, `AiBudget::new`, their typed `record()` methods, and `definition_for(&rom::Actor)`.
Definitions freeze the configured service identity and reject deletion. Raw Resource reads require that service; owner views remain a facade responsibility.
`OwnerIdentity::from_actor` accepts a trusted host Actor. It validates bounded identity fields and preserves principal kind.
Service matching preserves authority, subject and principal namespace. ROM performs current actor-gate, revocation, expiry and revision checks.

`OwnerIdentity` contains the authenticated authority/subject reference and principal kind, without credentials.
`AiRun` stores owner identity, frozen request/policy, state, cursor, deadline, checkpoint sequence, consumed counters, active attempt, prepared tool commands, and validated output.
`RunState` is `Queued`, `Prepared`, `Executing`, `Waiting { retry_at_unix_ms: u64 }`, `AwaitingReconciliation`, `Completed`, `Failed`, `Cancelled`, or `CancelRequested`.
`AiBudget` stores account/window identity, configured limit, reserved total, settled total and bounded reservation entries.
Money uses `UsdNanos(u64)` and checked `u128` intermediates. Round worst-case estimates up. Never accumulate floating-point charges.
Route reservations use the frozen policy rate ceilings, including the request fee, rather than a possibly stale model catalog quote. Catalog prices establish eligibility; a budget reference still requires Resource authorization and reservation before dispatch.

An immutable reservation entry binds account/window, run, step, attempt ordinal, policy version and worst-case cost.
A matching replay does not reserve twice. Changed input under the same identity fails.
The initial `ReservationEntry` retains the complete validated `PreparedAttempt`, including exact request, schema/tools, policy, route and deadline.
Equality supplies exact replay matching without a new hashing dependency. No request text enters Debug or default telemetry.
Zero-cost free-tier attempts may reserve under a zero-limit internal account without a paid budget reference. They retain bounded attempt identities.
Limit the account ledger to 1024 retained entries and 256 KiB; fail `BudgetExhausted` at capacity.
Retention may remove settled entries only after the documented identity replay floor. Unknown reservations remain charged.

A tick first reserves account capacity through an expected-revision `AiBudget` action.
It then commits the run's `Prepared` attempt and dispatch identity through an expected-revision action.
These are two commits, so a crash can leave a conservative unused reservation. It cannot authorize an unreserved dispatch.
Reconcile that reservation through its original identity; do not reclaim it because a caller stopped waiting.
The run action freezes route, prompt fingerprint, schema/tool registry versions, deadline and reservation reference.
Before external I/O, commit `Executing`. Then dispatch only the frozen attempt.
The initial pure actions are `RESERVE_BUDGET`, `PREPARE_RUN`, `START_RUN`, `HOLD_RUN`, and `SETTLE_BUDGET`.
The current candidate also supplies `CHECKPOINT_RUN`, `CANCEL_RUN`, `FlowTick`, `FLOW_TICKS`, `RunHandle`, `RunMilestone` and authorized `RunView::project`.
`HoldRun::with_evidence` retains exact attempt/provider identifiers. Unknown outcomes cannot use the ordinary waiting transition.
`CheckpointRun::reconciled_not_accepted` requires trusted service confirmation of provider nonacceptance and the original attempt evidence.
It preserves consumed counters, expiry, evidence and account charge, and commits a bounded delayed wake through the existing channel.
Retry floors round milliseconds up to seconds. A rounded due time at or beyond expiry produces failure without successor work.
The decoded input is not proof of provider nonacceptance; the coordinator must authenticate that evidence and current owner authority.
Cancellation of an executing or unknown attempt preserves reconciliation and money. Late results cannot publish a completed output for a cancelled run.
`RunView::project` requires a current authorization callback before projecting output; its actor and serialized identities grant no authority.
Preparation binds exact run/request/policy/cursor/counters and reservation identity. The trusted coordinator must verify the committed account entry before preparation and execution.
A serialized reservation reference alone proves no account commit. Task 3 must use public authorized account reads and original receipt identities.

Cost settlement requires trusted usage evidence. Unknown usage keeps the worst-case reservation.
Actual cost is optional. No API claims exact accounting for a lost response with no provider generation ID.
Trusted settlement accepts actual cost only up to the reserved ceiling. Conflicting rewrites of settled usage fail; reservation replay keeps settled status.
Expired executions may checkpoint unknown evidence into reconciliation. This retains the original expiry and charge; it grants no retry or deadline extension.
Pricing decimal parsing is exact; malformed, negative, nonfinite, unknown or overflowing prices are ineligible.

## Ergonomic flow facade

```rust
pub struct FlowHost;
pub struct FlowBuilder;
pub struct FlowClient;
pub struct RunHandle(pub String);
pub struct Submission { pub id: String, pub idempotency: String,
                        pub request: CompletionRequest, pub policy: RoutingPolicy }
pub trait FlowAuthority: Send + Sync {
    fn submit(&self, actor: &rom::Actor, submission: &Submission) -> AiResult<OwnerIdentity>;
    fn inspect(&self, actor: &rom::Actor, owner: &OwnerIdentity) -> AiResult<()>;
    fn cancel(&self, actor: &rom::Actor, owner: &OwnerIdentity) -> AiResult<()>;
    fn resolve(&self, owner: &OwnerIdentity, reads: &mut dyn rom::AuthorizationRead)
        -> rom::Result<rom::Actor>;
    fn attempt(&self, actor: &rom::Actor, owner: &OwnerIdentity,
               prepared: &PreparedAttempt, reads: &mut dyn rom::AuthorizationRead)
        -> rom::Result<()>;
}
impl FlowHost {
    pub fn new(service: rom::Actor, provider: Arc<dyn Provider>,
               authority: Arc<dyn FlowAuthority>,
               validator: Arc<dyn OutputValidator>, clock: Arc<dyn AiClock>) -> AiResult<Self>;
    pub fn install(self, builder: rom::Builder) -> AiResult<FlowBuilder>;
}
impl FlowBuilder {
    pub fn limits(self, limits: rom::Limits) -> AiResult<Self>;
    pub fn build(self, storage: Arc<dyn rom::Storage>, pool: Arc<rayon::ThreadPool>)
        -> AiResult<(rom::Runtime, FlowClient)>;
}
impl FlowClient {
    pub async fn submit(&self, actor: &rom::Actor, submission: Submission) -> AiResult<RunHandle>;
    pub async fn view(&self, actor: &rom::Actor, run: &RunHandle) -> AiResult<RunView>;
    pub async fn resume(&self, actor: &rom::Actor, run: &RunHandle,
                        expected_revision: u64, idempotency: &str) -> AiResult<RunView>;
    pub async fn cancel(&self, actor: &rom::Actor, run: &RunHandle,
                        expected_revision: u64, idempotency: &str) -> AiResult<RunView>;
}
```

`FlowHost::install` registers Resources and a versioned channel in an owned FlowBuilder.
`FlowBuilder::build` delegates to the existing Builder, attaches the resulting Runtime, then returns Runtime and FlowClient.
The consumed builder prevents attaching a different Runtime or binding twice. No process-global Runtime slot is permitted.
Source inspection confirms `rom::Builder::build` registers storage and constructs Runtime; it does not start recovered workers.
`Runtime::start_work` delegates to `start_reactions`, whose spawn is the worker start boundary.
FlowBuilder attaches the constructed Runtime before exposing either handle. The host then calls `Runtime::start_work`.
A restored due item therefore cannot run before attachment through the supported construction path.
Binding wraps the constructed Runtime in `Arc<rom::Runtime>`. `FlowClient` owns that Arc.
The registered callback stores `Weak<rom::Runtime>` behind a runtime-local attachment cell.
Dropping the final FlowClient makes callback attachment unavailable without a Runtime ownership cycle or core API change.
Applications keep FlowClient alive while work runs; unavailable attachment returns a bounded closed outcome.

Submission commits a queued run and initial reaction Work under the original command identity.
The creation-only mapper requires Resource revision 1, `Queued` state and checkpoint 0.
Its action commits a private `queue_admitted` marker and the channel intent together. These are two durable Work stages.
The marker defaults to false for legacy private records. Admission changes only false to true; every other field stays equal, and the marker cannot later change.
An unchanged ledger action cannot emit the channel intent: ROM correctly rejects no-op external effects.
The actual SQLite/redb failure is retained in `/var/tmp/rom-ai-task3-noop-diagnostic2.log`; no generic invariant was relaxed.
The channel callback processes one bounded tick using current run state.
Its success action commits the checkpoint and the next intent together. Completion produces no further intent.
Duplicate tick delivery observes the stored step/receipt and does not dispatch another attempt.
The existing `Runtime::start_work` owns supervision. `FlowHost` creates no polling task or competing scheduler.

The Task 3 facade has no tool registry or tool actor grant. Task 5 must introduce that boundary separately.
Current attempt authorization runs inside bounded `Runtime::establish_actor` with trusted reads.
It checks the frozen owner, exact request, original budget reference, policy, model and ceiling before reservation and immediately before generation or lookup.
Catalog discovery can precede attempt authorization. Revoked-grant tests establish zero generation and lookup calls, not zero catalog requests.

`FlowHost::install` selects eight action and I/O slots and a 1 MiB command envelope, overriding the supplied Builder profile.
`FlowBuilder::limits` accepts explicit positive capacities up to 32 and command bytes up to 1 MiB; it never widens that explicit profile.
The shared provider semaphore uses the admitted I/O capacity and covers catalog, generation and manual reconciliation.
Eight manual lookup admissions bound callers waiting for that semaphore. Dropping a caller releases its permits.
Core `Overloaded` maps to the sanitized `BudgetExhausted` category; the one-slot fixture demonstrates a finite capacity rejection.
Lookup admission, current authority, proof reads, permit wait and HTTP share two seconds. Complete manual recovery has an 18-second bound.
The channel callback also has an 18-second bound, below its 20-second supervision timeout and 30-second lease.
Trusted synchronous policies and observers must finish within their documented bounds; a future timeout cannot preempt arbitrary callback code.

Private run records retain at most 32 operation stamps without pruning, within the existing 256 KiB codec bound.
Each stamp freezes requester, key, operation kind, original expected revision, checkpoint, admitted time and nonce.
Changed identity fails closed. A finished replay returns a freshly authorized current view, not a historical operation projection.
Pending reconciliation replay performs no lookup. A new explicit key admits another bounded, charged lookup.
Cancellation replay recovers the original command through Runtime's receipt, using its original revision, step, time and child identity.
Serialized stamps and owner identities provide no credentials.

Run evidence commits before account settlement. An interruption between these commits keeps the ceiling reserved or charged as unknown.
Settlement rechecks the account entry's exact key and complete immutable prepared attempt; settled status does not erase that proof.
Unknown and known-cost settlements use different stable command identities. Known cost cannot exceed the ceiling or replace conflicting known usage.
After a durable completed result or confirmed-nonaccepted waiting checkpoint, existing Work can retry settlement without generating another request.
Fresh owner, prepared-attempt and account proof precede that retry. Other started outcomes never authorize blind generation retry.
Separate real-adapter fixtures inject confirmed noncommit before settlement and unknown acknowledgement after it commits.
Restart and repeated delivery preserve the original cost, event count and generation count.

Expired or rejected results retain exact call evidence and authoritative usage through the existing hold action, without publishing completed output.
Cancelled late results retain evidence and cost while keeping output hidden. Manual recovery of restored executing work holds the original attempt before lookup.
Sanitized observations follow committed stages. Observer panic isolation does not make an arbitrary blocking observer safe.

Unknown and confirmed-rate-limit outcomes re-read the exact active prepared attempt and original committed account entry before writing.
They use the current revision, with one bounded CAS retry for a concurrent immutable operation stamp; they never repeat provider generation.
Unknown outcomes retain previous trusted evidence and usage. An already-held exact attempt remains unknown without another hold mutation.
Malformed completions and wrong-attempt evidence hold internally generated evidence and ignore supplied usage and references.
Valid requested-tool completions retain exact validated usage and provider identity through the held-state coordinator, without executing tools or publishing output.
Two declared-tool, paid-ceiling regressions first reproduced missing generation identity on the actual adapters.
They now preserve known cost, one generation and current budget authorization across restart.
Three response cases and three cancel/reconciliation race cases pass on both actual adapters.
The queued-owner test separately admits bounded capacity rejection during contention, then requires current denial on replay after the operations drain.

Task 3 does not yet establish an explicit model-rejection proof beyond confirmed rate limits and reconciliation results.
An unsupported-capability error alone remains conservative uncertainty; the adapter design must distinguish confirmed nonacceptance before advancing a rejected model.
These generic cases support AP-UX-007/008 durable progress and queue/resume, AP-UX-023/024 retained context and fresh execution, and AP-UX-027 routing policy.
AP-UX-015 source navigation and AP-UX-029 interpretation, grounding, prompts and answer quality remain consumer-owned acceptance.
The latest coordination audit tracks 47 source references and 33 maintained intake groups; withdrawn AP-UX-030/032 stay excluded.
AP-UX-033 guest invitations remain application-owned, with generic draft, authorization and idempotency seams only.

A confirmed rate limit commits `Waiting { retry_at_unix_ms }` plus exactly one delayed next intent in the same Resource checkpoint.
The old tick is then acknowledged. Existing Work automatically claims the new tick when its persisted due time arrives.
Use the generic delayed-channel prerequisite below. No consumer polling task or second scheduler is required.
The due tick rechecks run deadline, current authority and remaining budgets before reserving another provider attempt.
If Retry-After reaches or exceeds run expiry, fail with `DeadlineExceeded` instead of creating a useless delayed tick.
`FlowClient::resume` remains an explicit authorized recovery action for a stopped/reconciled run.
It must not bypass a waiting floor, cancelled state, unknown effect hold, consumed limits or run deadline.
Routine rate limiting does not require the application or owner to call resume.

A tick runs at most one generation request or one tool call. Prepared mutation commands execute separately from planning.
Run counters also bound callback-issued commands because a normal callback `Runtime::execute` starts its own causal root.
Do not claim ROM's original root budget automatically covers those commands.

The host profile sets delivery timeout 20 seconds, lease 30 seconds, and at least two action and I/O admissions.
It explicitly sets finite `rom::Limits::command_bytes` to 1 MiB for bounded JSON-string envelopes.
Core defaults remain unchanged at 16 KiB. Private run/account JSON stays bounded at 256 KiB before envelope encoding.
Keep channel timeout strictly below lease. Checkpoint actions must not starve behind the worker's own admission.
Tests must demonstrate bounded overload instead of deadlock. The profile bounds AI ticks, not every provider's useful latency.
A later longer profile requires measured support and explicit lease/timeout changes.

## Generic delayed-channel prerequisite

The root coordinator owns this generic prerequisite's spec and implementation. AI code consumes it after conformance passes.
It does not add model/provider concepts to core.

Inspected source establishes the existing scheduler:

- `crates/rom/src/reaction_work/model.rs::WorkRecord` already stores `due: u64` in Unix seconds.
- `reaction_work/ledger.rs::enqueue` currently initializes due from the causal start.
- `ledger.rs::update(Claim)` skips pending records while due is greater than the current Clock.
- `ledger.rs::update(Finish)` only computes fixed exponential retry delay; it accepts no requested wake time.
- `reaction_work/control.rs::control` currently schedules operator retries at control.now.
- `ledger.rs::prepare_restore` currently resets a recovered ordinary lease to due zero.
- `reactions/worker.rs` already wakes on Runtime changes or a 100-millisecond idle timer.
- SQLite `persistence.rs::reaction_update` and redb `storage.rs::reaction_update` transactionally use shared `StorageState::update_work`.

Recommended minimum public seam:

`Channel<P>::intent_at(self, payload: P, not_before_unix_seconds: u64) -> Intent`.

Add `not_before: Option<u64>` to frozen Intent and PendingWork with `serde(default, skip_serializing_if = "Option::is_none")`.
Existing `intent` remains immediate and serializes as before. Raw struct literal callers need the new field and migration notes.
Freeze timing with the intent, payload, definition and command identity. Changed replay timing is changed input.
Reject scheduled unversioned effects, arithmetic overflow, and a floor at or beyond causal start plus original max age before commit.
For a valid delayed notification, initial due is `max(cause.started_at, not_before.unwrap_or(0))`.
The waiting period consumes no claim, attempt or root work count.

Keep due at least the frozen floor during ordinary retry, operator retry, and lease restore.
Do not extend start time, lease policy, retry epoch, age, attempts or root budget.
Restore retains reconciliation holds for unknown started deliveries; scheduling cannot turn uncertainty into permission to send.
Current service/source authority still runs at actual delivery time.
Cancellation commits run state. When its delayed tick later runs, it observes Cancelled and acknowledges without provider dispatch.
The generic seam need not delete another component's work to implement cancellation.

The next eligible wake is the minimum pending due or expired-lease recovery boundary, subject to reconciliation/terminal states.
Initial support uses the existing worker's 100-millisecond idle wake to evaluate that persisted state.
Runtime change notifications wake earlier when new work arrives. No new Storage wake API, per-item timer or consumer loop is required.
Document that the timer bounds idle rechecks, not execution latency under admitted work, OS scheduling or provider delay.
An earliest-wake optimization is separate work; it must preserve change notification, shutdown and bounded fallback behavior.

Compatibility requires more than serde defaults. An old reader could ignore a frozen floor and operator/restore paths could dispatch early.
The coordinator must choose a new native/archive format fence (current inspected format is 8) or an equivalent enforced minimum-reader gate.
Do not claim safe downgrade merely because old pending records decode. Old data without a floor retains immediate behavior.
Preserve backup evidence and historical prototypes. Test upgraded archives and deliberate old-reader rejection on both adapters.

Required shared conformance: atomic checkpoint plus delayed intent; zero claims before due; exactly one claim at due;
restart before due; restore before due; operator retry cannot shorten floor; changed schedule identity mismatch;
backoff later than floor; denied current authority; cancelled run no dispatch; unknown delivery remains held;
root age/overflow/capacity rejection; delayed lifecycle capacity reservation; source/receipt/event rollback on rejected scheduling.
Verify the existing worker automatically picks due work, and a new immediate intent wakes its existing idle loop.

## Typed tools and effects

```rust
impl ToolRegistry {
    pub fn new(version: u32) -> AiResult<Self>;
    pub fn read<I: rom::Input, O: rom::Input, F>(&mut self,
        name: &str, description: &str, read: F) -> AiResult<()>
        where F: Fn(ReadContext, I) -> AiFuture<'static, O> + Send + Sync + 'static;
    pub fn action<R: rom::Resource, I: rom::Input>(&mut self,
        name: &str, description: &str, action: rom::Action<R, I>) -> AiResult<()>;
}
```

`ReadContext` exposes an authorized actor and restricted read operations. It must not expose `Runtime::execute`.
Its public methods are `read<R: Resource>(&self,id:&str)` and `query<R: Resource>(&self,query:&rom::Query<R>)`, returning authorized typed snapshots.
A read closure cannot be assumed pure merely because Rust permits it. Hosts must register bounded read-only functions without external effects.
CPU calculation uses the host's bounded blocking pool, not synchronous work on the async executor or within action validation.

An action tool uses the registered Resource kind/action, validated target ID, expected revision and typed input.
The model cannot choose a service principal or action name beyond the registry.
`FlowAuthority::tool_actor` supplies current trusted authority; ordinary Runtime checks remain decisive.
Persist `PreparedToolAction { call_id, target, action_version, expected_revision, input, idempotency, retry_epoch }` before invocation.
Freeze input and command identity through unknown outcomes. A retry resolves the original receipt.
Serialize tool calls in this increment. Bound loops and preserve native tool call IDs and result order.

Arbitrary HTTP, shell, code evaluation and non-Resource mutation tools are outside the first registry.
A future external effect tool needs a channel profile, receiver deduplication or trusted verifier, and separate acceptance cases.
Application publication remains an action or several explicitly staged actions. The facade promises no cross-Resource atomicity.

## Uncertainty, cancellation and disclosure

Use `ReconcileBeforeRetry` for provider dispatch ticks.
If a started request loses its reply, hold the run and Work for trusted reconciliation.
Without a generation ID or conclusive evidence, return `Unresolved`. Ordinary retry cannot bypass the hold.
Confirmed model rejection can prepare another attempt only with remaining authority, deadline, attempt count and a new reservation.
A provider completion stored locally can complete reconciliation without another generation.

Cancelling a queued/prepared run prevents dispatch and commits `Cancelled`.
Cancelling `Executing` commits `CancelRequested`; it does not assert the remote request stopped or cost disappeared.
Persist late authoritative evidence and retain reservation history. Expose unresolved effects as `AwaitingReconciliation` with `cancel_requested=true`.
Do not issue compensation automatically. Never restart a cancelled run under its old identity.

`RunView` exposes run ID, Resource revision, state, bounded milestones, counters, sanitized failure, cancellation flag, and authorized completed output.
It excludes prompts, raw tool arguments, service keys, credentials, budget account identities and provider raw bodies.
Every read and replay rechecks current `FlowAuthority` and Resource authority.
A browser cache or run handle is not an authorization credential.

Provisional token streams are outside initial implementation. The interface may later add an opt-in bounded ephemeral observer.
Committed milestones are Resource facts; generated narration and tokens are not events.
Existing live queries can observe projected run state. Disconnect does not cancel accepted work.

Telemetry uses the object-safe `FlowObserver: Send + Sync` trait with `fn observe(&self, observation: &FlowObservation)` with bounded run/step correlation, category, attempt count and elapsed time.
No prompt/output/tool argument/raw error enters default telemetry. Disabled observer is the default.
Observer panic or failure cannot roll back a commit or trigger another provider request.

## OpenRouter adapter and dependency selection

```rust
pub struct OpenRouter;
pub struct OpenRouterConfig { pub endpoint: String, pub credential_ref: String,
                              pub catalog_ttl_seconds: u64, pub response_bytes: usize }
pub trait CredentialSource: Send + Sync {
    fn resolve<'a>(&'a self, reference: &'a str) -> AiFuture<'a, SecretToken>;
}
impl OpenRouter {
    pub fn new(config: OpenRouterConfig, credentials: Arc<dyn CredentialSource>) -> AiResult<Self>;
}
```

Endpoint and credential references are host configuration, never model/client arguments.
Production endpoints require HTTPS, no URL user info/query/fragment, approved origin and no redirects/system proxy.
Loopback HTTP exists only in test configuration. Keys have redacted Debug and never enter Resource values.
Use manual serde body encoding and bounded chunk reads. Parse HTTP 200 error envelopes.
Construct the client with `retry(reqwest::retry::never())`, `no_proxy()`, and redirect policy `none()`.
Reqwest must not silently repeat a started billable request outside the durable attempt ledger.
Catalog cache uses per-endpoint refresh coordination, not one mutex held across all network requests.
Catalog refresh obeys the run deadline. Expired cached capability data cannot grant new paid authority.
Require configured schema/tool capabilities; preserve local output validation after wire schema adaptation.
Keep unsupported schema errors separate from invalid application output.

Reuse `reqwest = =0.13.5`, `default-features=false`, `features=[rustls]`, already present through rom-auth.
Reuse workspace Tokio/serde/serde_json/rayon and existing sha2 0.10.9. No AI SDK, schema engine or decimal dependency is selected.
The published v0.13.5 manifest and local registry source declare MSRV 1.85.0 and MIT OR Apache-2.0; ROM requires Rust 1.99.
See [versioned upstream manifest](https://github.com/seanmonstar/reqwest/blob/v0.13.5/Cargo.toml).

The lock contains rustls-webpki 0.103.15. [RUSTSEC-2026-0049](https://rustsec.org/advisories/RUSTSEC-2026-0049.html) is patched at 0.103.10.
This checks one identified advisory, not the complete transitive graph. No dependency audit was run here.
Before dependency edits, run the existing license/advisory gate with current primary advisory data and record the exact graph/features.
Reject a new dependency unless its MSRV, license, maintained source and applicable advisories have evidence.

Provider mapping uses official [routing](https://openrouter.ai/docs/guides/routing/provider-selection), [models](https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties), [schemas](https://openrouter.ai/docs/guides/features/structured-outputs), [tools](https://openrouter.ai/docs/guides/features/tool-calling), [errors](https://openrouter.ai/docs/api_reference/errors-and-debugging), and [generation metadata](https://openrouter.ai/docs/api/api-reference/generations/get-request-&-usage-metadata-for-a-generation).
Rate ceilings constrain eligible prices; they are not aggregate spending caps.
Provider metadata lookup requires a known ID and cannot certify lost-ID requests never executed.

## R13/R14 alignment and support limits

R13 should say actual cost accounting is conditional on authoritative usage evidence. Unknown costs retain reservations.
R13 should distinguish catalog eligibility, native model fallback and provider selection from application result validation.
R14 should include queued cancellation, started cancellation, original command identity, and channel timeout/lease/admission composition.
R14 prepare/publish tests must cover partial commits; Astral publication has two Resource commits.

Initial support is one Runtime owner per database, finite non-streaming generation and serialized typed Resource tools.
Confirmed rate limits resume automatically through persisted Work due times. Unknown started outcomes remain held for explicit trusted reconciliation.
Two unrelated public-interface consumers must pass offline acceptance on SQLite and redb.
No deployment, disaster recovery, human usability or live model quality is established by those tests.
