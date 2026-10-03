# Resource author workshop

This runnable Rust application uses the maintained ROM libraries. It has no per-kind repository, HTTP controller, or alternate schema. The existing Studio screens remain a separate mock; this demo provides a local API and an executable author walkthrough.

From the repository root with Rust 1.99:

```sh
./demo/run smoke                     # finite actual TCP journey, SQLite
./demo/run smoke redb                # same declarations, different adapter
./demo/run reference sqlite          # public-API application recovery journey
./demo/run reference redb            # identical journey on the other store
./demo/verify                       # fmt, Clippy, tests, both smoke commands, docs
./demo/run serve sqlite ./demo.db 8080
```

In the provided development container:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && ./demo/run smoke'
```

`serve` binds only `127.0.0.1`. Ctrl-C closes observation streams and drains runtime work. The final status reports `Stopped` and zero owned work. `serve` prints status on startup/shutdown. Restart the same database to retain Resources, receipts, journal, and pending work. The startup seed uses stable receipt identities and does not overwrite later Resource edits. If seed values or declarations change incompatibly, use a new database.

## 1. Declare two unrelated kinds

[`src/lib.rs`](src/lib.rs) derives `Resource` for `Task { title, done }` and `InventoryItem { code, quantity }`. `StockCode` implements the public `Field` codec. It trims and uppercases input, validates its small alphabet, and declares its string wire shape. The runtime applies that codec to writes **and query values**. Unknown fields and invalid codes are rejected.

Register each definition once in `declarations()`, with explicit policies. The same registration supplies validation, generic mutation, typed selectors, queries, observation, persistence, and HTTP. Adding InventoryItem needed no route or database table code.

```sh
curl -s http://127.0.0.1:8080/invoke -H 'Authorization: Demo local' \
  -d '{"kind":"tasks","id":"first","expected":null,"idempotency":"first-create","operation":{"type":"create","input":{"title":"Draft workshop","done":false}}}'
curl -s http://127.0.0.1:8080/invoke -H 'Authorization: Demo local' \
  -d '{"kind":"inventory","id":"bin","expected":null,"idempotency":"bin-create","operation":{"type":"create","input":{"code":" bolt-7 ","quantity":12}}}'
curl -s http://127.0.0.1:8080/query -H 'Authorization: Demo local' \
  -d '{"kind":"inventory","query":{"filters":[{"field":"code","value":" bolt-7 "}],"limit":10}}'
```

The session header is a **public synthetic demo marker**, not a credential or a production login. Any local process can use it. No bootstrap actor is reachable through that resolver.

## 2. Write a small action or typed patch

`COMPLETE` sets `Task.done`. It knows nothing about databases or HTTP. `rename_task()` demonstrates the generated typed selector:

```rust,ignore
Command::patch(id, Patch::new().set(Task::title_field(), title))
    .at_revision(revision)
    .idempotency("demo-rename")
```

`smoke` sends that same typed command through `/invoke`. A stale revision conflicts. To recover the receipt, retry the identical accepted command with its original idempotency key. Use a new key for a new intent.

## 3. Observe state and committed facts

```sh
curl -N http://127.0.0.1:8080/live -H 'Authorization: Demo local' \
  -d '{"kind":"tasks","field":"done","value":false}'
curl -s http://127.0.0.1:8080/journal -H 'Authorization: Demo local' \
  -d '{"kind":"tasks","after":null}'
```

`/live` sends authorized current snapshots and can coalesce updates. `/journal` returns bounded committed facts and a cursor. To resume, send the cursor as `after`. `/subscribe` streams journal pages. Retention gaps are explicit; these APIs do not promise infinite history. HTTP responses use projected views and omit protected source/deletion metadata.

## 4. React and notify

Complete the task at its current revision:

```sh
curl -s http://127.0.0.1:8080/invoke -H 'Authorization: Demo local' \
  -d '{"kind":"tasks","id":"first","expected":1,"idempotency":"first-complete","operation":{"type":"action","input":{"name":"complete","input":null}}}'
curl -s http://127.0.0.1:8080/read -H 'Authorization: Demo local' \
  -d '{"kind":"dashboards","id":"workshop"}'
