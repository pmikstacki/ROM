# Maintained HTTP and journal evidence

Date: 2026-10-02. This package promotes the focused semantic interfaces selected
by the [transport trial](transport-trial-results.md), then binds them with Axum.
It is maintained code, not the disposable Tower experiment.

## Implementation and ownership

`Invocation` erases a typed command without creating a second execution path.
A tagged operation separates built-ins from same-named custom actions. Actor
identity remains a host parameter and includes principal kind. One bounded
counting serializer checks the complete invocation and host scope before durable
identity allocation. Runtime supervision and permits still own accepted work
after a response future disappears.

Core owns journal cursor, batch, page and subscription semantics. Storage hooks
advertise availability without granting disclosure authority. The persistence
package supplies atomic persisted generation/head/floor and retention in SQLite
and redb. Each runtime page verifies count/byte/order/scope contracts defensively.
It loads current authoritative rows, then uses the shared historical/current field
projection helper. Denied historical facts are omitted while the cursor advances.
No raw durable identity is exposed. Caller-owned cursors are acknowledgments.
SSE does not add durable processing guarantees.

`rom-http` owns generic routing, independent body admission, recursive duplicate
key rejection, safe error categories and HTTP/SSE lifetime. It consumes only
projected mutation/read/query/live/journal results. Separate `/live` and
`/subscribe` contracts make coalescing state and ordered history explicit. See
[the protocol guide](../http.md) for exact shapes and lifecycle requirements.

## Executed evidence

Rerun from the repository root in the supported Rust 1.99.0 environment:

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/target-http" ./scripts/check
```

Observed full-check result: 90 integration tests and 1 doctest passed. The
subprocess crash fixture is marked ignored for direct harness invocation. Its parent conformance test executes it. Formatting, warnings-denied Clippy and
documentation, core without default features, consumer execution, five expected
compiler-failure fixtures and both positive compiler fixtures passed. OpenSpec
validation passed all four active changes. Executed source was parent commit
`1cc7f01` plus the HTTP/journal files and lockfile committed with this report;
no prototype or target artifacts are part of the package.

Focused executable suite:

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/target-http" cargo test -p rom-http --locked
```

`crates/rom-http/tests/loopback.rs` runs real loopback HTTP/1 TCP sockets and real
SQLite transactions. It does not substitute a Tower in-memory oneshot service
for network evidence. The focused suite contains 13 passing tests. It covers:

- Typed/erased/wire receipt parity, unauthorized subject-header forging and
  forged Actor JSON, expired/unverified credentials, altered duplicate identity.
- Nested duplicate keys and unsupported fields, declared and chunked byte
  overflow, a slow body retaining a separate admission permit until timeout.
- Disconnect while an actual SQLite transaction is paused before commit:
  action capacity remains occupied, a second request gets 429, releasing the
  transaction permits same-identity replay with one row/event/receipt.
- Actual commit with injected lost acknowledgment: 503 outcome unknown,
  followed by successful same-identity resolution without a second event.
- Live membership removal, current revocation, idle expiry without writes,
  subscription saturation and permit release on dropped TCP connection,
  graceful shutdown with an open stream.
- Field projection on mutation/read/query/live/journal and denied predicate
  access, including a secret field that never appears in wire responses.
- Distinct ordered journal facts, bounded pages, canceled waits preserving
  unread facts, hidden history, wrong/future cursors, explicit retention loss and explicit fresh-head recovery.
  A shared SQLite/redb test also verifies head persistence across reopen.

The companion invocation tests prove the original missing dispatch failed before
implementation, parity after integration, and oversized kind/action/host scope
rejection before identity construction. Shared persistence tests separately
exercise both database implementations, crash recovery and retained history.

## Libraries versus ROM guarantees

Axum 0.8.9 supplies generic routing, HTTP bodies and SSE from a Rust stream.
ROM supplies authoritative identity/field checks, bounded domain observations,
mutation deduplication, accepted-work supervision and cursor meaning. Axum's
body helper enforces an explicit byte ceiling; this implementation also provides
its own concurrent body admission and read deadline rather than relying on a
default extractor limit. [Axum SSE](https://docs.rs/axum/0.8.9/axum/response/sse/),
[body utilities](https://docs.rs/axum/0.8.9/axum/body/),
[historical body-limit advisory](https://rustsec.org/advisories/RUSTSEC-2022-0055.html).

The adapter pins Axum 0.8.9 with HTTP/1, Tokio and JSON features, and
futures-util 0.3.34 with `std` for stream unfolding. Axum is MIT licensed and
futures-util is MIT OR Apache-2.0, with its published Rust floor at 1.71; the
actual integrated build runs ROM's declared 1.99 floor. Primary dependency
metadata and source are the authority, with the workspace lockfile fixing the
executed transitive versions. [Axum release metadata](https://docs.rs/crate/axum/0.8.9),
[futures-util manifest](https://docs.rs/crate/futures-util/0.3.34/source/Cargo.toml.orig).

The reviewed historical Axum-core, Hyper and futures-util advisories apply to
older versions than this lockfile. This focused source review is not a complete
machine-checked advisory audit. Release audit remains a separate workspace gate.
[Hyper advisory](https://rustsec.org/advisories/RUSTSEC-2021-0079.html),
[futures-util advisory](https://rustsec.org/advisories/RUSTSEC-2020-0062.html).

## Limits and next deployment evidence

One owner per store is the supported topology. Journal cursors expose coarse
history progression; hidden rows and storage identities remain protected.
History reset is explicit, never inferred from an expired cursor. Live streams
are lossy current-state refreshes. Journals preserve distinct retained facts.
Slow consumers retain a bounded subscription and a bounded current batch, plus
HTTP/socket buffers. The tests establish application bounds, not a load-test
memory envelope for arbitrary connection counts or stalled socket writes.

The host still owns verified credentials, TLS, origin policy, connection/header
admission and a process termination policy for native code that will not drain.
A hard shutdown deadline, hostile proxy parsing matrix, distributed
notifications, broker delivery and external identity-provider integration are
not established by these loopback tests. The synchronous resolver must remain
nonblocking; network verification needs host-side bounded preprocessing or a
future asynchronous resolver profile. No per-Resource HTTP controllers are
needed, and Axum/Hyper/Tower remain absent from the driver-free core graph.
