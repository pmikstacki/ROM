# HTTP integration boundaries for the maintained MVP

Date: 2026-10-02. Focused implementation brief after the
[Tower/direct trial](transport-trial-results.md). This is source-backed planning;
no maintained HTTP adapter has been executed yet.

## Recommendation

Use Axum in `rom-http`, outside `rom`, for a small generic binding. Current
official documentation identifies Axum 0.8.9 and exposes SSE responses from Rust
streams. This supplies wire handling, not Resource semantics. A live handle
can become an SSE stream while still owning a ROM subscription permit and
obeying current authorization. Keepalive traffic is transport liveness, not a
committed Resource event or processing checkpoint.
[Axum SSE](https://docs.rs/axum/0.8.9/axum/response/sse/).

ROM's typed and erased invocation APIs must lower into the same registered
mutation pipeline. The erased form is needed for descriptor routing, not a
second implementation of operations. Reads, live observations and journal
subscriptions retain separate contracts. A client-controlled kind/name never
selects an arbitrary native function: only registered definitions are callable.

## Boundaries and acceptance vectors

| Boundary | Required behavior | Concrete test |
| --- | --- | --- |
| Identity | Host-selected verifier produces trusted context. JSON fields and headers claiming a subject/role cannot grant authority. | Forged admin fields cannot invoke an action; verified principal gets the same policy outcome as embedded use. |
| Body admission | Bound bytes before deserializing and bound concurrent accepted bodies independently from CPU work. | Oversized declared and chunked bodies reject before action execution; a slow sender cannot create unbounded retained requests. |
| Decoding | Apply the accepted Resource/action schema; reject unknown fields, duplicate object keys, wrong types and unsupported operators with safe paths. | `false`, zero, empty and explicit null round-trip; absent update does not become null or false. |
| Mutation identity | Keep mutation variant, registered action and normalized input semantics unambiguous. | A custom action named like a built-in cannot replay the wrong receipt; altered payload under one identity fails. |
| Result knowledge | Distinguish rejection, committed outcome and unknown observation. Dropping HTTP interest does not release accepted work. | Commit, close socket before reply, reconnect/retry same key: one mutation/event. |
| Live state | Bounded full snapshots may coalesce; reconnect establishes a fresh current-authorized snapshot. | Task completion removes it from filtered results; revocation before poll reveals no retained old rows; subscription permits release on disconnect. |
| Journal | Cursor/generation/scope/retention are ROM semantics. Never coalesce distinct facts. | Resume within floor replays; expired/future/wrong-scope cursor returns typed error without leaking protected history. |
| Error mapping | Map safe semantic categories once. Keep internal diagnostics protected. | Storage/codec failures do not echo credential, secret, source path or raw driver error. |
| Lifecycle | Stop HTTP intake, close live observers, drain ROM work, then tear down shared services. | A long-lived stream does not make shutdown hang; a cancelled HTTP response cannot orphan accepted work. |

Axum's default body limit applies only to extractors that cooperate with it;
manual frame consumption can bypass it. The binding must use a bounded body
reader or a globally applied limit, not infer protection from a default layer.
The official comparison explicitly distinguishes `DefaultBodyLimit` from the
global `RequestBodyLimit`.
[DefaultBodyLimit](https://docs.rs/axum/0.8.9/axum/extract/struct.DefaultBodyLimit.html),
[tower-http limit](https://docs.rs/tower-http/latest/tower_http/limit/index.html).

Axum provides graceful server shutdown, but that does not implement ROM's
accepted-job accounting or close application-owned infinite streams for it.
Coordinate both lifecycle owners explicitly and exercise their combined path.
[Axum shutdown handle](https://docs.rs/axum/0.8.9/axum/serve/struct.WithGracefulShutdown.html).

## Critique and limits

Axum is a focused adapter choice, not a transport abstraction in core. Its Tower
integration does not reverse the trial's cancellation finding: ROM still owns
work and stream permits. Adding a general middleware platform to Resource
declarations would expose infrastructure complexity without solving the user's
authoring problem. A custom raw Hyper server would give lower-level control but
require more request/error/SSE plumbing; the current evidence does not justify
that cost for this MVP.

SSE supports server-to-client delivery; writes remain ordinary authenticated
HTTP requests. Bidirectional protocols and broker bindings can be separate
capability profiles. Do not promise replay from a live snapshot stream or infer
exactly-once processing from an SSE event ID. Core compilation without Axum,
Hyper or Tower is a release check. Real TLS and external provider interoperation
remain separately recorded deployment evidence; loopback tests do not establish
them.
