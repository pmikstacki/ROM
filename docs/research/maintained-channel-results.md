# Maintained named notification channels

Date: 2026-10-02. Baseline main `2518493`; implementation is the commit containing this report on `codex/durable-channels`. This extends the maintained Resource core and existing reaction work ledger. It does not copy the prototype outbox database or claim the whole MVP is complete.

## What an author writes

```rust,ignore
const NOTICE: Channel<String> = Channel::new("account-notices", 1);

async fn application_send(delivery: Delivery<String>) -> DeliveryOutcome {
    // Custom host/provider I/O goes here. Reuse delivery.id for receiver dedup.
    DeliveryOutcome::Accepted
}

let service = Actor::trusted("host", "notifier")
    .with_kind(PrincipalKind::Service);
let builder = Runtime::builder()
    .channel(NOTICE, service, application_send);

// Inside an ordinary typed Resource action after changing its state:
Ok(vec![NOTICE.intent("account changed".to_owned())])
```

No SMTP/cloud SDK is in the core. Ordinary named async functions and capturing closures are accepted. The host owns Tokio and Rayon. `start_work()` starts the same generic runtime-owned worker used for reactions; `process_work(max_steps)` runs the same finite supervised batch. Existing `start_reactions`/`process_reactions` remain compatible and now also dispatch notification records.

Payloads use the existing `Input` encode/decode contract. Strings, booleans and supported field types work directly. A structured custom payload currently needs an `Input` implementation; there is no notification-specific serde derive. This is a remaining authoring cost, not hidden per-Resource infrastructure. A future serde convenience wrapper can remove repetitive codecs without changing the durable contract.

`Channel::intent` sets an explicit payload version marker. Existing `Intent::new` remains an opaque stored effect and is **not** implicitly sent. A typed notification whose channel is unknown, version differs or payload fails canonical codec validation rejects the entire upstream mutation before commit. Host registration rejects duplicate/empty names, zero versions and non-Service identities. Native callbacks and codecs remain trusted application code.

## One atomic persistence and work lifecycle

The core validates channel intents and builds Notification obligations before calling the existing generic `Storage::commit`. Resource state, event, receipt, ordinary effect record and notification work therefore use the same native SQLite/redb transaction. No send happens inside a database transaction. A failed downstream delivery preserves the committed upstream Resource. No-op mutations still reject external effect intents in this profile.

Notification records share `WorkLedger`, root budgets, lease generations, claim recovery, backlog caps and terminal inspection with reactions. A configured channel version and stable service principal key are persisted with the immutable source snapshot and payload. The worker verifies current service authority and permission for the complete current and historical source fields before exposing the payload. Changed definitions, revoked service identity, hidden source fields or deleted sources stop without invoking the channel.

`Delivery` carries the typed payload, durable attempt number and stable receiver ID. The ID is a domain-tagged SHA-256 digest of the committed action identity, causal path, channel name and effect ordinal. It is 64 lowercase hex characters, remains unchanged across retries and has a namespace distinct from reaction work. This uses pinned `sha2 = 0.10.9`, already present in the authentication dependency lock. It bounds receiver keys; it is not a secret or an authorization token.

Before invoking the function, the worker persists an Unknown delivery observation. It then records `Accepted`, `Retryable`, `Permanent`, `Unknown`, `TimedOut` or `Panicked` under the current claim generation. `Accepted` marks work Done. Permanent and panic outcomes stop visibly. Retryable, Unknown and timeout use bounded exponential retry under the existing attempt/root/age policy. A worker lost on its last allowed attempt later becomes terminal with its Unknown observation retained. Local lease fencing prevents stale acknowledgments overwriting a newer claim; it cannot fence the external receiver's side effects.

The function runs in an owned Tokio task while its already-admitted blocking work retains supervision and an I/O permit. Default delivery timeout is five seconds; the host can change it with `Builder::delivery_timeout`. Registration requires a positive timeout shorter than the configured lease. Timeout aborts and joins a cooperative future and records uncertainty, never external rollback. Canceling the caller that observes a batch does not abort the accepted send. Runtime shutdown waits for accepted attempts and the shared loop.

