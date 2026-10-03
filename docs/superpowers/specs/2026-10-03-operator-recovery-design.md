# Authorized operator work recovery

Date: 2026-10-03.
Status: proposed stage 4.2 design, based on current maintained source. No implementation or test execution is claimed.

## Goal and boundaries

Give an authorized operator a bounded way to inspect durable work, retry an unchanged intent and reconcile an uncertain outcome.
Keep compensation as an explicitly declared Resource action.
Use the same core operations from embedded Rust and optional transports; CLI syntax must not define recovery semantics.

Resource remains the only application entity.
Work records, claim tokens and operator receipts are infrastructure metadata, like existing mutation receipts and journal cursors.
They do not become freely mutable Resources or a second application model.
This slice adds no per-Resource controllers and no raw ledger endpoint.

A future built-in Resource may expose recovery requests through ordinary Resource views.
It must delegate to this atomic control contract and must not write ledger fields through generic patch operations.
That additional surface is not required for stage 4.2.

## Inspected foundations and gaps

- [`reaction_work.rs`](../../../crates/rom/src/reaction_work.rs) has bounded work, claims, attempts, root budgets and delivery outcomes.
- [`storage_state/work.rs`](../../../crates/rom/src/storage_state/work.rs) validates an update on a candidate state before publication.
- [`reactions.rs`](../../../crates/rom/src/reactions.rs) checks current service authority and consults durable receipts for frozen action invocations.
- [`channels.rs`](../../../crates/rom/src/channels.rs) supplies stable delivery IDs and records delivery start before calling the provider.
- [`execution/authority.rs`](../../../crates/rom/src/execution/authority.rs) checks current actor validity and identity gates through bounded reads.
- [`rom-cli`](../../../crates/rom-cli/src/args.rs) and [`rom-http`](../../../crates/rom-http/src/server.rs) expose Resource operations, but no operator work commands.

`Storage::reaction_records` is trusted host inspection. It returns raw frozen payloads and is unsuitable as a transport response.
`WorkRecord.generation` advances for claims and restore fencing, but not every state change.
For example, `DeliveryStarted` and `Finish` can change a record without changing that generation.
A generation-only operator CAS would therefore accept some stale views.

Current delivery semantics are explicitly at least once.
`DeliveryFinished` maps `Unknown`, `TimedOut` and `Retryable` to retry, subject to existing budgets.
Duplicates can occur unless the provider deduplicates stable IDs.
This is an existing accepted delivery profile, not evidence of a new correctness defect.

## Smallest complete scope

Implement four transport-neutral capabilities:

1. Inspect currently authorized, redacted work summaries and individual work status.
2. Schedule a retry of the unchanged frozen intent, within its existing budgets and execution identity.
3. Resolve a Resource action from its durable receipt, or reconcile an external delivery through a host-registered verifier.
4. Submit an existing domain compensation action through the ordinary invocation API and CLI.

Do not add generic rollback, payload editing, force-complete, arbitrary provider outcomes, claim stealing or budget reset.
Do not add distributed operator coordination; stage 4.1 supplies the supported single-owner boundary.

## Public core surface

Use a named `operator` module with separate protocol, authorization, runtime and ledger-transition modules.
The proposed public methods are:

```rust
Runtime::operator_capabilities(&Actor) -> async Result<OperatorCapabilities>
Runtime::work_list(&Actor, WorkQuery) -> async Result<WorkPage>
Runtime::work_read(&Actor, WorkHandle) -> async Result<WorkView>
Runtime::work_control(&Actor, WorkControlRequest) -> async Result<WorkControlResult>
```

`WorkControlRequest` contains a work handle, expected `WorkVersion`, explicit idempotency key, retry epoch and one operation.
The operations are `Retry` and `Reconcile { evidence_ref: Option<String> }`.
An evidence reference is a bounded opaque identifier, not an outcome asserted by the caller.
An omitted reference permits provider lookup by stable delivery ID or Resource receipt resolution.
Reject unknown request fields and oversized identifiers before submission.

The default operator policy denies every capability.
The host installs an `OperatorAuthorizer` that checks actor, operation and a redacted work scope using bounded `AuthorizationRead`.
The scope identifies the registered definition/version and affected Resource keys when available.
The callback receives no credentials or arbitrary frozen payload.
The host chooses who is an operator; ROM must not infer permission from a role name, principal kind or Resource discovery access.

Core checks current actor authority before inspection and before each control commit or receipt disclosure.
List authorization precedes disclosure and pagination of visible records.
Control authorization is separate from inspection permission.
The service actor recorded in the frozen work still executes the action or delivery; the operator does not impersonate it.
Existing current Resource, field and service authority checks remain in that execution path.

Application authorization and verifier callbacks run outside native mutexes and transactions.
Core may use its existing commit gate for short current-authority checks and commits.
No network call or provider verifier may hold that gate.
After a verifier returns, recheck current authority and the exact work version before accepting its result.

## Bounded views

