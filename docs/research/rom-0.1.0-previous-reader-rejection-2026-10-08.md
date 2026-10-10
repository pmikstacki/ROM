# Previous-release reader rejection

Date: 2026-10-08. Status: executed upgrade boundary; full release admission remains open.

## Source identity

The probe compiled against accepted 0.0.3 source 584c01b614127b3f62799f1a26a1cdf3f734c3dc.
Its source archive SHA-256 is f18922f9c2a6059570c0bff541deaf379f23c99a02c6185c33c8f3f965a45abe.
All 442 selected source files matched the accepted manifest before compilation and after execution.
The dedicated probe graph contains 91 packages. Each dependency matches the previous release graph.
Initial offline lock generation selected four different dependency versions. Those versions were corrected before compilation.
The unexpected lock remains preserved; it was not used for the accepted build.

The executable SHA-256 is 2154691fe11633087f5cf88670279b8faebbd138ee39d514729cd3496af0d00f.
The build used Rust/Cargo 1.99, two jobs, disabled incremental compilation, and debug information disabled.
Its dedicated target allocated 257052672 bytes after compilation. A hard filesystem quota was not applied.

## Executed cases

The current delayed-upgrade suite passed two tests. It retained populated source and destination databases for both adapters.
The actual previous-release binary ran four controls:

| Adapter | Populated format 8 | Upgraded format 9 |
| --- | --- | --- |
| SQLite | Open succeeded | Exact Unsupported error; main database and WAL unchanged |
| redb | Open succeeded | Exact Unsupported error; main database unchanged |

Each probe ran against an exclusive copy. Original fixture bytes remained unchanged.
Missing paths fail before adapter open. Rejection cannot pass through a missing file or arbitrary error.

The first runner compared every directory entry. SQLite rejection created its empty ownership lock file.
Main database bytes remained unchanged. The runner failed because it included this lock in its data equality assertion.
The corrected runner verifies main database and applicable WAL bytes, and retains all observed file hashes.
No product code changed. The failed runner and its copied databases remain preserved.

## Evidence and limits

Evidence directory: /root/ROM/.superpowers/rom-010-old-reader-rMxXNO.
Its preparation.json, lock-conformance.json, build.log, current-upgrade.log and corrected-result.json record the execution.
The run-data-boundary.mjs runner and first-oracle-failure.json identify the correction.

This proves the previous database reader rejects the current native format for these populated fixtures.
The predecessor fixture uses current format-compatible population followed by the predecessor marker.
It does not establish complete population by an actual 0.0.3 application writer.
Previous-reader rejection of archive format 7 remains untested here.
Whole-application upgrade, fresh-host restore, matching blobs, and production rollback remain separate release gates.
