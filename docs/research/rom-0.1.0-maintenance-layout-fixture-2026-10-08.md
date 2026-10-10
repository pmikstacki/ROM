# Persisted layout and browser backend fixture

Date: 2026-10-08. Status: tested source candidate; connected browser admission is in progress.

PortalSettings now stores the full layout through the normal Resource field and mutation contracts.
The wire value is JSON document text. It contains the public layout item fields without a separate persistence mechanism.
Application-owned validation admits only the history, equipment, and work widget identities.
It rejects duplicates, unknown keys, invalid visibility values, excessive items, and out-of-bounds geometry.
The complete Settings transition also validates geometry against its current column count.
Ownership checks still run before application validation.

The valid-layout test failed before the field existed.
After correction, the valid layout and five malformed cases passed.
All seven application tests passed on the updated source. All-feature Clippy passed with warnings denied.
The fixture feature is included in the local Rust verifier.

## Private fixture protocol

The optional `rom-maintenance-http-fixture` binary binds an ephemeral IPv4 loopback port.
It uses the application's public declarations and the existing generic HTTP adapter.
Its fixed credential resolver is shared with the HTTP tests.
It creates no Resource-specific transport or repository.

The command requires these arguments:

```text
--fixture-only <sqlite|redb> <absolute-database-path> <absolute-ready-path> <absolute-stop-path>
```

All paths must share one existing directory. Ready and stop paths must be distinct and unused.
The fixture publishes `lane`, `address`, `base_url`, and `database` in an exclusive readiness file.
It seeds pump-1, valve-1, check-1, repair-1, and workspace.
It drains when the stop file appears or after 300 seconds.
Fixed credentials are test inputs, not production authentication.

## Native evidence

The binary was built with Rust/Cargo 1.99, two jobs, incremental compilation disabled, and the locked offline dependency graph.
An exclusive private copy preserves the executable independently of the mutable build target.
Streaming hashes matched before copying, after copying, and on the preserved file.
The preserved inode differs from the build target.

- Native identity: `/var/tmp/rom-010-maintenance-native-e7Sabk/identity.json`
- Preserved binary: `/var/tmp/rom-010-maintenance-native-e7Sabk/rom-maintenance-http-fixture`
- Binary SHA-256: `660ae055aec7ef0734b01b54da147cf5f2cd1faf3560de7f05ceac64d9bf4b8d`
- Binary size: 116039208 bytes
- Source and manifest entries: 178

Actual host smoke tests launched and drained the binary separately on SQLite and redb.
Alice read the persisted layout with HTTP 200. Bob's read of Alice's equipment returned HTTP 403.
The result is `/var/tmp/rom-010-maintenance-fixture-smoke-ptjc9m/result.json`.
This is source-authoring evidence. It is not admission of a release archive or production deployment.

## Remaining work

Run the installed Svelte portal against this exact backend identity on both databases and actual browser engines.
Verify interruption, exact retry, selection, dates, references, history, layout persistence, and principal changes.
Public guest compute/export, production identity integration, original-consumer acceptance, and human accessibility remain separate open requirements.
Run the updated complete verifier and independent review before integration.