This conservatively occupies a bounded I/O slot while awaiting async I/O. It is correct for the finite profile but can reduce throughput. A future that blocks a Tokio thread or never yields cannot be forcibly terminated; strong isolation requires a separate process. Application-created detached tasks are outside ROM supervision and should not be used to evade the channel acknowledgment boundary.

## Delivery ambiguity is measured

The local fake receiver applies its effect and returns Unknown to model a lost acknowledgment. The test closes/reopens the actual ROM database, then retries under the same stable key. With receiver-owned deduplication, two attempts produce one effect. Without it, two attempts produce two effects. This is run on both SQLite and redb. The receiver is an in-process fixture whose state survives the ROM runtime/database reopen; this is not a remote service or machine-crash durability certification.

ROM provides durable bounded attempts. It does not promise eventual delivery, transactional exactly-once external effects, inbox arrival or human attention. Provider receipt reconciliation, retry-after/jitter, acceptance receipts, manual review and operator requeue are not implemented. `Storage::reaction_records` is a trusted, ledger-bounded administrative inspection surface; no public HTTP notification administration endpoint was added.

## Storage compatibility and limits

Both adapters now write **format 3** and explicitly reject format 2 without migration. Notification work has a new serialized enum variant that older binaries cannot decode. No separate notification table or outbox schema is added. The default retry profile remains three attempts, 30-second lease, one-second exponential base delay, 3,600-second chain age and 256 total root work attempts. Work retains the existing 1,024-record / 1 MiB encoded-ledger caps, and channel effect payloads remain subject to normal command/effect bounds. Done/terminal records remain retained; archive/maintenance policies are still needed for long-running operation.

The existing whole-metadata clone/serialization and idle-claim write amplification also apply to channels. This package does not claim throughput improvements. Protected deletion/configuration metadata and the final combined format/profile are coordinated with the parallel packages; they must be integrated and independently reviewed before an MVP completion claim.

## Verification

`./scripts/check` is the one-command native Rust 1.99 verifier. It covers strict OpenSpec checks, fmt, warning-free clippy, workspace/identity/HTTP tests, rustdoc, driver-free no-derive core, executable consumer, compile-fail fixtures and authentication feature/rustls profiles. The new channel module also includes a compiled ordinary-function registration example.

Seventeen focused tests cover both native adapters where relevant: atomic enqueue/reopen without send; accepted typed dispatch; bounded retry/permanent/unknown outcomes; unknown/version/type rejection before commit; service revocation; effect/lost-ack/reopen with and without receiver dedup; cooperative timeout and panic; caller cancellation and shutdown drain; registration changes; stale acknowledgment fencing; two differently typed functions; unresolved last-attempt recovery; before/after native commit faults; both format 2 rejection controls; current/historical field denial before callback execution; and 32 immediate native database reopens proving drained work releases its adapter handle.

One early development run returned native Unknown during two seed commits. Identical reruns and 20 focused enqueue/reopen repetitions passed without a code change. Available evidence does not establish the cause; no error was converted into success or assumed rollback. The final complete verifier must be used for this commit, and this observation remains recorded for future recurrence diagnostics.

The disposable 12-test notification prototype remains useful prior evidence; these tests exercise the maintained core and shared adapters instead. No real messages were sent, no providers were selected, and no performance result is inferred from timings under parallel builds.

## Lifecycle defect found during integration

The full workspace run intermittently failed a redb reopen with Storage. Temporary diagnostic output identified DatabaseAlreadyOpen; a 20 ms delay after the I/O response made the failure deterministic. The tracked work guard had decremented active work before the closure released its Runtime clone. Thus shutdown could return while completed work still retained the database.

The guard now owns only an independent lifecycle mutex/watch signal and its permit. I/O jobs explicitly drop their adapter-owning Runtime before dropping the guard, and the shared async loop follows the same order. The amplified regression passed after this fix, then diagnostic code was removed. A permanent 32-cycle actual redb reopen test verifies the external storage handle is the sole strong reference after shutdown/drop. This corrects the previously documented drain guarantee; it does not require sleeps or retries to open a database.
