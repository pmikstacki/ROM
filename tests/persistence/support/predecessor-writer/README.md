# Accepted-release writer fixture

Build this fixture against the verified accepted 0.0.3 source, not the current workspace.
Accepted revision: `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.

Replace `@SOURCE_ROOT@` in the manifest template with that verified extraction's absolute path.
Copy the fixture into a fresh private evidence directory. Preserve its source hashes before compilation.
Use a dedicated lock. Compare every dependency version with the accepted release lock before compilation.
Use a separate target, two build jobs, no incremental compilation, and an explicit command deadline.
Record the target allocation separately from its planned allowance. A planned allowance is not an enforced quota.

The `populate` operation uses the predecessor's public storage contract and native adapters.
It creates a live Resource, a tombstone, mutation receipts, effects, pending work, an active delivery, and an operator receipt.
It exports the predecessor's archive and a private logical snapshot for later comparison.
It never rewrites the storage marker. Existing output paths fail before publication.

Run the maintained current-reader trial explicitly:

```sh
ROM_PREDECESSOR_EVIDENCE=/absolute/path/to/evidence \
  cargo test -p rom-storage-conformance --test predecessor_writer --locked -- --ignored --nocapture
```

The evidence parent must contain separately populated `sqlite` and `redb` directories.
The test compares native upgrade data and fences predecessor cursors and active claims.
It also converts the predecessor archive into a new current archive without changing its source.
The test has an explicit ignore annotation because ordinary workspace tests cannot supply a separately witnessed historical executable.
A default workspace test run does not establish this release gate. Execute the command above with verified inputs.

The `read-archive` operation accepts the predecessor archive. The `reject-archive` operation requires `Error::Unsupported` for a current archive.
Supply separate private copies through each operation's `before.rombk` path. Compare archive bytes before and after execution.
Keep historical source, failed trials, and all input evidence unchanged.

This fixture does not establish whole-application upgrade, rollback, historical application codec replay, or packaged release acceptance.