```

`Reaction::new` maps a completed Task snapshot into the Dashboard `DISPLAY` action. That action emits `Channel<String>` intent `NOTICE`. The generic worker persists and executes the chain. Reaction and channel actors are explicit Services, checked by the same current policies. Downstream work is asynchronous, so the dashboard can update after the HTTP action response.

The typed receiver records deliveries in an in-process sink and deduplicates stable IDs while the process lives. The smoke asserts one recorded payload. This is synthetic delivery, not email or an external exactly-once guarantee; the sink is not durable across restart.

## 5. Load configuration and establish trust explicitly

Host-only `bootstrap()` creates SourceActivation and loads [`settings.toml`](settings.toml) via config-rs `ReloadTicket`. Settings remain a normal Resource. Whole-kind source ownership, normal field/row policy, expected revisions, source generation, and protected provenance all apply. A failed reload preserves accepted values. The session can read Settings but cannot write Settings or read/update its SourceActivation. The bundled source write permit expires in 2100. It does not make accepted data expire. There is no filesystem watcher or automatic environment overlay.

Bootstrap also creates a User, a **disabled** synthetic IdentityProvider, and an explicit authority/subject/kind IdentityLink. These are ordinary Resources. They are not a working login and do not give the demo session Human authority. `IdentityGate` permits only explicitly configured host principals (including the Blob worker). The app has no privileged public bootstrap endpoint, first-caller admin rule, or email auto-linking.

For actual verified Human/Service actors, follow `rom-identity`'s ProviderActivation → verifier proof → bind flow and its executable signed-token tests. A production host must choose its provider configuration, key/secret acquisition, login/session handling, tenant policy and revocation behavior. The demo resolver must be replaced before deployment.

## Verification and boundaries

`smoke` creates a fresh database and opens real loopback TCP. It tests both kinds and rejects invalid custom input and forbidden administrative access. It observes typed patch and completion updates, tests journal resume, and processes the reaction/notification chain. It then verifies drained shutdown. Integration tests repeat the application on SQLite and redb. No external credentials, mail service, or accounts are required.

This alpha demo deliberately keeps policy and domain functions small. Its local session shares Task/Inventory access; it is not a tenant isolation example. No production UI, distributed consistency, infinite journal, durable external notification deduplication, or blob storage completeness is implied.

## 6. Attach real bytes without another controller

`attachments.rs` registers no endpoint. `declarations()` registers the ordinary maintained Blob definition and explicitly trusts its Service worker in IdentityGate. The host opens an exclusively trusted folder through `rom-blob-object-store`. BlobService then reserves metadata, uploads a bounded stream, verifies digest/length, and reads authorized bytes.

`smoke` persists an attachment, stops BlobService before stopping the runtime, reopens **the same database and folder**, and reads it without uploading again. It then denies another principal, detaches the attachment, and checks that byte reads fail. Both SQLite and redb run this path. The private fixture directory is removed only after all services stop; normal detachment does not physically erase bytes.

`serve` stores the synthetic guide in `<database>.objects` and keeps BlobService alive until Ctrl-C. The shutdown trigger first drains BlobService, which can still need the core to finalize accepted uploads. HTTP/runtime shutdown follows. The metadata can be read through the existing generic route:

```sh
curl -s http://127.0.0.1:8080/read -H 'Authorization: Demo local' \
  -d '{"kind":"blobs","id":"workshop-guide"}'
