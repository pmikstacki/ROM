# ROM command-line client

`rom-cli` builds the `rom` executable. It uses the generic `rom-http` protocol;
adding a Resource requires server registration, not a CLI command or client schema.
The CLI does not open application databases directly. Studio and login/session
ceremonies are outside this release.

Build and check from the repository with Rust 1.99:

```sh
cargo build -p rom-cli
./crates/rom-cli/verify
./target/debug/rom --help
```

If you set `CARGO_TARGET_DIR`, the executable is under that target instead.
The source distribution includes the optional CLI alongside the libraries. To run
the remaining examples from the repository without installing the binary, define:

```sh
rom() { cargo run --quiet --locked -p rom-cli -- "$@"; }
```

This respects `CARGO_TARGET_DIR`. Alternatively, use
`cargo install --path crates/rom-cli --locked` and ensure Cargo's bin directory is
on your PATH.

## Connect and inspect

Start the workshop in another terminal:

```sh
./demo/run serve sqlite ./demo.db 8080
# redb uses the identical declarations and CLI protocol:
# ./demo/run serve redb ./demo.redb.db 8080
```

The workshop's `Demo local` header is a **public synthetic marker**, not a real
login or a private credential. Create a local fixture file, then run:

```sh
printf '%s\n' 'Demo local' > /tmp/rom-demo-auth
rom --endpoint http://127.0.0.1:8080 --auth-file /tmp/rom-demo-auth discover
rom --endpoint http://127.0.0.1:8080 --auth-file /tmp/rom-demo-auth discover tasks
rom --endpoint http://127.0.0.1:8080 --auth-file /tmp/rom-demo-auth query tasks
```

`--endpoint` is always explicit. A base path is preserved, so
`https://example.org/rom-api` sends reads to `/rom-api/read`. HTTPS verifies the
server certificate. Plain HTTP is accepted only for numeric loopback addresses
(including IPv6), not hostnames. URL credentials, query strings and fragments are
rejected. Redirects, proxies and automatic network retries are disabled.

`--auth-file` is optional: the host decides whether a request without credentials
has authority. When used, the file contains the complete Authorization value
(e.g. its host-defined scheme and credential), not a JSON document. A single final
LF or CRLF is removed; embedded line breaks, empty values and files larger than
8 KiB are rejected. Credentials never enter command-line flags or diagnostic
messages. The CLI does not save, refresh or acquire credentials; protect the
input file according to the host's credential policy.

Discovery returns only deliberately disclosed metadata. It does not grant read,
query, mutation or action access. `discover KIND` selects from that authorized
snapshot; unavailable kinds are not distinguished from hidden kinds. Known-kind
commands work without discovery. Custom action input remains opaque: consult the
application's action contract, and let its registered codec validate the value.

## Commands and JSON input

Global connection/output options can appear before or after a command.
`--output human` is the default and prints readable, indented JSON with IDs and
revisions intact. `--output json` prints compact JSON: one value for a finite
result, one complete value per line for live/journal frames. Human and machine
output both escape terminal control characters. Diagnostics go to stderr.

| Command | Input and meaning |
|---|---|
| `discover [KIND]` | Authorized metadata snapshot or one disclosed kind. |
| `read KIND ID` | Current projected Resource, including its revision. |
| `query KIND [--query-file FILE]` | All by default, or a bounded equality/conjunction query. |
| `create KIND ID --idempotency KEY --input-file FILE` | Complete Resource value; expected revision is null. |
| `replace KIND ID --expected REV --idempotency KEY --input-file FILE` | Complete replacement, not a partial patch. |
| `patch KIND ID --expected REV --idempotency KEY --input-file FILE` | Explicit `set`/`remove` field operations. |
| `delete KIND ID --expected REV --idempotency KEY` | Delete at an explicit revision. |
| `action KIND ID NAME --expected REV --idempotency KEY --input-file FILE` | Registered custom action, separate from standard operations. |
| `invoke --request-file FILE` | Exact generic Invocation envelope, including its original identity and expected revision. |
| `live KIND [--query-file FILE]` | Whole authorized current snapshots; no replay cursor. |
| `journal KIND [--after-file FILE]` | One bounded journal batch and its cursor. |
| `journal-head KIND` | Current cursor; explicitly establishes a new checkpoint. |
| `subscribe KIND [--after-file FILE]` | Ordered journal batches with their cursors. |

Every JSON source flag accepts `-` for stdin. Requests and input files are limited
to 64 KiB, with duplicate object keys rejected at every nesting level before
conversion. Unknown request/query/invocation properties are rejected. Values retain
exact signed/unsigned 64-bit integers; downstream JSON tools must also preserve
them if they will replay a request.

For example, create one Resource:

```sh
rom --endpoint http://127.0.0.1:8080 --auth-file /tmp/rom-demo-auth \
  create tasks first --idempotency first-create --input-file - <<'JSON'
{"title":"Draft workshop","done":false}
JSON
```

