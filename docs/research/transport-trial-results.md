# Transport composition trials and remaining operational contracts

Date: 2026-10-02. Executed disposable Rust experiment plus primary-source research. Retained on `codex/prototype-transport-trials`, based on main `e3e6a37`; not merged or deployed. Resource remains the sole domain entity. This evaluates implementation choices under the [transport review](transport-layer-review.md), [cohesion review](architecture-cohesion-review.md), [technology review](technology-fit-review.md), and [boundary specification](../../openspec/changes/establish-rom/specs/boundary-adapters/spec.md). It does not select production dependencies or complete actual HTTP/RabbitMQ conformance.

## Recommendation

Keep focused, core-owned `invoke`, `observe` and `subscribe` methods as the semantic boundary. Offer Tower adaptation for bindings that benefit from its middleware. Do not make `Service::poll_ready`, a response future, or a generic response type the definition of accepted work, permission, or stream lifetime.

Tower is useful here: its existing concurrency, load shedding, timeout and buffer layers compose with little binding code and have observable, documented behavior. The experiment also demonstrates exactly why they cannot replace ROM's own supervisor, idempotency, current authorization, subscription permits and shutdown exclusion. Direct methods are the clearer baseline for embedded callers; a Tower wrapper remains a strong optional candidate. This recommendation is implementer judgment from a bounded probe, not a human usability study or throughput benchmark.

