# Design: durable mutation recovery

Status: proposed for review. No implementation, failing test execution or acceptance result is claimed.

## Intent and evidence

The application author needs save, lost acknowledgement, reload and safe retry without rebuilding operation identity handling.
New edits must remain visible while an earlier command has an unknown outcome.
The same workflow must compose with explicit navigation and identity lifecycle decisions.

Inspected source includes `studio/src/lib/client/session.ts`, `types.ts`, `serialization.ts`, `request.ts` and `mutation.ts`.
Controller navigation and disconnect behavior are in `studio/src/lib/application/controller.ts`.
Consumer autosave is in `/root/astral-plane/web/src/ConversationSessions.svelte`; session refresh is in its `App.svelte`.
Backend durable identity and replay are in `crates/rom/src/policy.rs`, `invocation.rs`, `execution/command.rs` and `replay.rs`.
HTTP errors are in `crates/rom-http/src/error.rs`.

The client's WeakMap owns cloned submitted data. The public request object is not literally frozen.
The private snapshot prevents caller edits from changing an attempt. Its client generation prevents stale reply acceptance and stale prepared-object reuse.
The controller disconnect clears pending state. The consumer retains custom signatures, pending operations and navigation barriers.
These are source observations, not newly executed tests or proof of a producer defect.

## Selected approach and alternatives

Add one recovery lane for one Resource target. The lane owns an accepted command and the latest unaccepted edit draft.
Persist before dispatch. Restore the exact accepted command through the existing client prepare method.
Retain current authorization and existing backend receipts as the only authority for execution and disclosure.

Exporting private prepared objects would couple storage to WeakMap ownership and session generation. Reject that route.
A parallel mutation repository or new receipt service would duplicate existing durability guarantees. Reject that route.
The selected helper manages browser intent only. It never claims to know committed server state from local storage.

## Proposed public API

Add `studio/src/recovery.ts` as a facade. Reexport the same API from the existing root.
Add public `rom-studio/recovery` and `rom-studio/client` subpaths for independent source consumers.
The client subpath exposes createClient, RemoteError, parseWire, stringifyWire, the recovery helper and their public types.
Add `studio/src/client.ts` as its source facade. It does not add browser auth exports.
Any future browser auth entry is a separate host-profile decision, not part of transport credential callbacks.
The root application entry retains its existing packaging limits.

Implementation files have these responsibilities:

- `studio/src/lib/recovery/types.ts`: public interfaces and discriminated state.
- `record.ts`: bounded versioned record codec and strict invocation validation.
- `controller.ts`: lane state, storage ordering, generation rebinding and attempts.
- `recovery.ts`: named exports only.

```ts
interface DurablePrincipal {
  authority: string;
  kind: "embedded" | "human" | "service";
  subject: string;
}
interface StoredIntent {
  version: string;
  payload: string;
}
interface PendingIntentStore {
  read(slot: string): Promise<StoredIntent | null>;
  compareExchange(
    slot: string,
    expectedVersion: string | null,
    next: StoredIntent | null,
  ): Promise<boolean>;
}
interface RecoveryBinding {
  client: RomClient;
  principal: DurablePrincipal | null;
}
interface MutationRecoveryOptions {
  namespace: string;
  slot: string;
  target: { kind: string; id: string };
  binding: RecoveryBinding;
  store: PendingIntentStore;
  newVersion: () => string;
  maxBytes?: number; // default 1048576; integer > 0
}
type RecoveryPhase =
  | "idle" | "prepared" | "submitting" | "unknown" | "succeeded"
  | "rejected" | "conflict" | "quarantined" | "storage_error";
type CommitKnowledge = "not_attempted" | "unknown" | "committed" | "not_committed";
interface MutationRecoveryState {
  phase: RecoveryPhase;
  commitKnowledge: CommitKnowledge;
  hasDraft: boolean;
  hasUnresolvedIntent: boolean;
  draft: Operation | null;
  result: ProjectedView | null;
  error: { category: string; reason: string } | null;
}
interface BeginMutation {
  expected: bigint | null;
  idempotency: string;
  retryEpoch?: bigint; // omission retains epoch zero
}
interface MutationRecovery {
  readonly state: MutationRecoveryState;
  subscribe(listener: (state: MutationRecoveryState) => void): () => void;
  restore(): Promise<void>;
  stage(operation: Operation | null): Promise<void>;
  begin(identity: BeginMutation): Promise<void>;
  retry(signal?: AbortSignal): Promise<ProjectedView>;
  rebind(binding: RecoveryBinding): Promise<void>;
  discard(options: { acknowledgePossibleCommit: boolean }): Promise<void>;
  dispose(): void;
}
function createMutationRecovery(options: MutationRecoveryOptions): MutationRecovery;
```

`stage` updates the latest edit draft. It accepts no new command identity and performs no network request.
`begin` snapshots the current draft, target and supplied identity. It persists the accepted command before exposing prepared state.
`retry` is the only dispatch operation. It accepts prepared or unresolved records and sends the same command.
A successful retry does not dispatch the next draft. The host explicitly begins that edit with its chosen expected revision and new identity.
The host never automatically rebases an action or merges a patch after conflict.
Read-only state snapshots are isolated from controller internals; caller mutations cannot alter stored bytes or results.

## Durable record and storage ordering