`WorkView` includes an opaque work handle, work version, category, definition/version, safe state, attempts, due time and delivery status.
It can include authorized source/target keys, but never the frozen Resource values or action input.
Do not disclose raw causal root identities, service principal keys, receipt fingerprints, channel payloads or protected metadata.
Render structured failure categories; do not return callback panic text or provider error bodies.

Derive a stable opaque handle from a domain-separated hash of the internal work ID.
Resolve it inside the bounded ledger; do not accept it as an authorization credential.
Detect duplicate handles within a returned snapshot and reject corruption.

`WorkQuery` supports state/category/definition filters, an explicit page limit and an opaque continuation cursor.
Use moving pagination, not a retained server snapshot.
The cursor binds the storage generation, filter and ordering; it grants no permission.
If restore or maintenance changes that generation, return `HistoryGap` rather than restarting silently.
Recheck authorization for every page.

Charge the bounded raw ledger before projection, then charge each full public response before returning it.
Do not truncate JSON or apply the visible page limit before authorization.
For this slice, scanning the existing bounded ledger is sufficient; a second work-index subsystem is unnecessary.
Default response limits are 64 records and 64 KiB, configurable by the host under checked arithmetic.
An individually oversized result returns `TooLarge` without partial disclosure.

## Record versions and atomic controls

Add a monotonic `revision: u64` to `WorkRecord`, independent of claim generation.
Increment it on every accepted change to state, attempts, generation, due time or delivery outcome.
Use checked arithmetic; overflow rejects the entire candidate without publication.
The public `WorkVersion` contains storage generation and work revision.
Use the existing `StorageState` generation, which changes on restore, through a bounded storage response.
Do not expose a hash of frozen secret payloads as the CAS token.

The adapter's atomic control transaction must compare the exact version and validate the requested transition.
It commits the work change and a durable operator receipt together.
A control that invalidates an old claim also advances claim generation.
No stale worker can finish the newly controlled record.
Active `Leased` work is not eligible for manual retry or reconciliation, even if an operator believes it stalled.
Use the existing lease recovery path first; do not use the operator API to steal an external call in progress.

An operator receipt records the principal namespace, request identity, exact request fingerprint, target handle, operation, time and redacted result.
Replay with the same principal/key and identical request returns that receipt after current authorization.
The same key with changed input returns `IdentityMismatch`.
A lost acknowledgement produces an unknown outcome; replay the exact request rather than creating a new key.
Replay arbitration precedes expected-version comparison so the original successful request survives its own version change.

Operator receipts are bounded infrastructure audit records in `StorageState`, not Resource success events.
Default admission is 1024 receipts and 1 MiB of serialized receipt data, checked atomically before a control commits.
At capacity, return `Overloaded`; never evict evidence silently to admit another control.
Retain them through backup, restore and maintenance in this slice.
Apply existing retry epoch admission/replay floors to operator identities too; do not invent an unbounded second retry namespace.
Automatic pruning or a new budget allowance requires a separate explicit retention design.

Both native adapters use the same pure ledger transition code within their existing atomic state transaction.
Keep the new storage control methods unsupported by default for custom adapters.
Do not emulate atomic control with a public read followed by an unconditional write.
The public trusted adapter port uses `StorageWorkSnapshot` and `StorageWorkControl`, not transport envelopes.
Expose these contract types for custom adapters, but do not deserialize verified transition evidence from client input.
Use `Storage::work_snapshot(max_records, max_bytes)` for a coherent bounded view and `Storage::control_work(&StorageWorkControl)` for the atomic transition.
The snapshot carries storage generation, bounded records and trusted eligibility metadata; only core creates public projections.
Reserve maximum numeric widths for the new revision in existing lifecycle capacity accounting.
Receipt admission must not consume the capacity already reserved for a worker to finish its accepted claim.

## Retry eligibility

`Retry` changes scheduling; it never calls a Resource action or provider inside the control transaction.
The ordinary supervised worker executes the intent afterward.
Preserve payload, work ID, causal root, start time, definition/version, service identity and original retry epoch.
Preserve all used attempts and root budget counts.

Pending work can become due now if current authority and budgets permit.
A stopped record can return to Pending only when its original definition is available and the host authorizes retry of that category.
Budget exhaustion, expired chain age, depth and fanout limits remain hard stops.
Retry does not extend a chain's age or erase the failure history.
The next actual claim consumes the ordinary attempt and root budgets; an operator scheduling request does not fabricate a completed attempt.

For frozen Resource actions, check the original durable receipt before any new execution.
If the receipt confirms commit, reconcile to Done without a new Resource mutation, event or notification.
Receipt absence permits the existing atomic idempotent execution path; it does not justify a new identity or altered expected revision.
An expired retry identity returns `IdentityExpired`, rather than starting a new chain.
If changed domain state makes the frozen request invalid, the author must use a separate ordinary action with its own explicit intent.

## External delivery profiles and reconciliation

Keep delivery guarantee and retry policy distinct.
The host declares the channel's policy; a transport capability does not grant operator authority.

