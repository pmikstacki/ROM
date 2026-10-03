# Runtime and native storage ownership

Date: 2026-10-03.
Status: implementation design for release task 4.1. This document does not report executed ownership tests.

## Purpose

Enforce the single-owner deployment contract already stated by `Storage`.
One Runtime owns an adapter's local authority, notifications and work supervision.
One native adapter handle owns a database path until its native connection closes.
An offline operation excludes native owners until that operation finishes.

The core remains independent of paths, database drivers and operating-system locks.
Resource operations, reference integrity and durable work retain their current semantics.
This design does not provide distributed ownership or protect against an administrator who changes files behind ROM.

The [operations audit](../../research/release-operations-gap-audit.md) identifies the existing gaps.
The implementation has two guards because Runtime ownership and native file ownership have different lifetimes.

## Core contract

Add these public interfaces in a named `storage_ownership` module:

```rust
#[derive(Clone, Default)]
pub struct StorageOwnership { /* shared private state */ }
pub struct StorageOwner { /* non-cloneable release guard */ }

impl StorageOwnership {
    pub fn acquire(&self) -> Result<StorageOwner>;
}

// Object-safe method on Storage:
fn acquire_owner(&self) -> Result<StorageOwner>;
```

`StorageOwnership` has one atomic claim flag shared by its clones.
Acquisition returns `Error::Conflict` if another claim exists.
The guard owns the shared state and clears the flag on drop.
It is `Send + Sync`, but cannot be cloned or released by another caller.
Acquisition does not execute application callbacks.

The default `Storage::acquire_owner` returns `Error::Unsupported`.
An adapter must explicitly implement ownership before a Runtime can use it.
This preserves source compatibility for custom implementations, but deliberately rejects an incomplete adapter at build time.
An unconditional successful default would preserve the current correctness gap.

`Builder::build` first performs its pure declaration and limits validation.
It then acquires ownership before reading retry boundaries or registering descriptors.
The resulting guard becomes a field of `execution::Inner`.
If later construction fails, the local guard drops and a new build can try again.

The guard remains held until the final Runtime handle and all accepted work release `Inner`.
`shutdown` drains work and closes intake; it does not release ownership while stopped Runtime clones remain.
Dropping a caller future or a shutdown waiter cannot release the guard.
To construct a replacement Runtime, the host drains the old Runtime and drops its handles.
The host can retain the adapter and then build a replacement on that same adapter.

Native file ownership lasts longer if the host retains the adapter.
Offline maintenance requires the host to drop all adapter handles as well.
This explicit lifetime rule avoids revoking storage authority from existing objects.

## Custom adapters and transparent wrappers

Each independent in-memory store contains one `StorageOwnership` value.
Handles that share the same backing state must share that value too.
Separate helper values in two wrappers around the same store would violate the contract.
Transparent wrappers delegate `acquire_owner` to the wrapped `Storage`.

A native adapter combines this helper with its native guard.
A remote adapter must additionally enforce its deployment-wide exclusion before returning a claim.
The in-memory helper alone does not establish cross-process or cross-host ownership.
An adapter that cannot meet these requirements remains unsupported for Runtime construction.

Direct `Storage` operations remain a trusted host and conformance interface.
The guard does not turn these existing public methods into an unforgeable security boundary.
Hosts must not use direct writes to bypass their owning Runtime's policy and notification pipeline.

## Native lock choice

