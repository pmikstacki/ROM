# HTTP binding

`rom-http` binds registered Resource definitions through one generic protocol.
The host constructs `Http::new(runtime, resolver, limits)` and runs
`http.serve(listener, stop_future)`. No Resource-specific controller or route is
required. `AuthResolver` is a fast, nonblocking host function from HTTP headers
to `Result<Actor>`; use a verified credential context, never a claimed subject
header. Actor is deliberately not deserializable. The binding does not implement
cryptographic credential verification or TLS.

All routes use POST with JSON. Clients can use a streaming fetch client for SSE;
these POST streams are not the browser's GET-only `EventSource` API.

| Route | Request | Response |
| --- | --- | --- |
| `/discover` | `{}` | Authorized `Discovery` catalog |
| `/invoke` | `Invocation` | `ProjectedView` |
| `/read` | `{kind,id}` | `ProjectedView` |
| `/query` | `{kind,query} or legacy {kind,field,value}` | Array of projected views |
| `/live` | `{kind,query} or legacy {kind,field,value}` | SSE current snapshots, initial then changes |
| `/journal/head` | `{kind}` | Explicit current `JournalCursor` |
| `/journal` | `{kind,after}` | Bounded `JournalBatch` |
| `/subscribe` | `{kind,after}` | SSE ordered journal batches |

Example invocation:

```json
{"kind":"tasks","id":"one","expected":1,"idempotency":"finish-1","operation":{"type":"action","input":{"name":"complete","input":null}}}
```

The [discovery catalog](discovery.md) requires explicit metadata grants, defaults
to no visible Resources, and does not assert permission to read or mutate rows.
Custom action inputs remain opaque. Hidden reference targets are not disclosed.

Operation tags are `create`, `replace`, `patch`, `delete`, and `action`. Create/replace
carry a complete Resource value in `input`; delete has no input. Replacement is
not a partial patch. [Explicit PATCH](presence-and-patch.md) distinguishes omission, null and removal. Custom action input is checked by its registered codec.
Unknown request fields and duplicate JSON keys, including nested duplicates,
fail before dispatch. All results use the current row and field projection
policy. Predicate authority is checked even when no row matches.

`Runtime::execute<R>` and `Runtime::invoke` share the same mutation pipeline.
The former exposes a complete typed Resource; a partial field grant requires
`invoke_projected`. The HTTP binding always uses projected results. Request
routing, idempotency, payload and host identity are counted before allocating
the durable mutation identity. Same identity with different semantic input is
`identity_mismatch`. A transport disconnect does not cancel accepted work.

Live state coalesces invalidations and recomputes an authorized bounded snapshot.
It has no replay cursor. Journal facts are ordered and never coalesced. The
journal cursor includes history generation, kind and global position. A batch
may contain no visible facts and still advance over inaccessible or other-kind
history. Cursor positions therefore reveal coarse history progression; they are
not secret or authorization credentials. Raw storage identities and raw rows
are never serialized through the journal endpoint.

Persist the returned journal cursor only after processing its batch. The server
holds no durable consumer acknowledgement. Wrong generation/kind, future cursor,
and a cursor before the retention floor produce `history_gap` (410), including
a missing cursor after the beginning has expired. Never treat this response as
successful processing or silently restart from the retained tail.

After consciously accepting lost history, request `/journal/head`, then obtain an
authorized snapshot, then subscribe from that head. Events after the head may
already appear in the snapshot; reconcile overlap by Resource revision. Retention
can race this recovery and produce another explicit gap. This sequence rebuilds
state; it cannot reconstruct lost business-event processing.

SSE `data` events contain JSON; `error` events contain one safe error category and
terminate the stream. Keepalive comments carry no Resource information. Each
stream owns a ROM subscription permit until drop. Polling also rechecks expiry
without requiring a Resource write; the default poll interval is 100ms. This
performs cheap lifecycle/expiry/local-revocation checks while retaining one pending read across ticks; it does not cancel and restart a slow query. Tune it
against the host's required revocation latency and capacity.

`Limits` independently bounds body bytes, accepted concurrent bodies and body
read time. Core limits bound accepted actions, I/O jobs, subscriptions and snapshot
rows/bytes. Concurrency counts above Tokio's semaphore maximum return configuration
errors before constructing a runtime or HTTP binding. Slow request bodies time out
as `overloaded` (429), and
oversized declared or chunked bodies are `too_large` (413). Other mappings are
403 denied, 404 missing/unregistered, 409 conflict or identity mismatch, 400
invalid/unsupported, 503 closed/not committed/outcome unknown, and 500 internal.
Wire errors omit raw input, credentials, driver errors and source paths.

Use `Http::serve` for coordinated shutdown: intake closes, SSE handles terminate,
and accepted ROM work drains. Hosts using `router()` directly must call
`Http::shutdown()` and coordinate their own server shutdown. Native blocking
business/storage work must terminate; graceful drain is not a hard kill deadline.
Host connection/header limits, TLS, reverse proxy behavior, origin policy and
external-provider interoperation remain deployment responsibilities. This MVP
runs one ROM owner per storage instance; it does not coordinate cross-process
live notifications or worker ownership.

An error or lost response after submission is not blanket proof that no commit occurred. In particular, permissions can be revoked after commit and before disclosure, yielding `denied`; the committed bundle remains. Keep the same idempotency identity for reconciliation, never invent another identity solely because a result was unavailable. There is no unauthenticated receipt-status escape hatch.