| Profile | Unknown or timed-out delivery | Manual reconciliation |
| --- | --- | --- |
| Existing at-least-once | Retry within existing budgets; duplicate risk remains explicit | A registered verifier can establish a terminal result |
| Provider-deduplicated | Retry with the same stable delivery ID under the declared provider contract | Verify by that same provider identity |
| Reconcile-before-retry | Hold without resending until an authoritative result exists | Accepted completes; terminal NotAccepted can permit unchanged retry |

Preserve the existing at-least-once default for compatibility in this slice.
Do not silently change every unknown outcome into a hold.
Changing the framework's default production delivery policy remains a product decision, not an implementation consequence of adding operator tools.

For the opt-in hold profile, add an explicit persisted awaiting-reconciliation state.
On timeout, unknown outcome or restart after delivery-start, the worker must not send again until the profile permits it.
Fence old claims when entering the hold state.
Declare and validate the profile with channel identity/version; changing a persisted work profile must not reinterpret old delivery evidence silently.

A host-registered verifier receives the stable delivery ID and bounded evidence reference.
It returns `Accepted`, `NotAccepted` or `Unresolved` with a non-secret evidence identifier suitable for audit.
Use bounded timeout, admission and supervision, and sanitize verifier errors.
Never let request JSON claim a trusted verification result.

`Accepted` marks the work Done without sending.
`NotAccepted` means the provider guarantees the old attempt cannot later become accepted; an eventual-consistency lookup miss is insufficient.
That result permits Pending only if the host's retry profile and original budgets permit it.
Otherwise keep the record stopped with its established delivery result.
`Unresolved` performs no state change and returns an explicit unresolved result; it grants no resend permission.
A later verification attempt uses a new operator request key. Exact retries of an accepted operation use the original key.

Providers without an authoritative verifier can still use their declared at-least-once or deduplicated profile.
For a hold profile without usable verification, the API reports that reconciliation is unavailable.
This slice offers no universal `force accepted`, `force not accepted` or `ignore duplicate risk` switch.
Whether to add audited human attestations later is a separate product/security decision.

## Compensation and transport

Compensation stays an ordinary action on a Resource, with its normal auth, expected revision, input and idempotency key.
The CLI already has the generic `action`/`invoke` mechanism; document a recovery journey using it.
Do not infer a compensating action from work state or automatically restore an earlier snapshot.
Compensation can itself emit events and create durable work through the existing pipeline.

Add optional HTTP endpoints for operator capabilities, work list, work read and work control.
They call the public core methods and use the existing verified-actor resolver, body limits and response handling.
Do not serialize `WorkRecord` or expose `Storage::reaction_update`.
Keep Resource discovery unchanged; operator capability discovery is a distinct administrative contract.

Add CLI commands `work capabilities`, `work list`, `work show`, `work retry` and `work reconcile`.
Mutation commands require an exact request file with work version, identity and operation, so retries reproduce the original envelope.
No implicit retry, automatic budget reset, inline credential, hidden provider call or automatic compensation is allowed.
Keep existing uncertain-response and Ctrl-C behavior, including blocked output pipes.
Validate response operation identity, target handle, protocol version and expected result shape before showing success.

## Persistence compatibility

New record revisions, delivery profiles/hold states and operator receipts change persisted semantics.
Older readers can discard unknown metadata or misinterpret replay eligibility.
Bump native and archive format versions together; do not rely on serde defaults as a compatibility policy.
Explicit upgrade assigns initial revisions, preserves existing at-least-once behavior and initializes an empty operator receipt store.
Restore changes storage generation, invalidates old work-view tokens and retains operator idempotency receipts.
Maintenance must validate receipt bounds, revisions and profile consistency, including retained work omitted from the current runtime registry.

## Acceptance scenarios

1. Denied inspection returns no handles, counts or payload fragments. Current identity revocation also denies a previously authorized retry receipt.
2. A state change that preserves claim generation still invalidates an earlier WorkVersion.
3. Two controls with the same version and different keys yield one commit and one conflict; duplicate identical keys yield one receipt.
4. Exit after atomic control commit but before response returns the same receipt on restart, with no repeated scheduling transition.
5. A late worker finish cannot overwrite an operator-controlled generation; active leases are rejected before external verification.
6. A committed Resource action with a lost reply becomes Done by receipt resolution, with unchanged Resource/event/effect counts.
7. Retry preserves attempt/root counters and chain start time. Exhausted attempts, root budget, age, depth and fanout remain blocked.
8. All three delivery profiles show their declared unknown-outcome behavior. The compatibility profile still permits duplicates when the provider does not deduplicate.
9. Verifier Unresolved, timeout, malformed evidence, unavailable registration and changed current authority never grant unsafe resend in the hold profile.
10. Backup/restore retains receipts and hold state; old view tokens fail, and explicit legacy upgrade preserves the prior delivery profile.
11. Both native adapters pass the same control and failure-injection cases; reference/domain compensation remains one generic Resource invocation.
12. CLI rejects a well-formed response with the wrong operation ID or work handle and exits promptly with blocked output on Ctrl-C.

Record actual commands, source revision and results when these tests run.
This design lists required evidence; it does not claim that evidence already exists.
