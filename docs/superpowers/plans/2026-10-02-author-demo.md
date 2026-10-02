# Maintained author demo

The author should see ordinary Rust Resource declarations and small business functions become a working API without per-kind controllers or repositories. Build in `demo/`, preserving the existing consumer example. SQLite is the default and redb is an explicit storage option; both use the same declarations.

1. Declare Task, unrelated InventoryItem with canonical StockCode, Dashboard reaction target, and source-owned Settings. Register explicit row/field policies. Task completion reacts into Dashboard and emits a typed local notification. Verify codec rejection and shared declarations.
2. Bootstrap native User, IdentityProvider and explicit IdentityLink through normal commands under a private Embedded host principal. The synthetic provider remains disabled; HTTP uses a separate limited Embedded loopback demo session, not simulated verified Human evidence. IdentityGate allows only explicit host principals. Seed SourceActivation and load TOML through ReloadTicket. Keep administrative Resources inaccessible from the HTTP actor.
3. Provide a library for declarations/bootstrap, finite `smoke` binary mode, and `serve` mode with a generic worker and graceful Ctrl-C shutdown/status. HTTP binds numeric loopback only. Finite smoke uses actual TCP, creates/reads both kinds, checks invalid codec and administrative denial, performs a typed patch, checks live refresh/journal and reaction notification.
4. Add one-command launch and verifier (locked Rust1.99, bounded jobs, persistent target), README author walkthrough and supported boundaries. Run smoke against both SQLite and redb, fmt/clippy, workspace check as feasible. No production UI, external accounts, credentials or mail delivery claims.

Future provider proof verification is already executable in rom-identity integration tests; demo synthetic trust is explicitly bounded. Blob attachment integration is optional after the independent blob package lands. No core/adapter edits are planned.

## Folder attachment follow-up

Register maintained `rom_blob::definition()` and allow its exact Service worker in IdentityGate. Add a tiny public-API host helper creating a folder adapter and BlobService. The same smoke database will reserve/upload/read, stop BlobService before HTTP/runtime shutdown, reopen database and folder, read persisted Ready attachment without reupload, detach and prove reads stop. Serve exposes no new byte endpoint; it manages the service lifecycle and a synthetic startup attachment. Keep folder roots exclusively host-owned. Apply the parent's blob shutdown-race fix before final combined verification.
