# Actual mutation recovery acceptance

This fixture uses installed Studio public subpaths and a separate disposable HTTP host.
It does not use private aliases, copied producer dependencies or the shared component server.
The host implements a note action through ROM's public declaration and HTTP APIs.
SQLite and redb store real rows, events, receipts and effects on disk.
Test-only inspection runs under the disposable fixture authority.

Build the fixture host after coordinator allocation:

```sh
cargo build --locked -p rom-recovery-host
```

The coordinator owns workspace and root lock admission.
The host accepts `sqlite|redb DATABASE PORT BUDGET_MS`.
Use an absolute disposable database path.
Port zero selects an available loopback port.
The host has a finite lifetime between 1000 and 600000 milliseconds.
Supply `--host-provenance` with the frozen compile record and matching binary SHA256.
The verifier records compile-time and current workspace lock hashes separately.
Its first stdout line reports the actual address, adapter and database path.
`retain sqlite|redb SOURCE DESTINATION` applies public offline retention into a fresh destination.
It retires epoch zero without silently generating a replacement mutation identity.

Prepare an authoring consumer without native execution:

```sh
node studio/tests/mutation-recovery/verify.mjs /var/tmp/rom-recovery-prepare-UNIQUE --adapter sqlite --prepare-only
```

Run each adapter against the exact supplied source package:

```sh
node studio/tests/mutation-recovery/verify.mjs /var/tmp/rom-recovery-sqlite-UNIQUE --adapter sqlite --host-binary /ABSOLUTE/rom-recovery-host --host-provenance /ABSOLUTE/native-source.json --port 43282 --source-archive /ABSOLUTE/studio-source.tar.gz --admit
node studio/tests/mutation-recovery/verify.mjs /var/tmp/rom-recovery-redb-UNIQUE --adapter redb --host-binary /ABSOLUTE/rom-recovery-host --port 43282 --source-archive /ABSOLUTE/studio-source.tar.gz --admit
```

`--source-dir` accepts an extracted Studio source directory instead of an archive.
The default source is the authoring checkout; this mode cannot admit a release.
Packaged source reports no checkout HEAD when Git metadata is absent.
Its release identity must come from the verified archive witness, not a parent checkout.
Supply ROM_WEBKIT_EXECUTABLE and ROM_CHROMIUM_PATH for the repository's actual browser executables.
Each command has a 180000ms timeout and an 8MiB output limit.
Browser execution uses one worker and five cases per engine.
All generated files stay in a new caller-selected evidence directory.
The verifier refuses an existing directory and preserves failure logs.

The proxy forwards an actual invoke request and reads its response.
Before it drops an acknowledgement, it verifies the adapter's committed receipt and row revision.
Process restart reopens the same database under a new host process.
The HTTP cases compare request bytes and durable counts before and after replay.
They test fresh sessions, mismatched identity, stale revision, deletion ambiguity and retired retry epochs.

The Svelte host selects IndexedDB transactions for pending intent CAS.
It stores editor text separately from wire operations.
Browser cases cover retained C drafts, invalid text, blocked navigation, current revocation and principal quarantine.
An inventory profile applies separate editor validation and navigation labels through the same generic helper.

A process restart is not a power-loss test.
IndexedDB strict transaction completion does not certify every browser or storage device.
The HTTP pending-file fixture provides atomic comparison only within its one test process.
Fixture credentials are fabricated; this does not test a production authentication provider.
These generic acceptance cases do not verify application-specific usability feedback.

The HTTP acceptance entry is compiled by Vite SSR with installed public package imports.
Node cannot strip TypeScript inside node_modules directly.
Inspection retains raw `wire` text for exact integers; convenience JSON number fields are not authoritative.
The proxy suppresses automatic retransmission acknowledgements until explicit resume.
The held-response case checks late principal-change disclosure and actual IndexedDB credential exclusion.
