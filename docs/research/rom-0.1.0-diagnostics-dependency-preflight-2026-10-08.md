# Diagnostics dependency preflight

Date: 2026-10-08. Status: candidate investigation; dependency adoption and execution remain open.

The [observability design](rom-0.1.0-core-observability-design-2026-10-08.md) needs restart-stable correlation without exposing raw identities.
The selected candidate is RustCrypto HMAC-SHA256. It does not replace authorization or durable receipts.

## Primary-source inspection

The downloaded [hmac 0.12.1 release](https://docs.rs/crate/hmac/0.12.1) requires digest 0.10.3 with its `mac` feature.
Its README declares Rust 1.41 as the minimum supported version. Its license is MIT OR Apache-2.0.
The published archive includes both complete license files.
The existing ROM lock contains sha2 0.10.9, digest 0.10.7, and subtle 2.6.1.
This candidate keeps the existing SHA-2 interface. It does not claim to be the newest hmac release.

The official registry archive has 42,657 bytes.
Its observed SHA-256 is `6c49c37c09c17a53d937dfbb742eb3a961d65a994e6bcdcf37e7399d0cc8ab5e`.
The crates.io metadata request returned HTTP 403. Cargo must compare the archive with the resolved registry checksum before adoption.
Archive, manifest, README, and license hashes are in `/var/tmp/rom-010-diagnostics-dependency-preflight.json`.

A fresh [RustSec database](https://github.com/rustsec/advisory-db) clone has revision `b8a1a33e246a0a9a3b5f377248c41a503defec74`.
Its commit timestamp is 2026-10-07. This inspection covers the prospective HMAC and existing SHA-2 dependency family only.

| Inspected package | Finding in this snapshot |
| --- | --- |
| hmac, digest, subtle, block-buffer, crypto-common, typenum, cpufeatures | No package advisory directory found |
| sha2 0.10.9 | RUSTSEC-2021-0100 lists patched versions starting at 0.9.8 |
| generic-array 0.14.9 | RUSTSEC-2020-0146 lists patched versions starting at 0.13.3 |

Directory absence does not prove that a package has no vulnerability.
This is not a complete resolved-graph audit or final release security admission.
The final feature graph, registry checksum, complete dependency licenses, and supported ROM compiler still need verification.

## Executed initial contract test

The maintained diagnostics integration suite compiled against the current unmodified public core.
It exited 101 because the selected diagnostic types and `Builder::diagnostics` do not yet exist.
This is the expected missing-interface RED, not an executed behavior or successful implementation.
The exact log is `/var/tmp/rom-010-diagnostics-initial-interface-red.log`.

The diagnostics worker stages implementations outside captured core source during the application recovery trial.
Core integration, actual database regressions, exporter isolation, redaction review, and overhead measurements remain open.

## Current integration evidence

Cargo resolved `hmac 0.12.1` in the current root lockfile, with `digest 0.10.7`.
The registry checksum matches the independently downloaded archive checksum recorded above.
The resolved metadata command completed with exit 0; its native log is `/var/tmp/rom-010-diagnostics-resolved-metadata.json`.

The integrated diagnostic unit suite passed 11 tests, including independent HMAC vectors and bounded publication controls.
Its native log is `/var/tmp/rom-010-diagnostics-unit-integration.log`.
The first work integration run passed eight tests and failed one fixture setup with a missing expected revision.
The first Clippy run rejected six unchanged-error observers that used `map_err` instead of `inspect_err`.
These failures remain in `/var/tmp/rom-010-diagnostics-work-integration.log` and `/var/tmp/rom-010-diagnostics-clippy-integration.log`.
Corrected integration tests, Clippy, overhead measurements, and full release admission remain open.

The corrected integration run passed all nine maintained cases on both database adapters.
The subsequent Clippy run passed with warnings denied.
An explicitly selected overhead test passed and produced 18 measurements across two adapters and three diagnostic modes.
The same command sequence passed `cargo test -p rom`, including 115 unit tests, integration suites, and two doctests.
The native logs are `/var/tmp/rom-010-diagnostics-work-corrected.log`, `/var/tmp/rom-010-diagnostics-clippy-final.log`,
`/var/tmp/rom-010-diagnostics-overhead.log`, and `/var/tmp/rom-010-diagnostics-core-regressions.log`.
These scoped checks do not establish full release admission or installed-consumer acceptance.

The shutdown extension passed ten maintained integration cases and Clippy with warnings denied.
The updated core regression command also passed, including its doctests.
Logs are `/var/tmp/rom-010-diagnostics-shutdown-green.log`, `/var/tmp/rom-010-diagnostics-shutdown-clippy.log`,
and `/var/tmp/rom-010-diagnostics-shutdown-core-regressions.log`.
Shutdown records describe an explicit waiter; cancellation can leave a Started observation without a terminal observation.
`Runtime::status` remains the source of current intake and owned-work state.
Independent review and the full matching-source release verifier remain required.

Independent review found incorrect phase inheritance after authorization and after an empty receipt lookup.
Maintained negative cases reproduced unregistered-resource and stale-revision errors with incorrect diagnostic phases.
The correction adds explicit Resolve, StorageRead, completed receipt lookup, and validation boundaries.
An actual-database wrapper also checks receipt and row-read failures without changing their returned business errors.
The corrected suite passed 13 cases; its overhead case remains explicitly ignored in this ordinary invocation.
Clippy passed with warnings denied.
Logs are `/var/tmp/rom-010-diagnostics-revision-phase-green.log` and `/var/tmp/rom-010-diagnostics-revision-phase-clippy.log`.
The timing description now includes Rayon queue wait and return transit in the supervised Reaction interval.
The earlier 18-row benchmark predates these additional records; it is not final-source performance evidence.
