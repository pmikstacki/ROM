# Maintenance portal declaration increment

Date: 2026-10-08. Status: verified source candidate; Task 4 remains incomplete.

The independent application is in `examples/maintenance-portal`.
Its library imports only public ROM APIs. Module facades contain declarations and exports.
Four Resources share one ownership policy: Equipment, Inspection, WorkOrder, and PortalSettings.
Equipment references and inspection timestamps use existing typed reference and temporal contracts.

## Executed evidence

The first test failed with E0432 because the public application declarations did not exist.
Initial fixture corrections replaced unsupported `u32` and explicitly classified trusted test actors as human principals.
Neither correction established a ROM defect.

The first semantic tests reproduced two incorrect application declarations.
The equipment field exposed a string shape. An invalid inspection date was accepted.
The implementation now uses `ResourceRef<Equipment>` and `rom_fields::DateTime`.
Both corrected field tests passed. Timestamp normalization and descriptor codec identity agree.

Two additional tests executed against real SQLite and redb databases.
Each created all four Resources, checked owner isolation, and invoked completion through the normal action pipeline.
An unauthorized completion left the open WorkOrder unchanged.
Replaying the original command retained revision two and exactly two WorkOrder journal events.

```sh
cargo test --locked --offline --manifest-path examples/maintenance-portal/Cargo.toml
cargo clippy --locked --offline --manifest-path examples/maintenance-portal/Cargo.toml --all-targets -- -D warnings
```

Four tests passed. Clippy exited zero with warnings denied. The declared toolchain is Rust/Cargo 1.99.0.
The command ran through `nixos-container run rom-dev`; the source path was `/workspace/ROM`.
The standalone lock was generated offline. It did not replace the workspace lock.
Source hashes and scoped results are in `/var/tmp/rom-010-maintenance-semantic-evidence.json`.
RED and GREEN logs remain in `/var/tmp/rom-010-maintenance-field-semantics-red.log` and `/var/tmp/rom-010-maintenance-semantic-green.log`.

The local Rust verifier now includes this application and the external AI tool compile fixtures.
The updated full verifier has not run. Other 0.1.0 work remains active.

## Remaining acceptance

No HTTP server, installed Svelte portal, or production identity journey ran in this increment.
Complete the existing Task 4 transport, navigation, history, reference, layout, recovery, and export scenarios.
Run the same application on both databases from admitted source artifacts.
Check the current server authorization boundary instead of substituting synthetic UI scope changes.
Original-consumer acceptance and separate human authoring and accessibility assessments remain required.
These tests do not establish release readiness.