Then patch it using the revision from the response:

```sh
rom --endpoint http://127.0.0.1:8080 --auth-file /tmp/rom-demo-auth \
  patch tasks first --expected 1 --idempotency first-rename --input-file - <<'JSON'
{"title":{"op":"set","value":"Review workshop"}}
JSON
```

Omitted patch entries remain unchanged. `{"op":"set","value":null}` writes
an explicit null where supported; `{"op":"remove"}` removes an optional field.
False, zero and empty strings are values. Projected reads can omit protected
fields, so do not convert a projected read blindly into a complete replacement.

A query file can contain:

```json
{"filters":[{"field":"done","value":false}],"after_id":null,"limit":20}
```

Predicates are conjunctive equality or scalar comparisons. Optional-field absence uses
`{"field":"memo","value":null,"absent":true}`. Query files may also supply `comparisons`,
`order` and a client `after` anchor; see [shared query semantics](queries.md).
`after_id` remains available for ID-only ordering. Moving pages retain no snapshot
and provide no total count. Collection snapshot bounds still apply with a small page limit.

## Mutations and uncertain outcomes

Supply an explicit idempotency key for each intended mutation. Non-create
mutations also require the expected revision, including in a raw invocation.
Keep the original request when reconciling an unavailable result. Repeating the
same principal-scoped identity and semantic input recovers its authorized durable
receipt; changing input with the same identity is an identity mismatch. After a
conflict, a deliberately revised mutation is a new intent and needs a new key.

**Every failed remote mutation response is treated as unresolved.** HTTP error
categories do not identify the execution phase. Even conflict, validation,
size-limit, denial or unavailable-resource errors can arise from a host authority
check after commit. The CLI shows the safe category without claiming rollback.
A malformed, oversized or lost success response is unresolved too. A local input
error before submission is distinct.

Replay only as the same principal, with the original key, expected revision and
semantic input. The CLI cannot certify that a replacement credential still
represents the original principal. It never retries automatically or stores
pending requests/credentials. `invoke --request-file` accepts the existing wire
Invocation shown in [HTTP documentation](http.md); it does not invent another
mutation identity.

## Streams and process behavior

`live` emits full snapshots, initially and when state changes; updates can coalesce.
`subscribe` emits whole journal batches, including empty batches whose cursors
advance. Flush each complete output frame before consuming another. A successful
pipe write does not prove the receiving application processed or persisted it.
Persist a journal cursor only after your application has processed its batch.

Finite response bodies and individual SSE frames are limited to 2 MiB. Parsing
handles arbitrary chunk boundaries, split UTF-8, CRLF, comments and multiline data.
Response identities must match the requested kind/ID. Journal batches must preserve
the supplied and preceding cursor generation, kind and forward position, with
ordered events inside each cursor range. Malformed/oversized/incomplete streams stop visibly. There is no automatic
reconnect, unbounded frame queue or implicit history reset. Keepalive bytes count
as connection activity, not application progress.

| Timeout option | Default | Accepted seconds |
|---|---|---|
| `--connect-timeout` | 10 | 1–300 |
| `--request-timeout` | 30 | 1–3600 |
| `--idle-timeout` | 30 | 1–3600 |

The request timeout bounds response-header waiting and, separately, finite body
consumption. Stream bodies instead use the inactivity timeout. A slow stdout
consumer applies backpressure; no additional output frames are queued. Ctrl-C
also exits while blocked on stdin/stdout. Closing stdout terminates cleanly. The interrupt handler does not write diagnostics,
so a full stderr pipe cannot prevent exit; use its exit status.
Stopping a mutation only stops waiting and does not cancel accepted server work.

A history gap requires an explicit decision about recovery. To rebuild state after
accepting lost history: obtain `journal-head`, obtain an authorized snapshot, then
subscribe from that head and reconcile overlap by Resource revision. This cannot
recover missing business-event processing, and retention can race recovery again.

| Exit | Meaning |
|---|---|
| 0 | Success, intentional non-mutation interruption, or stdout pipe closure. |
| 2 | Invalid arguments, credentials, endpoint or local request input. |
| 3 | Explicit remote rejection of a read/discovery/observation operation. |
| 4 | Read/stream transport, response protocol or output failure. |
| 5 | Mutation submitted; outcome unresolved, including any non-success reply. |
| 6 | Read/stream history gap. |
| 130 | Interrupted while waiting for a submitted mutation; outcome unresolved. |

The executable tests run two unrelated Resource kinds against real HTTP on SQLite
and redb, with actual stream updates and revocation. Fault coverage includes lost
commit replies, post-commit denial and arbitrary host-gate errors, identical retry,
identity mismatch, exact integers and partial presence, malformed inputs/frames,
redirect rejection, stream inactivity, broken output pipes and signal interruption.