Use a persistent adjacent lock file and `std::fs::File::try_lock`.
Do not lock the database file through a second independent file handle.
The pinned redb implementation already uses whole-file and range locks; a second lock can conflict with the engine.
See the [redb 4.3.0 backend source](https://docs.rs/redb/4.3.0/src/redb/tree_store/page_store/file_backend/optimized.rs.html).

Rust stabilized `File::try_lock` in 1.89.0; ROM already requires Rust 1.99.
Open the lock file for reading and writing. Map `WouldBlock` to `Error::Conflict`.
Other I/O failures produce `Error::Storage`, except unsupported locking, which produces `Error::Unsupported`.
The guard explicitly unlocks before closing its private descriptor and never exposes a cloned handle.
Incidental fork/exec descriptor inheritance does not extend normal guard ownership.
Drop cannot report an unlock error; descriptor closure remains the fallback.
Abrupt process exit bypasses Drop and relies on operating-system handle closure.
These choices follow the [Rust file-lock contract](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).

Do not delete the lock file on release.
Deleting it could let a contender create a different inode while another process still holds the old lock.
A persistent empty lock file is normal; it is not a stale-owner signal.
Do not inspect or kill a PID to steal ownership.

The initial validated profile is local Linux storage, as used by `rom-dev`.
Unix filesystem metadata supports the alias checks below; other Unix targets require their own acceptance run before support is claimed.
Unvalidated platforms must return `Unsupported`, rather than silently omit the guard.
Network filesystems, remote mounts and distributed operation remain outside the supported profile.
Advisory locks depend on all participating ROM processes following the protocol.
See the [Linux flock documentation](https://man7.org/linux/man-pages/man2/flock.2.html).

SQLite's own transaction locks remain active and unchanged.
They are not a substitute for lifetime ownership of Runtime-local state.
The WAL deployment also remains subject to SQLite's [same-host shared-memory requirements](https://sqlite.org/wal.html).

## Path and lock-file rules

The parent directory is trusted and private to the deployment administrator.
No participant may rename, unlink or replace a database or its lock file while a guard exists.
The protocol does not defend against malicious concurrent directory changes by that administrator.

For an existing database, canonicalize the full path before deriving the lock path.
For a missing database, canonicalize its existing parent and append the unchanged file name.
Reject missing sources for offline operations; never create an empty source as a side effect.
Reject dangling final symlinks and non-regular existing database files.
Resolve symlink aliases to the same canonical database path.
Keep path values as `Path` and `OsString`; do not use lossy UTF-8 conversion or SQLite URI text as identity.

Append `.rom-owner` to the canonical database path to obtain the lock path.
Reserve this suffix for ownership files; reject database names that end in this suffix.
An operating-system path-length failure returns an error, without opening the database.
Create new lock files with mode `0600` and no truncation.
For an existing lock file, require a regular non-symlink file with one link and private permissions.
Check its path metadata against the opened file before locking and again after acquisition.
Propagate metadata errors; absence and access failure are not interchangeable.

After acquisition, recheck the database path and its identity before opening the engine.
An existing database with more than one hard link returns `Unsupported`.
Canonical paths do not unify hard-link aliases in different directories.
This rejection is a limitation of this native implementation, not a universal ROM storage policy.
Do not add a global inode lock registry in this milestone.

The checks cover cooperating opens and maintenance under the trusted-directory assumption.
They do not make unsupported live renames, bind-mount aliases or externally created hard links safe.

## Native helper and ownership transfer

Put the shared file mechanism in named modules in `rom-backup`, alongside its existing native publication support.
Both native adapters already depend on this crate. This placement avoids duplicate platform and path logic.
The helper contains no database driver or Resource behavior.

```rust
pub enum NativeAccess { OpenOrCreate, Existing, Fresh }
pub struct NativeOwnership { /* canonical path and locked file */ }
impl NativeOwnership {
    pub fn acquire(path: &Path, access: NativeAccess) -> Result<Self>;
    pub fn path(&self) -> &Path;
}
```

`Fresh` refuses an existing destination after taking its lock.
`Existing` requires an existing source; `OpenOrCreate` permits initial engine creation.
All acquisitions are nonblocking. Two-source/destination operations cannot wait indefinitely on reversed lock order.
If the second acquisition fails, the first guard drops before returning the error.

Each native adapter stores its native guard after its connection/database fields, so those fields drop first.
Each adapter also stores a separate `StorageOwnership` for Runtime construction.
Its private open helper accepts an already acquired `NativeOwnership` by value.
This helper prevents recursive acquisition when maintenance publishes and reopens a destination.
Normal public open acquires the guard before SQLite connection setup or redb preflight/recovery.
An unsupported format must retain the existing no-source-mutation behavior.

## Offline operations and publication

Upgrade, migration, retention and SQLite index rebuild acquire an `Existing` source guard first.
They retain it through snapshot collection, conversion, destination publication and final reopen.
Do not drop source exclusion when `read_snapshot` returns its owned snapshot.
Acquire a `Fresh` destination guard before staging. Transfer it into the returned adapter.
Source and destination aliases therefore conflict before a transformation can publish.

Restore requires only a fresh native destination guard; its input archive is not a live database.
`backup_to(&self)` uses the existing adapter's native guard and coherent read transaction.
It must not reacquire a lock already held by that adapter.
Backup remains possible while the owning Runtime exists, under the existing snapshot guarantees.

Do not change snapshot bounds, retry epochs, reference rebuilding, receipt replay or SQLite WAL recovery.
Source database and WAL bytes remain unchanged by offline operations.
A newly created persistent `.rom-owner` sidecar is permitted metadata and must be documented in preservation tests.

The current `Stage` publishes through a non-overwriting hard link.
Keep that guarantee. Add a native-publication finalization helper with this order:

1. Close and validate the staged native database; retain the destination reservation.
2. Publish the hard link and sync the destination directory through the existing `Stage` path.
3. Verify that the known private staging file and destination identify the same inode.
4. Remove only that owned staging link, then sync the staging directory.
5. Confirm the destination has one link, then reopen it with the retained destination guard.

The staging adapter must close before publication. Its own temporary-path guard can then drop.
The final adapter must not observe an unlocked interval at the destination.
After publication, a finalization or reopen failure returns `Unknown`; the destination might already exist.
Before publication, an error leaves the destination absent.

If a process exits after the hard link but before staging-link removal, the destination can have two links.
Normal open must fail closed with `Unsupported`, rather than bypass the alias rule.
The operator can finish cleanup under the destination lock after identifying the exact owned staging artifact.
Acquire that existing sidecar lock directly; normal database acquisition intentionally rejects the two-link file.
Verify inode identity and the private stage path before removing that staging link; then sync its directory and retry open.
Never automatically remove unknown links, scan-and-delete all maintenance directories, or remove the destination.
The implementation documentation must give this recovery procedure and distinguish it from a stale lock file.
Deterministic publication tests must exercise this narrow interruption window.

## Evidence required for task 4.1

- Two Runtime builds on the same adapter: only one succeeds; transparent wrappers cannot bypass the claim.
- A failed build releases its claim. Shutdown with a retained clone keeps the claim; dropping the final owner releases it.
- Cancelled callers and drain waiters do not release ownership while accepted work is running.
- Separate handles and processes cannot open the same database, including relative paths and symlink aliases.
- Hard-link aliases, unsafe lock sidecars and unsupported platforms are rejected before native setup.
- A process exit releases OS ownership. A new process recovers persisted work without deleting the lock file.
- Maintenance rejects a live source; an open attempt during conversion or publication also fails.
- Fresh destinations remain reserved through final reopen. Same-source aliases and existing destinations remain refused.
- SQLite committed WAL and redb recovery-copy paths retain source bytes and existing recovery semantics.
- Publication interruption before and after the hard link has the documented destination and recovery outcome.

Preserve the reference race in `tests/persistence/tests/references.rs`.
Its two-Runtime setup becomes an explicit ownership rejection test.
Move the create-source versus delete-target race to two direct concurrent adapter commits with the same original assertions.
Exactly one operation commits; the other conflicts; no dangling reference or extra success event remains.
Ownership complements atomic integrity and does not replace that evidence.

## Scope and decisions

No new product choice requires user approval for this slice.
The accepted single-owner objective determines exclusion; the design fixes lifetimes and native limitations explicitly.
Operator work commands, identity-provider deployment and distributed ownership remain separate release tasks.
No storage/archive format change is required: ownership is process and filesystem coordination, not persisted Resource state.
