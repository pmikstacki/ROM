# Maintained module cleanup

Date: 2026-10-03.
Baseline: `f3fbed9927a1b515628604e4aad63559049d4abf`.
Status: implementation, independent review, and combined verification passed.

## Audit and scope

The maintained crate roots already followed the facade rule before this pass.
Eleven library roots and the demo root contained declarations, imports, exports, and necessary shared definitions.
The two procedural-macro entry points delegated to expansion modules. They remain the documented exception.
The maintained CLI test `mod.rs` contained a declaration and reexport.

The audit found two concrete duplication problems: recursive JSON decoding and host text validation.
It also found named files that combined declarations, lifecycle, or request processing.
The refactor addresses those responsibilities. It does not split coherent transactions only to reduce file length.
Historical prototypes, worktrees, and evidence remain unchanged.

## Changes

| Area | Shared implementation or module boundary | Preserved contract |
| --- | --- | --- |
| HTTP and CLI JSON | One public `rom::parse_json` decoder; separate caller wrappers | Duplicate keys fail at every depth. HTTP retains object admission; CLI retains generic roots and its schema checks. |
| Provider inputs | One private bounded text validator | Credential bytes retain their separate exact-byte validation. Identifier validation does not trim secrets. |
| Provider commands | Configuration, command orchestration, and serving modules | Explicit provisioning, approved references, signal registration, and drain order remain unchanged. |
| Queries | Normalization, comparison, and storage-read modules | Authorization, bounds, cursor checks, candidate bindings, and selection order remain unchanged. |
| Reactions | Declaration, worker, claim, and routing modules | The shared action resolver, accepted work ownership, current authority, and recovery contracts remain unchanged. |
| Configuration | Activation, ticket lifecycle, and error modules | Public paths, source generations, reload identity, and safe error mapping remain unchanged. |
| HTTP tests | One support module and named case groups | Existing assertions and bounded fixture ownership remain present. |

The shared decoder uses existing serde dependencies.
Its callers still enforce input byte limits before decoding.
The configuration parser's syntax-only duplicate check remains separate because its allocation and provenance responsibilities differ.

An optional `ROM_PACKAGE_TARGET_DIR` lets package verification reuse dependency builds.
Each run still creates archives, extracts fresh sources, and audits the consumer's dependency paths.
It does not turn a workspace build into evidence for an extracted package.

## Acceptance record

The coordinator ran this command sequence in `rom-dev` with Rust 1.99.0 and two build jobs:

```sh
./scripts/check
./demo/verify
./demo/verify-provider
node scripts/check-packages.mjs
```

The complete sequence exited with code 0. The package consumer used 12 freshly extracted archives.
Both native provider journeys passed, including natural expiry and fresh-token recovery.
The [combined log](evidence/module-cleanup-2026-10-03/combined.log) retains the executed results.
All 464 entries in [the source manifest](evidence/module-cleanup-2026-10-03/source-sha256.txt) matched after execution.
The [verification context](evidence/module-cleanup-2026-10-03/verification-context.txt) records paths, settings, and source provenance.

Three independent reviews accepted the source changes with no open findings.
A separate review accepted the optional package-cache change.
Author checks, source review, and coordinator execution remain separate evidence categories.
The audit and raw reports are retained in the same evidence directory.
The earlier [module review](module-cleanup-results.md) remains unchanged as historical evidence.

The refactor preserves public paths, serialized names, fingerprints, descriptor versions, query versions, and native/archive formats.
The additive decoder is the only new public function in this cleanup.
Release stages 4.4–4.6 remain open for extension conformance, author skills, final application/package acceptance, and artifacts.

## Writing review

This report follows the [ROM writing rules](../writing.md).
Technical terms, paths, versions, and evidence values retain their meaning.
Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary.
Full compliance requires verification against the official standard.
