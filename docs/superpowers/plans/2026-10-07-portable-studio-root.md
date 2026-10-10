# Portable Studio root entry

Status: proposed structural refactor and consumer acceptance plan. No root-entry acceptance has run.

Goal: an application imports ROM Studio's existing App and renderer APIs without private source aliases.

## Source investigation

The root entry exports App from App.svelte. Its component graph contains producer-private $lib imports.
The current source scan identifies 293 files with those imports. Controls, client and recovery have independent portable entries.
Their passing fixture does not prove the root App entry is portable. Preserve that distinction in release admission.

## Selected approach

Use relative imports within the published source graph. Preserve all public root and subpath exports.
Do not require a consumer to recreate $lib, patch installed vendor source or resolve files from the producer checkout.
Do not add a second App implementation or alter control behavior.
A build-time rewrite only in test copies would hide the producer defect and is rejected.
A compiler alias plugin supplied to every consumer exposes private source layout and is rejected.

## Ownership and tests

The coordinator owns this refactor after existing component ownership is released.
Keep R4 recovery sources and active fixture ownership unchanged; they already use relative paths.
Core/native/AI scheduling work is separate. No backend source or live preview changes belong to this task.

1. Add an independently installed consumer using App and registerRenderer from rom-studio.
2. Run its frozen-lock type check/build. Retain the expected unresolved private-alias failure.
3. Replace private imports with correct source-relative paths in maintained source, including transitive shadcn controls.
4. Confirm the consumer installs a physical package copy and has no producer alias configuration or vendor patch.
5. Run the consumer type check/build and two-engine browse through discovery, generic fields, settings and work.
6. Run the existing full Studio unit/component checks and affected source/license/notice/package checks.
7. Obtain independent structural review and run the full local verifier before integrating the refactor.

Keep original exports and runtime semantics unchanged. Preserve supported TypeScript .js-to-source resolution and Svelte file extensions.
Validate import targets rather than treating text replacement counts as proof.
The acceptance must use an extracted candidate first, then the independently verified clean release source.
Historical copied consumers and failed reports remain evidence, not inputs to overwrite.

## Explicit deployment bootstrap candidate

The public Studio entry now exports `parseStudioBootstrap` and `createStudioBootstrap`.
The parser accepts a bounded version-1 JSON document with explicit authority, application namespace, retry epoch, and two IndexedDB store configurations.
The retry epoch is a canonical unsigned decimal string. Its upper bound is `18446744073709551615`.
The parser rejects unknown fields, control characters, invalid integers, and configuration above 65536 UTF-8 bytes.
No origin, provider display label, CSRF token, or session generation supplies an authority namespace.

The host declares each store name, maximum record bytes, maximum slots, and timeout.
Each store reserves 4096 bytes above the configured payload bound for its versioned envelope.
This margin does not bypass actual record-size admission or certify power-loss durability.
The payload bound is at most 1 MiB; each store record bound is at most 4 MiB.
Each store allows at most 4096 slots and an open/transaction timeout at most 60000 milliseconds.
Store failure does not create a volatile fallback. Failure opening the second store closes the first store.

The profile snapshots configuration before opening storage.
A deterministic slot binds application namespace, record type, authority, principal kind, subject, Resource kind, and Resource ID.
The encoded slot must fit 4096 UTF-8 bytes. Another authority cannot use that profile.
Invalid editor drafts and accepted mutation intents remain separate records.
The host receives an idempotent `close` function for both store handles.
Public metadata and stored bytes grant no backend authority.

Five unit cases passed in `/var/tmp/rom-010-studio-bootstrap-snapshot-green.log`.
The intended missing-module RED is `/var/tmp/rom-010-studio-bootstrap-red.log`.
An initial type check found two TypeScript narrowing errors; its output is preserved.
Evidence: `/var/tmp/rom-010-studio-bootstrap-check.log`.
The corrected public entry passed Svelte checking with no errors or warnings.
Evidence: `/var/tmp/rom-010-studio-bootstrap-public-check.log`.

The standalone `main.ts` now reads exactly one deployment-owned JSON script and opens the explicit stores.
The Host configuration can inject that bounded profile into HTML before serving assets.
The normal demo now supplies an explicit deployment profile. Its retry epoch comes from the current storage contract at startup.
It declares the demo Human authority as `local`, independently of the operator authority.
Its command and editor stores have separate names. Its startup profile grants no backend permission.
These unit tests use representative stores; they do not prove browser persistence or consumer acceptance.
The existing App recovery and real-provider identity matrices remain separate release requirements.

The related bootstrap, IndexedDB adapter, and application-session unit suites passed 48 cases after formatting.
Evidence: `/var/tmp/rom-010-studio-bootstrap-related-green.log`.
Candidate source hashes are `/var/tmp/rom-010-studio-bootstrap-source-evidence.json`.
This record is not a clean-source release witness.
Host integration resumed after the original-provider callback source freeze ended.

## Bootstrap contract checks

The Rust Host and frontend use the same version-1 JSON fixture.
The Host passed three bootstrap tests and Clippy for all targets.
Evidence: `container:/var/tmp/rom-010-host-bootstrap-corrected-green.log`.
An earlier assertion expected an unescaped greater-than character. Its failed output remains preserved.
The corrected test verifies the complete escaped JSON roundtrip and unchanged assets after a rejected size limit.

A frontend regression first accepted a Unicode control character that Rust rejects.
Evidence: `/var/tmp/rom-010-studio-bootstrap-unicode-red.log`.
The corrected identifier validation rejects Unicode control characters and lone surrogates.
It preserves valid Unicode scalar identifiers, including supplementary characters.
Bootstrap and IndexedDB checks passed nine tests. Svelte checking reported no errors or warnings.
Evidence: `/var/tmp/rom-010-studio-bootstrap-unicode-green.log` and `/var/tmp/rom-010-studio-bootstrap-current-check.log`.

The current related controller suite has 53 tests: 50 passed and three failed.
The failures cover simultaneous form composition and principal changes. Their implementation remains active.
Evidence: `/var/tmp/rom-010-studio-bootstrap-related-current-failed.log`.
Do not use this aggregate result as a passing release gate.
Current candidate hashes are `/var/tmp/rom-010-studio-bootstrap-current-source-evidence.json`.
Installed App tests, production identity, full verification, and clean-source admission remain open.

## Normal demo host integration

The intended missing-profile API test failed with E0425 before implementation.
Evidence: `container:/var/tmp/rom-010-demo-bootstrap-red.log`.
The corrected demo passed 15 related Studio tests, its binary build, and all-target Clippy.
Evidence: `container:/var/tmp/rom-010-demo-bootstrap-green-build-clippy.log`.

An actual HTTP probe started and drained the compiled demo on SQLite and redb.
Both the Studio root and SPA navigation returned one matching bootstrap profile.
Each host exited with code zero, no signal, and no stderr bytes.
Evidence: `/var/tmp/rom-010-demo-bootstrap-http-JxGpki/result.json`.
The probe used synthetic assets and did not execute a browser.
Probe source: `/var/tmp/rom-010-demo-bootstrap-probe.mjs`.
Current source and binary hashes: `/var/tmp/rom-010-demo-bootstrap-source-evidence.json`.
This is candidate authoring evidence, not clean-source or release admission.