Use format `rom-mutation-intent-v1`. The record contains namespace, durable principal, target, latest draftWire and accepted invocationWire.
It also contains accepted draftWire, attempted flag, phase and commit knowledge. It contains no projected result or backend receipt.
`invocationWire` is the exact string from the existing bounded wire serializer. Do not serialize it through ordinary JSON as a value tree.
The outer record may encode these wire strings through JSON. Decode its bounded strings before using the ROM wire codec.

Retain expected revision, idempotency key, operation tag and optional retry epoch exactly.
Preserve absence, null, false, zero, empty text, removal, exact integers, decimal-token category and negative zero.
Validate kind/id, nonempty identity, revision range, operation discriminants and field-update tags before preparing a restored command.
Reject unsupported format versions, duplicate keys, excess fields, excessive size/depth, invalid Unicode, inherited properties and corrupted wire records.
Do not silently normalize or mint a new command when decoding fails.

Store writes use compareExchange with unique host-generated record versions. A failed comparison enters storage_error and loads no replacement implicitly.
Storage implementations must provide atomic comparison, bounded values and their stated durability. A resolved write is the host's durability boundary.
Serialized lane operations prevent local stage/begin writes from overwriting each other.
No network request runs before accepted-command persistence succeeds.
Before dispatch, persist attempted=true. A crash after that marker conservatively restores unknown, even if the request was never sent.
If post-result persistence fails, retain the accepted identity and report storage_error. Never silently permit a replacement command.
Concurrent helpers may replay the same command; server idempotency handles duplicates. CAS prevents one helper from deleting a newer record.
The helper provides no browser-wide leader election or exactly-once network delivery claim.

## Drafts, outcomes and navigation

Stage B then C while accepted A is in flight or unknown. Persist C as the latest draft; never rewrite A.
After receipt confirmation for A, keep C. Clear the draft only when its exact draftWire matches the accepted draftWire.
An explicit begin for C uses a new identity and a host-selected expected revision. Historical receipt revision is not silently treated as current row revision.
Invalid editor text remains in the application editor. It is not a valid Operation and does not enter the helper's wire record.
The host supplies draft storage if it needs invalid editor text to survive reload.

Unknown means an attempted mutation might have committed. Rejected and conflict describe the latest response, not erased history.
After an unknown attempt, a denied, expired-identity, identity-mismatch, history-gap or conflicting retry retains unknown commit knowledge.
A confirmed receipt establishes committed. The host's current authority may prevent that disclosure; the helper cannot override it.
A not_committed response establishes only the current attempt's result. It does not erase uncertainty from an earlier attempted command.
For a command with no earlier uncertain attempt, an explicit not_committed response can establish not_committed.
Any error classification that lacks that guarantee stays conservative. No new key is generated automatically.

Navigation is application-owned. A host waits for its current save/attempt, then checks hasUnresolvedIntent and hasDraft.
If either is true, retain the editor and offer retry or explicit local discard. Switching a route must not silently replace the stored command.
`discard` removes only local pending intent. If it was ever attempted or unknown, require acknowledgePossibleCommit=true.
Discard does not cancel the server or prove rollback. The host must describe that distinction.

## Identity, sessions and credentials

Durable principal equals backend authority, principal kind and subject. Host stamp, expiry, CSRF and client generation are not durable identity.
The host supplies this tuple from authenticated session context. A stored tuple never grants authentication or authorization.
Namespace binds the record to the configured deployment. Stored command data cannot select an endpoint or credential provider.
No bearer token, cookie, CSRF value, host secret or session-generation token enters the record.
Host storage policy must exclude credential-bearing mutation inputs; private application data requires an explicit storage/retention decision.
The helper has no localStorage default or process-global store.

`rebind` invalidates local response tickets and aborts the current transport attempt.
For the same durable principal, retain the exact record and re-prepare it in the new client generation. Do not retain the old prepared object.
For null or changed principal, clear disclosed result, draft and error content from observable state. Expose only quarantined status.
Do not submit an old command through the new principal. The old record stays in host storage until its owner restores or explicitly discards it.
Returning to the original principal requires explicit restore and current server authorization.
Check principal binding and local ticket before applying results, notifying listeners or resolving a result promise.
A stale attempt rejects without returning its projected result. Switching away and back cannot revive stale completions.

## Cancellation and receipt limits

Caller AbortSignal and dispose stop local transport/waiting. They do not invoke a server cancellation action.
After possible dispatch, persist or retain unknown; pending intent survives unmount.
An already aborted signal before dispatch produces no request and retains the prepared command.
Retries reuse the saved retry epoch. Omission means original epoch zero, not current server epoch.
Retention/history-gap failures remain visible; do not reconstruct a receipt, advance epoch or replay under a new key.
Existing backend action, event and receipt atomicity remains unchanged.

## Acceptance boundary

Unit fixtures exercise actual createClient serialization with an injected transport; they do not establish durable backend replay.
Actual loopback HTTP tests run SQLite and redb with disk files, server restart and a response-dropping proxy.
The proxy forwards a real request, waits for backend commit evidence, then drops acknowledgement. It never fabricates successful storage.
An extracted Svelte application installs public recovery/client/controls/CSS entries and talks to the real isolated host.
It reloads saved browser intent, retries under a renewed same-principal session, and checks one row/event/receipt outcome.
Both browser engines and both adapters are required. No production consumer or real credential is used.

Implementation starts after coordinator design review and separate file ownership assignment.
