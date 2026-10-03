# Storage ownership

ROM uses two ownership guards. The core guard permits one Runtime for a shared store.
The native guard permits one ROM database handle for a local filesystem path.
These guards protect worker supervision and live invalidation. Database transactions still enforce atomic commits and reference integrity.

## Runtime lifetime

`Runtime::builder().build(...)` acquires the store's claim before reading retry boundaries or registering descriptors.
A second builder returns `Conflict`, including through a transparent wrapper.
A failure after acquisition releases the failed builder's claim.

The claim stays with the Runtime's shared state. Accepted work retains that state when its caller stops waiting.
Shutdown stops intake and drains accepted work. It does not release the claim while a Runtime clone remains.
To replace a Runtime, await shutdown and drop all its handles.
If the host retains the storage adapter, a new Runtime can then claim that same adapter.

The native guard has a separate lifetime. It remains held while the native adapter exists.
Before offline maintenance, drop the old Runtime and all native adapter handles.
A backup through `backup_to(&self)` uses the existing owner and a coherent read transaction; it does not acquire a second owner.

## Custom adapters and wrappers

Each shared backing store needs one `StorageOwnership` value.
Its `acquire_owner` implementation calls that value's `acquire` method:

```rust,ignore
fn acquire_owner(&self) -> rom::Result<rom::StorageOwner> {
    self.ownership.acquire()
}
```

The fragment assumes `self.ownership: rom::StorageOwnership` belongs to the shared backing state.
Cloned store handles must share it. Independent stores use independent values.
Transparent wrappers forward `acquire_owner` to the wrapped adapter instead of creating a new helper.
The default trait method returns `Unsupported`, so an incomplete custom adapter cannot construct a Runtime.

The helper only provides process-local exclusion. A remote adapter also needs exclusion across its supported deployment.
Direct `Storage` calls remain trusted host interfaces for conformance and maintenance.
They do not provide authorization and must not bypass the owning Runtime's actions or notification pipeline.

## Native deployment profile

The validated target is local Linux storage in the ROM development container.
Other platforms return `Unsupported` for native ownership. Network filesystems, bind-mount aliases and distributed writers are outside this profile.
The deployment administrator must control the database directory.
Do not rename, unlink or replace database files or their ownership sidecars while an owner exists.

Native adapters resolve symlinks to a canonical path before opening the engine.
An existing hard-linked database is unsupported. Canonical paths alone cannot unify hard-link aliases across directories.
The adjacent `.rom-owner` sidecar is a private regular file with one link.
The suffix is reserved; a database cannot use it as its own filename.

The sidecar persists after normal shutdown and process exit. Its presence does not indicate an active owner.
The operating-system lock provides exclusion. Never delete the sidecar to acquire ownership.
Lock contention returns `Conflict`; unsafe sidecars and unsupported aliases return `Unsupported`.

SQLite preserves the exact `:memory:` and empty-path sentinels as independent ephemeral stores.
Each such adapter still has a Runtime claim, but no filesystem ownership guard.
SQLite URI inputs beginning with `file:` return `Unsupported`; use filesystem paths for persistent databases.
An explicit path such as `./file:literal` remains an ordinary filename.

## Offline operations

Upgrade, Resource migration, retention and index rebuild reserve both source and fresh destination before conversion.
They retain those reservations through staging, publication and reopening.
Concurrent ROM opens of either path return `Conflict` during that operation.
Callbacks cannot redirect publication by changing the current directory: maintenance uses the guarded canonical paths.

The source database and committed SQLite WAL remain unchanged by offline maintenance.
A persistent ownership sidecar can be created as coordination metadata.
Restore reserves its fresh native destination; its input archive is not a live native database.
No storage or archive format bump is necessary for these process and filesystem guards.

## Interrupted native publication

Maintenance publishes a complete staged database with a non-overwriting hard link, then syncs the destination directory.
It removes only the known private staging link, syncs that directory, and reopens the destination under the retained guard.
Before publication, failure leaves the destination absent. A failure after publication returns `Unknown`.

A process exit between publication and staging cleanup can leave two links to the complete database.
Ordinary open rejects that state. This is different from a persistent, unlocked `.rom-owner` file.

For recovery, retain the destination and its exact private `.rom-maintenance-*` directory from the interrupted operation.
Acquire the existing destination sidecar's lock directly; normal native acquisition rejects the two-link database.
Under that lock, verify that the destination and known private `data` artifact are regular files with the same device and inode.
Both must have exactly two links. Reject an unknown third link, a symlink, or a different inode.
Remove only the verified private `data` link. Sync its directory, then confirm that the destination has one link.
Release the lock and open the destination normally. Never remove the destination or scan and delete unrelated maintenance directories.

The publication tests execute this procedure with a child process interrupted before and after publication.
The helper and native conformance tests also cover aliases, process exit, current ownership, and reservation failure cleanup.
The release report records the executed checks; these scenarios alone do not establish power-loss certification.