```

There is no public byte upload/download route. Keep the folder and all ancestors exclusively host-owned. The filesystem adapter is not a symlink sandbox. Backup must account for both metadata and referenced external bytes; a database copy alone is not a complete attachment backup. Physical cleanup requires host-established detachment, grace and quiescence.

## 7. Compensate only an explicitly confirmed business failure

[`compensation.rs`](src/compensation.rs) declares Stock and Checkout Resources and
one reaction. Checkout's durable context exists before the forward reservation.
A **simulated** confirmed payment rejection releases that checkout's token. Unknown
or transient outcomes keep the reservation for reconciliation. This is an
application-authored failure fact, not automatic compensation for a terminal
worker error, HTTP error, timeout or uncertain acknowledgment. No payment provider
is contacted.

Start `serve` on a fresh database. From the repository root, use the generic CLI.
The function below needs no particular target directory or installed binary:

```sh
printf '%s\n' 'Demo local' > /tmp/rom-demo-auth
rom() { cargo run --quiet --locked -p rom-cli -- --endpoint http://127.0.0.1:8080 --auth-file /tmp/rom-demo-auth "$@"; }
rom action reservation-stock workshop-stock reserve --expected 1 --idempotency reserve-a --input-file - <<'JSON'
{"token":"checkout-a","quantity":3}
JSON
rom action reservation-stock workshop-stock reserve --expected 2 --idempotency reserve-b --input-file - <<'JSON'
{"token":"checkout-b","quantity":2}
JSON
rom action checkouts checkout-a record-payment --expected 1 --idempotency payment-unknown --input-file - <<'JSON'
"unknown"
JSON
rom read reservation-stock workshop-stock
# Both reservations remain. The following is an explicit simulated rejection:
rom action checkouts checkout-a record-payment --expected 2 --idempotency payment-rejected --input-file - <<'JSON'
"confirmed_rejected"
JSON
rom read reservation-stock workshop-stock
```

The asynchronous worker eventually leaves `{"checkout-b":2}` with total stock 10.

`ReserveInput { token: String, quantity: u64 }` derives `rom::Input`; the generated
codec supplies the same named arguments to Rust and transported calls. A malformed
quantity reports `reserve.quantity` without echoing its value. New reserve invocations reject the previous demo's
one-entry map payload. Update clients to the named fields
above. Use a fresh fixture for this walkthrough. Stored Resource shapes are
unchanged, but this is an experimental action-input API change, not an automatic
conversion of old requests or their idempotency fingerprints.
An identical historical request with a matching retained receipt still follows
normal authorized idempotent replay before action-input decoding.

A read immediately after the command can still show A. After the worker runs,
read again. Replaying the **identical** final command with its original expected revision
and key returns its receipt and adds no event. Use unique reservation tokens per
checkout. Never recycle a token for a new workflow. This shared local workshop is
not a tenant-isolation or adversarial business-policy example: its synthetic
session can directly mutate these ordinary Resources.

The targeted release changes only its token, and preserves concurrent reservations
and restocks. If the target changes after work materialization, normal revision
conflict handling stops that action instead of rebasing it silently. Revocation
also blocks it. Inspect/reconcile stopped work through trusted host APIs. This
example introduces no privileged public retry route. Committed history remains.

Tests on both adapters count one mapper claim for an unknown outcome, and two
claims (mapper plus target action) for confirmed rejection. Targeted recovery adds
one Stock journal event; receipt replay adds zero. There are no collection scans,
new queues or per-kind routes. Fanout from this mapper is at most one; the existing
runtime bounds attempts, depth, total work and ledger capacity. SQLite/redb tests
also check clean reopen and current service revocation; TCP smoke exercises the
same commands through the existing HTTP API. The separate disposable experiment
covers failure injection and SQLite subprocess exits; the maintained demo does
not include that experimental orchestration harness.

## Reference application: recover committed work

`./demo/run reference sqlite` (or `redb`) runs a finite journey in a fresh private
scratch directory and removes that directory afterwards. It never opens your
`serve` database. Read [`src/reference.rs`](src/reference.rs) for the application
code: only public Runtime, Command, query and live APIs are used.

1. Attach a real file. Persist compensation context. Reserve three units for checkout A and two for B.
2. Record an unknown payment outcome. Reaction processing keeps both reservations.
3. Record a **confirmed** rejection for A. Stop before you process its reaction.
4. Open the same database and folder with the same declarations. Read the file without reuploading. Read a typed, filtered,
   descending inventory query and its next moving page. An unprovisioned actor is
   still denied.
5. Observe the stock query. Recover pending work. After only A is released, the stock enters the query result. B remains reserved. Replaying the original rejection
   adds no event or work.
6. Complete a Task: it leaves its live list and its reaction updates the Dashboard.

The command demonstrates orderly shutdown/reopen. The integration test
`committed_rejection_recovers_after_process_exit_without_shutdown` additionally
runs preparation in a child, exits with code 86 immediately after the rejection
commit (no shutdown or destructors), then recovers in the parent. Both adapters
run that test. This is process-exit evidence, not a power-loss or migration test.
The ignored `reference_process_exit_child` test is a fixture invoked explicitly by
its parent, not a skipped acceptance requirement.

The larger release program remains open: enforced references, schema/format
upgrades, index lifecycle and production identity setup are separate stages in
[the release checklist](../openspec/changes/prepare-framework-release/tasks.md).
