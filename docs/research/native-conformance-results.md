# Native conformance and author skills

Date: 2026-10-03.
Status: release task 4.4 accepted. Final release tasks 4.5 and 4.6 remain open.

## Native profile

The [native extension guide](../native-extensions.md) defines profile revision 1.
`rom-conformance` exposes shared Field, storage, and optional blob assertions through public Rust interfaces.
Production core and adapters do not depend on this development package.
Native format 8 and archive format 6 remain unchanged.

Field cases contain input, expected typed value, and canonical encoding separately.
Storage scenarios use fresh exclusive fixtures, release owners before reopen, and preserve exact receipt replay.
An obsolete requested profile fails before the fixture factory runs.
Deliberately broken fixtures fail with static case information, without arbitrary adapter errors or stored values.

Existing native tests now call the shared assertions. They retain engine-specific interruption and recovery tests.
The blob profile retains conditional creation, concurrent creation, object bounds, empty objects, and idempotent deletion.
These assertions require a disposable namespace and a sixteen-byte object maximum.

Custom codecs remain versioned through their enclosing Resource and retained historical request codecs.
This profile does not implement independent Field identity, a Field registry, or cross-codec identity collision checks.
Those requirements remain open in the broader `establish-rom` proposal.
Resource versions do not substitute for independent Field versions.

## Author workflows

Four source workflows cover Resource/action authoring, native adapters, uncertain operator outcomes, and local release verification.
The assembler includes their executable assets, canonical references, source paths, and hashes.
The supplied source checkout remains an explicit prerequisite.
The bundle does not contain compiled ROM or every release artifact.

Compatibility admission runs before any workflow example.
The release example runs a finite selection of checks; it is not full release acceptance.
Full release instructions retain the complete source, demo, provider, package, and artifact obligations.

## Baseline findings

An agent attempted all four tasks before reading the new skills.
The Resource and native-adapter tasks passed public API tests, strict Clippy, and formatting after ordinary author corrections.
The actual CLI fault fixture reported an unresolved outcome, preserved the request bytes, and made one contact.
The release baseline checked an extracted committed source artifact and dependency paths.
It inspected, but did not rerun, the full package gate.

The baseline retains missing-trait, deprecated-method, diagnostic-oracle, and container Git-path mistakes.
These are author or environment findings, not newly discovered core defects.
The baseline successes do not support a claim that skills caused behavioral correctness.
The independent evaluator then followed the extracted skills with four concrete tasks:

- A Ticket Resource used lowercase normalization, an approval action, invalid-input rejection, owner denial, and exact replay on SQLite.
- A redb fixture proved fresh factories, exclusive reopen, obsolete-profile rejection before factory execution, and rejection of omitted effects.
- A real CLI `Reconcile` submission lost its response. It returned exit 5, sent one POST, and preserved the exact request file hash.
- Release admission rejected three invalid inputs before Cargo execution. The evaluator compared all 959 files in a source archive with its extracted contents.

The selected release workflow also passed. This was not the final release artifact gate.
The evaluator had prior ROM and baseline experience; this was an independent implementation attempt, not a blind or human-usability study.
No product defect was found in these tasks. Author and environment corrections remain separate evidence.

## Verification boundary

The coordinator ran `./scripts/check`, `./demo/verify`, `./demo/verify-provider`, and the extracted-package verifier successfully.
The external consumer used 13 fresh archives and executed its public conformance tests.
A manifest check then compared the native profile with compiled constants and an actual archive header.
Changing the manifest's native format from 8 to 9 made this check fail; the exact original bytes were restored.
The coordinator reran the complete `./scripts/check` after this test-only amendment. It passed.
All 451 recorded source, configuration, and script hashes matched the final manifest afterward.
The package and demo inputs were unchanged by that amendment.

`./scripts/check-skills` passed seven Node tests and all four extracted workflow examples.
All 32 recorded skill source hashes matched the author's frozen input afterward.
The initial admission implementation could omit indirect assets from its manifest.
A regression reproduced that gap. Assembly and admission now use the same complete asset inventory.
Tests also reject false lock identity, catalog changes, obsolete profiles, missing features, and changed files before execution.

Native, consumer, optional blob, and explicit local MinIO checks also passed in the implementer's runs.
The independent native review accepted the extraction and reran the focused public/native checks.
The explicit MinIO run remains implementer execution, not an independent reviewer rerun.
Independent skill review accepted the implementation and verified the extracted native example and additional admission failures.
The separate author evaluation also passed its four bounded tasks.

[Evidence](evidence/native-conformance-2026-10-03/context.txt) distinguishes source gates, package execution, baseline attempts, and review.
The manifests identify the executed inputs rather than treating a clean ancestor commit as the modified source.

These automated checks do not establish human usability, productivity improvement, universal provider compatibility, or machine power-loss durability.
Documentation follows the repository's STE guidance with fallback vocabulary review; it is not certified ASD-STE100 compliance.
