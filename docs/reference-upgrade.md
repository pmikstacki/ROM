# Upgrade the reference application

Run the finite acceptance journey from the repository root:

```sh
./demo/run upgrade sqlite
./demo/run upgrade redb
```

Each command creates private scratch files and removes them when the journey ends.
It does not upgrade an existing `serve` deployment. Use the public maintenance
methods in the demo's upgrade module to inspect the host-side integration.

## The application change

Checkout version 1 stores a stock ID as text. Version 2 declares
`stock_id: ResourceRef<Stock>`. Its JSON value remains a string. Its descriptor now
requires a live target and prevents target deletion while a live reference exists.

The application retains the exact version-1 type and codec. Migration constructs
the typed reference and rebuilds the live reference index. An invalid ID or missing
target prevents publication. The current definition registers the version-1 replay
codec so old receipts retain their meaning.

The two application profiles share composition and business rules. They differ in
the Checkout definition and reaction binding. They do not define separate database
tables, transport controllers or per-kind repositories.

## The journey

The old profile reserves stock for two checkouts. An unknown payment result leaves
both reservations in place. A confirmed rejection then creates durable pending
compensation for one checkout. Maintenance runs before that work is processed.

The host migrates into a fresh database, writes a backup, and restores it into
another fresh database. The current profile then recovers the pending work. It
releases only the rejected reservation. Receipt replay adds no new mutation,
event or queued work. The remaining reference scenario checks typed queries,
live membership, current authority and the Task-to-Dashboard reaction.

A separate pending checkout refers to stock with no reservations. Deleting that
stock fails because of restrict. Deleting the nonterminal checkout removes the
reference; stock deletion then succeeds. This distinguishes reference enforcement
from the stock rule that protects outstanding reservations.

The attachment's metadata survives database maintenance. Its bytes remain in the
external folder. Database archives do not contain or restore those bytes. Keep
the folder with the deployment when you use this model.

## Failure and compatibility boundaries

The command stops the old runtime cleanly. Integration tests additionally exit a
child process after the rejection commit without shutdown or worker drain. They
then run maintenance and recovery. A process-exit test is not a power-loss test.

Maintenance requires exclusive offline source ownership and fresh destinations.
The [ownership guards](storage-ownership.md) enforce exclusion across ROM native opens and maintenance.
Drop all old Runtime and adapter handles before maintenance; shutdown alone does not release retained handles.
Existing destination files must not be overwritten. Keep the source until the new
deployment passes acceptance. A dirty redb source is recovered through a private
copy. The original source remains unchanged.

SQLite maintenance uses an immutable read for a canonical source path without a
WAL. When a WAL exists, it uses a read-only transaction that includes committed
WAL records. Both paths require exclusive offline ownership. SQLite SHM lock
bookkeeping is outside the byte-preservation check.

An existing rollback journal produces `Unsupported` before SQLite opens the
source. Do not delete that journal to bypass the error. Preserve the source and
all sidecars. Complete SQLite recovery on a separate copy under host control,
then use that recovered database as the maintenance source.

This journey proves a Resource schema upgrade with the maintained storage format.
Explicit older native/archive format upgrades have separate conformance tests.
It does not run an old ROM binary or provide online cutover. The local demo actor
is synthetic and must not be used as production authentication.

See [Resource migrations](resource-migrations.md), [backup maintenance](../crates/rom-backup/README.md)
and [retention](retention.md) for the library contracts.