## Reproduction and evidence

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/transport-trials && ./prototypes/transport-trials/verify.sh'
```

The [one-command verifier](../../prototypes/transport-trials/verify.sh) runs `cargo fmt --check`, `cargo test --locked -- --test-threads=1`, and `cargo clippy --locked --all-targets -- -D warnings`. Observed result: **15 passed, 0 failed; formatting and Clippy passed**. Rust was `1.99.0 (b940084d7 2026-09-28)` and Cargo `1.99.0 (5f94df478 2026-08-27)`. The [lockfile](../../prototypes/transport-trials/Cargo.lock) pins Tower 0.5.3, Tokio 1.53.1, tokio-util 0.7.19 and Rayon 1.12.0. Current versions were checked against the registry on this date. Three Rayon workers are shared per test core; one host Tokio runtime owns async work. Builds use two compiler jobs and an isolated target directory.

The [implementation](../../prototypes/transport-trials/src/lib.rs) and [test cases](../../prototypes/transport-trials/src/tests.rs) are the primary evidence. Memory-only state deliberately avoids pretending that middleware gives durable acceptance. The CPU gates block Rayon only to expose lifetime boundaries; they are not application I/O patterns. Tests use notifications and explicit polling, with a paused Tokio clock for timeout, rather than elapsed-time sleeps. No HTTP server, broker, persistence adapter or production integration was run.

| Executed case | Observed result | Responsibility established |
| --- | --- | --- |
| Equivalent direct/Tower invocation | Same initial revision/value, same-key replay, conflicting duplicate rejection, stale expected revision rejection, forged serialized actor rejection and current-policy replay rejection. | ROM core; Tower routes into the same method. |
| Bare Tower concurrency negative control | At advertised concurrency one, dropping the first response allowed a second Rayon closure to start while the first CPU gate was still closed. | Tower response capacity does not describe independently running work. This intentionally passes when the defect is exposed. |
| Supervised Tower cancellation | Two dropped response futures left both ROM permits occupied; a third request was overloaded. Closing intake rejected another request and drain remained pending until both gates opened. Afterwards two permits returned and tracked jobs were zero; exactly one competing revision-zero action committed. | ROM-owned admission permits and task registration, not Tower. |
| Direct cancellation and duplicate while executing | Dropped caller left one supervised job; identical retry attached to it without consuming another work permit; different semantic input conflicted. Completion and subsequent replay returned revision one. | ROM's in-flight identity table and response-independent execution. |
| Tower timeout after admission | Timeout returned its `Elapsed` error; ROM still held one work permit. Releasing CPU work and same-key lookup returned the single committed revision. | Timeout is lost observation, not proof of action failure. A binding must map this to unresolved observation. |
| Byte accounting and outcome bounds | Charge 100 exhausted a 128-unit budget for another charge 64 before spawn; charge 129 was rejected. One job existed. Sixteen distinct stored outcomes filled the table; a seventeenth was refused while existing replay remained available. | ROM accounting; this is a synthetic byte charge, not a heap measurement. |
| Ready without call | Obtaining readiness reserved the concurrency slot; another clone stayed pending. Dropping the ready service released that reservation. | Tower readiness and reservation lifecycle. |
| Layer order | `buffer(1).concurrency_limit(1)` admitted one active plus one buffered request; the reversed order left second readiness pending while the first response was retained. | Tower layer order changes effective ingress capacity. |
| Live/journal divergence | Three commits coalesced live delivery to revision three. A journal held at most two queued frames and reported `Lagged`; cursor zero then reported `Gap`, while cursor one replayed revisions two and three. | ROM chooses coalescing versus history recovery; no substitution of live state for lost history. |
| Buffered disclosure and drop | Revocation before receiving a buffered live/journal frame returned `Denied`. Two active subscriptions exhausted capacity; dropping both restored it. | Current authorization and subscription-owned permits. |
| Tower stream lifetime negative control | A Tower response limit of one allowed two returned subscription handles to stay alive. The third was refused by ROM's independent stream limit. | Response completion and stream closure are different lifetimes. |
| Capabilities and authority | Missing live/journal profile failed explicitly; full capability support still denied unauthorized live/journal callers. | Capabilities do not confer permission. |
| Revocation during CPU work | Revoking before release rejected completion disclosure and left resource revision zero; capacity returned after drain. | Core execution-time authorization. |
| Tower load shedding | Outer concurrency one rejected a second call while ROM itself still had one unused work slot. | Genuine outer middleware rejection, distinct from core overload. |
| Terminal stream replay | Repeated `next` after shutdown consistently returned `Closed`. | ROM terminal-state handling, including subsequent calls. |

An initial Clippy run caught an unnecessary identity `map` in an assertion; it was removed and the complete verifier rerun successfully. A review also found that subsequent live `next` calls could wait after a terminal error; the terminal precheck and dedicated repeated-call test cover that case.

## Library guarantees versus ROM's additions

Tower 0.5.3's `Service` requires readiness before `call`; implementations may panic when callers skip it. Readiness may reserve resources, and cloning a ready service does not make the clone ready. These are useful composition contracts but introduce call-site obligations compared with one direct admission method. [Tower Service](https://docs.rs/tower/0.5.3/tower/trait.Service.html).

`ConcurrencyLimit` retains its permit in the response future. Thus dropping that future returns the permit, which is correct for the library's lifetime and inadequate for independently submitted Rayon work. ROM moves its own work/charged-byte permits into a tracked operation that outlives the reply. [Tower response-future source](https://docs.rs/tower/0.5.3/src/tower/limit/concurrency/future.rs.html).

Tower's builder order controls which layer sees the request first; buffering adds queue capacity before the inner service. Buffer readiness reserves a queue slot and its bound describes queued messages, not arbitrary payload bytes or all returned streams. Use explicit ordering and avoid an invisible extra command queue in the first binding. [ServiceBuilder](https://docs.rs/tower/0.5.3/tower/builder/struct.ServiceBuilder.html), [Buffer](https://docs.rs/tower/0.5.3/tower/buffer/struct.Buffer.html).

Tokio-util `TaskTracker` releases completed task tracking immediately and its wait finishes when closed and empty. Closing it does **not** prohibit new tasks, and dropping it does not abort them. The probe therefore serializes intake closure and task registration under the same ROM state lock. Tracker membership supplies neither a durable recovery obligation nor a supervisor-failure policy. [TaskTracker](https://docs.rs/tokio-util/0.7.19/tokio_util/task/task_tracker/struct.TaskTracker.html).

The wrapper's `poll_ready` is deliberately always ready to attempt bounded admission: admission needs the actual context, key, input and byte charge, which readiness does not receive. Calling the direct method synchronously either rejects or transfers execution to ROM; the returned future only observes an outcome. The Tower adapter preserves that timing even before the response is first polled. A future production wrapper could reserve a coarse slot in readiness, but must not equate it with final request-dependent admission.

Actix-service 2.0.3 uses `&self` for readiness/call, allows readiness false positives and explicitly requires services to tolerate calls without readiness. That may be convenient for shared-reference handlers but shifts enforcement back into the implementation. It does not remove ROM's admission or lifetime requirements; no Actix executable comparison was run, and adding a third stack would not answer a new uncertainty here. [Actix Service](https://docs.rs/actix-service/2.0.3/actix_service/trait.Service.html).

## Ergonomics and queue coverage

The direct path keeps admission failure typed and separate from a later protected result. Separate `Live` and `Journal` handles make overflow/recovery differences visible. Tower saves writing generic timeout/load-shedding adapters, but adds mutable readiness sequencing, layer-order reasoning, and boxed-error mapping: tests downcast the boxed semantic error while testing Tower overload/timeout separately. Production bindings should normalize these categories once. Application resource declarations should never contain Tower types or repeat the authorization path.

The probe has one concrete Resource, integer semantic input, and host-minted identity values. It is not evidence about descriptor routing across multiple resource kinds, typed input codecs, multitenancy, real credentials or allocation overhead. The public `host_context` helper is a trusted-host test seam; it is not an untrusted identity provider or a native-code sandbox.

| Queue or retained state | Probe coverage | Required production boundary |
| --- | --- | --- |
| Caller futures before admission | Direct path rejects synchronously; Tower readiness reservations and queue order exercised. | Listener connections, decoded payloads and pending readiness waiters need their own count/byte/time limits. |
| Accepted async/Rayon work | Two ROM permits acquired before spawning; retained through actual work and outcome recording. | Tune shared pool size separately from accepted jobs; tenant fairness and cooperative CPU expiry are untested. |
| Accepted payload budget | 128 units, maximum charge 128; pressure tested with charges 100 and 64. | Derive charge from trusted encoded/decoded sizes and include allocations, not caller claims. Fixed integer input does not measure real payload memory. |
| Duplicate waiters | One job for an identical pending identity. | Number of reply watchers is not bounded by work permits; add host ingress or per-identity waiter limits. |
| Completed outcomes | Fixed-size values and at most 16 entries; no silent eviction. | Durable retention, tombstones and replay horizon need separate policy. Tracker cleanup does not delete outcomes. |
| Live frames | One fixed 16-byte logical Resource frame per watch slot; coalesces. | Bound full projection bytes and computation, plus codec/network buffers and maximum subscriptions. |
| Journal frames | At most two fixed frames per channel; terminal lag on overflow. | Stop delivery, resume durable history or return an explicit gap; already-buffered terminal frames remain bounded until handle drop. |
| Journal retention | Last two in-memory frames; old/future cursors fail. | Scoped/versioned durable cursors, retention horizon and authenticated checkpoints. |
| Stream registry | Two active permits; dead senders pruned on registration/publication; shutdown clears registry. | Prompt expiry/revocation notification for idle streams, maximum idle/lifetime budget, and final binding cleanup. |
| Broker and transport buffers | Not executed. | RabbitMQ prefetch, serialized bodies, transport output, retransmits and HTTP body decoding must fit the host budget. |

The state lock linearizes the tiny in-process commit, history publication and subscription setup. It supplies no cross-process consistency. The finite memory outcomes must never be advertised as durable acceptance. Unexpected supervisor panic/runtime destruction, unwinding in CPU code, process crashes, restart recovery, distributed fairness and real uncertain commits are untested. There is no throughput or latency conclusion.

## Remaining topology and worker ownership research

Minimum first milestone: **one runtime owns writes and live-query publication**, with host-owned Tokio/Rayon services, one selected persistence adapter, optional HTTP binding and in-process embedding. Preserve a recoverable durable journal and action identity in the integrated implementation; the present memory probe does not supply them. This follows the [cohesion review's cross-writer warning](architecture-cohesion-review.md) and narrows the first proof without changing Resource semantics.

| Alternative | Primary-source constraint | Proposed ROM decision / defer reason |
| --- | --- | --- |
| Multiple runtimes sharing a SQL database | PostgreSQL describes `SKIP LOCKED` as useful for queue consumers but unsuitable for a general consistent read because it skips locked rows. [SELECT](https://www.postgresql.org/docs/current/sql-select.html#SQL-FOR-UPDATE-SHARE). | Later use transactionally claimed work with explicit lease/fencing generation, renewal, expiry, recovery and idempotent completion. A work claim is not a live-query snapshot or proof that a previous worker stopped. Define stale-worker commit rejection before enabling this profile. |
| Cross-process invalidation via database notifications | PostgreSQL delivers transaction-generated notifications after commit, can fold identical notifications within a transaction, and directs larger information into tables. [NOTIFY](https://www.postgresql.org/docs/current/sql-notify.html). | Use notifications only to wake authoritative journal/generation reads. Reconnect must reconcile from storage; do not treat notification receipt as complete durable history or current authorization. Keep this out of milestone one. |
| RabbitMQ competing or single-active consumers | Single Active Consumer chooses one receiving consumer and fails over when it disappears; ordinary consumers share deliveries. [Consumers](https://www.rabbitmq.com/docs/consumers#single-active-consumer). | Broker dispatch ownership is not fencing of a disconnected worker still running or permission to execute. Keep action idempotency/storage arbitration authoritative. SAC can simplify a later ordered binding but cannot establish exactly-once effects. |
| Separate durable worker service | Consumer acknowledgements and publisher confirms concern different broker interactions; channel-scoped delivery identity is separate from action identity. [Acknowledgements](https://www.rabbitmq.com/docs/confirms). | Optional durable inbox/handoff must atomically record its recovery owner before acknowledging. Otherwise acknowledge only after recoverable terminal outcome. Choose retry/dead-letter budgets and independently validated delegation at execution. Do not implement a bus-shaped core. |

Those are proposed policies, not guarantees of Tower, RabbitMQ, PostgreSQL or this probe. Backend and host owners must select embedded versus independently writing topology, ordering scope, stale-owner fencing mechanism, acceptable duplicate external effects and the worker recovery SLO before multi-instance support is advertised.

## Cursor, history and receipt retention

Retained requirements already distinguish coalescing live state from durable journal recovery, and atomic resource/revision/action-receipt/event persistence. [Event reactivity](../../openspec/changes/establish-rom/specs/event-reactivity/spec.md), [persistence](../../openspec/changes/establish-rom/specs/persistence/spec.md). A Web EventSource reconnect sends its last event identifier; it does not specify ROM's processing checkpoint or retention promise. [WHATWG EventSource](https://html.spec.whatwg.org/multipage/server-sent-events.html#the-last-event-id-header).

RabbitMQ streams permit age/size retention, evaluated by segment. Etcd similarly documents a history window bounded by compaction, and warns that watch delivery is asynchronous with no bounded delay guarantee. These are useful constraints when designing ROM's own recovery protocol, not reasons to adopt either product as the core journal. [RabbitMQ stream retention](https://www.rabbitmq.com/docs/streams#retention), [etcd API guarantees](https://etcd.io/docs/v3.6/learning/api_guarantees/).

Recommended minimum contract, still requiring integrated durable tests:

- Cursor binds to isolation scope, subscription/filter version, ordered history position and history generation; validate authorization before disclosing either data or protected history metadata. Per-resource revision order does not imply a globally ordered distributed journal.
- Resume within the advertised floor replays committed facts. Below the floor returns typed `HistoryGap` with an authorized recovery option. A current snapshot may rebuild a projection but must never masquerade as delivery of every lost event. Live reconnect instead obtains a fresh snapshot under its separate profile.
- Keep delivery position and durable consumer processing checkpoint separate. Add authenticated checkpointing only to profiles that promise it. Slow durable consumers either pin retention under a quota or receive explicit failure when policy advances the floor; unlimited pinning is not a bounded design.
- Publish an idempotency replay horizon tied to the maximum accepted retry/redelivery lifetime. After receipt retirement, retain enough durable identity/tombstone or reject out-of-horizon retries to avoid silently re-executing an old action. A missing record alone cannot prove no commit. Do not equate receipt, journal, audit and resource-lifetime retention.
- Initially retain receipts without automatic deletion within a finite configured storage budget; refuse new accepted obligations before exhausting the budget. This is a conservative milestone proposal, not the production retention policy. Replace it only after expiry semantics and migration/recovery tests exist.

Owner decisions: ordered scope, minimum journal horizon, maximum retry age, tombstone cost, checkpoint authentication, lag quotas, history-gap UX and deletion/privacy obligations. The probe's two-frame/16-outcome limits are stress values, not proposed defaults.

## Observability and operational profiles

W3C Trace Context restricts its fields to correlation and excludes sensitive information; OpenTelemetry supports links to spans from the same or different traces. Therefore correlate each transport attempt to supervised execution using tracing metadata, while retaining separate ROM action identity and authorization. Neither trace IDs nor broker delivery IDs grant authority or define durable command uniqueness. [Trace Context](https://www.w3.org/TR/trace-context/#privacy), [OpenTelemetry tracing API](https://opentelemetry.io/docs/specs/otel/trace/api/#link).

Proposed first milestone instrumentation: admission accepted/rejected by semantic category; accepted/running jobs and charged bytes; duplicate joins/replays/conflicts; dropped replies and unresolved observations; active streams, queued bytes, coalesced live updates, journal lag/gaps; commit latency versus caller wait latency; shutdown outstanding jobs and unfinished identity count. Use bounded-cardinality resource-kind/operation/binding labels. Put authorized, redacted action correlation in spans or access-controlled logs rather than high-cardinality metric labels. A retry gets a new attempt span linked to original execution. The probe does not instrument or benchmark a telemetry backend.

| Operational phase | Minimum proposed behavior | Evidence / gap |
| --- | --- | --- |
| Normal intake | Decode under body/connection limits, obtain trusted context, reject unsupported profiles, then bounded semantic admission. | Core count/charge rejection executed; wire decoding/limits pending. |
| Overload | Prefer explicit overload without an extra unbounded application queue. If Tower Buffer is enabled, include both its queue and readiness reservations in budgets. Bound duplicate observers independently. | Layer-order and load-shed checks executed; fairness/load testing pending. |
| Caller timeout/disconnect | Stop observation; retained ROM work continues under its contract. Record unresolved knowledge and support authorized same-key resolution. | CPU cancellation/timeout executed; durable and network reply-loss recovery pending. |
| Shutdown begins | Stop listener/broker intake; serialize core intake closure against acceptance; close observation with explicit terminal reason. | Core exclusion and stream terminal behavior executed. Real listener and consumer cancellation pending. |
| Drain | Keep runtime, database and pool alive while accepted jobs complete; reap completions continuously; acknowledge only recoverable outcomes. | TaskTracker and gate-controlled drain executed, memory-only. |
| Deadline exceeded | Report unfinished identities and do not call them rolled back. Apply documented host termination/recovery policy. Cooperative CPU stop can reduce work only at defined safe boundaries. | Proposed; no forced-stop or crash experiment. |

Tokio's shutdown guidance separates deciding, signaling and waiting. Dropping a runtime does not guarantee async task completion; a runtime shutdown timeout is not proof that blocking work stopped. ROM must complete its own drain before the host tears down needed services, and must explicitly handle the case where it cannot. [Tokio graceful shutdown](https://tokio.rs/tokio/topics/shutdown), [Runtime shutdown](https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Runtime.html#shutdown).

Owner decisions: operational count/byte limits, admission wait budget, maximum stream age/idle time, shutdown grace and hard-stop policy, telemetry exporter/redaction rules, alerts for retained work/lag/unknown outcomes, and the recovery objective. These settings are not derivable from crate defaults.

## Next acceptance gate and status

This trial supplies evidence for establish-rom task **4.5** (Tower versus direct, readiness/cancellation/stream lifetime). It intentionally leaves task **4.6** (actual bindings and durable cross-boundary conformance) unfinished and does not change the OpenSpec checklist on this experimental branch.

Next integrate one typed downstream Resource declaration with the selected persistence slice and a thin HTTP binding. Run direct and actual HTTP vectors against the same core: bounded admission, abandoned CPU reply, commit then lost reply, same-key recovery after restart, conflicting input, policy revocation before buffered disclosure, live resync versus journal gap, and shutdown with accepted work. Then add one transactionally persisted external-effect intention and prove worker-loss recovery. Keep Tower optional until that integration and a human author walkthrough show that its convenience outweighs exposing an additional public abstraction.
