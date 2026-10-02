# Resource author workshop

This runnable Rust application uses the maintained ROM libraries. It has no per-kind repository, HTTP controller, or alternate schema. The existing Studio screens remain a separate mock; this demo provides a local API and an executable author walkthrough.

From the repository root with Rust 1.99:

```sh
./demo/run smoke                     # finite actual TCP journey, SQLite
./demo/run smoke redb                # same declarations, different adapter
./demo/verify                       # fmt, Clippy, tests, both smoke commands, docs
./demo/run serve sqlite ./demo.db 8080
```

In the provided development container:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && ./demo/run smoke'
```

`serve` binds only `127.0.0.1`. Ctrl-C closes observation streams and drains runtime work; the final status reports `Stopped` and zero owned work. It prints status on startup/shutdown. Restart the same database to retain Resources, receipts, journal, and pending work. The startup seed uses stable receipt identities and does not overwrite later Resource edits. Use a new database when changing seed values or declarations incompatibly.

## 1. Declare two unrelated kinds

[`src/lib.rs`](src/lib.rs) derives `Resource` for `Task { title, done }` and `InventoryItem { code, quantity }`. `StockCode` implements the public `Field` codec: it trims and uppercases input, validates its small alphabet, and declares its string wire shape. The runtime applies that codec to writes **and query values**. Unknown fields and invalid codes are rejected.

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

`smoke` sends that same typed command through `/invoke`. A stale revision conflicts; retry the identical accepted command with its original idempotency key to recover its receipt. Use a new key for a new intent.

## 3. Observe state and committed facts

```sh
curl -N http://127.0.0.1:8080/live -H 'Authorization: Demo local' \
  -d '{"kind":"tasks","field":"done","value":false}'
curl -s http://127.0.0.1:8080/journal -H 'Authorization: Demo local' \
  -d '{"kind":"tasks","after":null}'
```

`/live` sends authorized current snapshots and may coalesce updates. `/journal` returns bounded committed facts and a cursor; send the cursor as `after` to resume. `/subscribe` streams journal pages. Retention gaps are explicit; these APIs do not promise infinite history. HTTP responses use projected views and omit protected source/deletion metadata.

## 4. React and notify

Complete the task at its current revision:

```sh
curl -s http://127.0.0.1:8080/invoke -H 'Authorization: Demo local' \
  -d '{"kind":"tasks","id":"first","expected":1,"idempotency":"first-complete","operation":{"type":"action","input":{"name":"complete","input":null}}}'
curl -s http://127.0.0.1:8080/read -H 'Authorization: Demo local' \
  -d '{"kind":"dashboards","id":"workshop"}'
```

`Reaction::new` maps a completed Task snapshot into the Dashboard `DISPLAY` action. That action emits `Channel<String>` intent `NOTICE`. The generic worker persists and executes the chain. Reaction and channel actors are explicit Services, checked by the same current policies. The dashboard may update after the HTTP action response: downstream work is asynchronous.

The typed receiver records deliveries in an in-process sink and deduplicates stable IDs while the process lives. The smoke asserts one recorded payload. This is synthetic delivery, not email or an external exactly-once guarantee; the sink is not durable across restart.

## 5. Load configuration and establish trust explicitly

Host-only `bootstrap()` creates SourceActivation and loads [`settings.toml`](settings.toml) via config-rs `ReloadTicket`. Settings remain a normal Resource. Whole-kind source ownership, normal field/row policy, expected revisions, source generation, and protected provenance all apply. A failed reload preserves accepted values. The session can read Settings but cannot write Settings or read/update its SourceActivation. The bundled source write permit expires in 2100; it does not make accepted data expire. There is no filesystem watcher or automatic environment overlay.

Bootstrap also creates a User, a **disabled** synthetic IdentityProvider, and an explicit authority/subject/kind IdentityLink. These are ordinary Resources. They are not a working login and do not give the demo session Human authority. `IdentityGate` permits only explicitly configured host principals (including the Blob worker). The app has no privileged public bootstrap endpoint, first-caller admin rule, or email auto-linking.

For actual verified Human/Service actors, follow `rom-identity`'s ProviderActivation → verifier proof → bind flow and its executable signed-token tests. A production host must choose its provider configuration, key/secret acquisition, login/session handling, tenant policy and revocation behavior. The demo resolver must be replaced before deployment.

## Verification and boundaries

`smoke` creates a fresh database, opens real loopback TCP, checks both kinds, rejects invalid custom input and forbidden administrative access, observes typed patch and completion updates, checks journal resume, processes the reaction/notification chain, then verifies drained shutdown. Integration tests repeat the application on SQLite and redb. No external credentials, mail service, or accounts are required.

This alpha demo deliberately keeps policy and domain functions small. Its local session shares Task/Inventory access; it is not a tenant isolation example. No production UI, distributed consistency, infinite journal, durable external notification deduplication, or blob storage completeness is implied.

## 6. Attach real bytes without another controller

`attachments.rs` registers no endpoint. `declarations()` registers the ordinary maintained Blob definition and explicitly trusts its Service worker in IdentityGate. The host opens an exclusively trusted folder through `rom-blob-object-store`, then uses BlobService to reserve metadata, upload a bounded stream, verify digest/length, and read authorized bytes.

`smoke` persists an attachment, stops BlobService before stopping the runtime, reopens **the same database and folder**, and reads it without uploading again. It then denies another principal, detaches the attachment, and checks that byte reads fail. Both SQLite and redb run this path. The private fixture directory is removed only after all services stop; normal detachment does not physically erase bytes.

`serve` stores the synthetic guide in `<database>.objects` and keeps BlobService alive until Ctrl-C. The shutdown trigger first drains BlobService, which may still need the core to finalize accepted uploads; HTTP/runtime shutdown follows. The metadata can be read through the existing generic route:

```sh
curl -s http://127.0.0.1:8080/read -H 'Authorization: Demo local' \
  -d '{"kind":"blobs","id":"workshop-guide"}'
```

There is no public byte upload/download route. Keep the folder and all ancestors exclusively host-owned: the filesystem adapter is not a symlink sandbox. Backup must account for both metadata and referenced external bytes; a database copy alone is not a complete attachment backup. Physical cleanup requires host-established detachment, grace and quiescence.
