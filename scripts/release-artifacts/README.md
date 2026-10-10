# Produce local source and Studio artifacts

Run the producer from a clean ordinary Git checkout:

```sh
./scripts/release [OUTPUT_DIR]
```

For versions 0.1.0 and later, the default output is `../rom-releases/rom-VERSION-SHORT_REVISION/`.
The completed output must be outside the producer checkout.
Historical 0.0.x releases keep `dist/rom-VERSION-SHORT_REVISION/`.
An explicit relative output path is relative to the caller's directory.
Use an output parent outside the source checkout for the public-consumer profile.
An ignored output parent inside the checkout is permitted only for the historical profile.
The producer rejects an existing output, including a dangling symbolic link.
It does not overwrite old flat archives or publish to a registry.
No argument or environment flag can skip its acceptance commands.

## Required inputs for 0.1.0 and later

The public-consumer profile requires complete operational evidence before production begins.
Set these four environment variables:

| Variable | Required value |
| --- | --- |
| `ROM_RELEASE_REQUIREMENTS_DIRECTORY` | Canonical absolute directory containing `record.json`, executed evidence, and independent reviews for R1–R14. |
| `ROM_RELEASE_REQUIREMENTS_SHA256` | SHA256 of that exact `record.json`. |
| `ROM_RELEASE_SQLITE_PROFILE` | Canonical absolute path to the selected SQLite input JSON. |
| `ROM_RELEASE_SQLITE_PROFILE_SHA256` | SHA256 of that exact input JSON. |

The requirement record must match the committed source identity. Every requirement must be accepted with executed evidence and an independent review.
Static validation verifies identities and record structure. It does not establish behavior or replace review of the evidence.
Missing, pending, failed, or mismatched requirements prevent production. These variables cannot skip a gate.

The SQLite input object has exactly three keys: `archive`, `cc`, and `ar`.
Each value identifies a canonical absolute file path. Compiler and archiver paths must identify executable regular files.
The archive must be the official SQLite 3.53.4 autoconf archive. Admission verifies its published SHA3-256 digest.
The builder verifies the C source digest and header identity, then builds a private static library.
It retains the source archive, licensing reference, commands, logs, and output hashes.
Native acceptance separately measures the Rust-linked version, source ID, and required compile options.
A successful C build alone does not satisfy native acceptance.

## Host requirements

Use Linux with Node.js 22, the repository's Rust floor, Git, GNU tar/gzip/coreutils, and `findmnt`.
GNU `mv` must support `--no-copy` and `--update=none-fail`.
The tested publication host uses coreutils 9.5 and ext4.
The producer requires ext4 and probes rejection of an existing destination before running gates.
The stage and completed output must share a filesystem.
These requirements concern trusted local process interruption and publication. They do not prove power-loss durability.

Git metadata must be visible to the host running the command.
A mounted worktree whose Git directory points to an unavailable host path is insufficient.
Dependency caches, actual browser runtimes, and provider fixture prerequisites must be available for the complete gate.
The frontend package step requires npm's cache for `npm ci --offline`.
Chromium and WebKit are both required. A mock browser cannot satisfy these gates.
Retain the existing Cargo target through `CARGO_TARGET_DIR` and `ROM_PACKAGE_TARGET_DIR` when appropriate.
Each Cargo gate receives two build jobs.

## Fixed acceptance commands

Both Studio profiles execute these first eight commands in order:

```sh
./scripts/check
./scripts/build --release
./demo/verify
./demo/verify-provider
./scripts/check-skills
node scripts/check-packages.mjs ROM_ROOT
./scripts/studio-browser-runtime-check
./demo/verify-studio --assets-dir ABSOLUTE_EXTRACTED_ASSETS
```

The package gate executes core library tests from the extracted package.
It also checks the copied consumer and reference application. Test fixtures must remain within their owning packages.

After the seventh gate, the producer copies complete frontend inputs into a private directory.
It excludes generated frontend directories and rejects conventional private environment and key files.
It runs these fixed commands in the copy:

```sh
npm ci --offline --no-audit --no-fund
npm run build
```

The package step checks that original and copied source identities remain unchanged.
It archives the production assets, extracts them, and verifies the complete asset inventory.
It removes the copied inputs and dependency cache before the eighth gate.
The eighth gate receives this extraction and uses the actual host, backend, provider, Chromium, and WebKit.
It does not substitute development assets. The producer checks the asset inventory again after the gate.

The first eight gates each have a one-hour deadline and a combined 32-MiB output bound.
Public-consumer gates have separate 1,200-second bounds. Supplemental package and AI commands each have 2,400-second bounds.
The native preparation shares a 1,200-second deadline across its commands.
These command limits do not constitute a complete-producer allocation quota or aggregate deadline.
Admit an aggregate execution budget and monitor disk allocation before starting production.
The shared Linux process helper retains and terminates its owned process group on a bounded abort.
The manifest records exact arguments, source directory, timestamps, exit status, and separate output logs.
The producer checks clean status, HEAD, tree, tracked content, and lock identity before assembly and before publication.

## Artifacts and verification

The complete directory contains source, skills, and production Studio archives, `manifest.json`, `SHA256SUMS`, and command evidence.
The source archive comes from the exact committed revision.
Tracked links, cache paths, and conventional private credential paths are rejected.
No ignored private host files are copied.
Source archive buffers have a 128-MiB limit; an oversized source fails before publication.
Studio archive buffers have the same limit.

Verification rejects unsafe archive paths and links before extraction.
It compares the complete extracted file inventory and digests with the captured source.
Git reconstructs the extracted tree without filters, including executable modes.
The Git archive commit marker, tree identity, selected source identity, and lock digest must match.
All four extracted skills preflights must accept the extracted source.
That admission does not claim that all examples ran from the source extraction.
The earlier skills gate separately executes those examples against the original source.

Checksums cover every artifact file except `SHA256SUMS` itself.
The manifest hashes its payloads; the checksum file also hashes the manifest and gate logs.
The selected skills identity covers workspace Cargo files and Rust/Cargo files under crates, demo, and examples.
It excludes `tests/persistence`; Git tree and source archive identities remain separate records.

Releases 0.1.0 and later require manifest version 3 and profile `rom-public-consumers-v3`.
The producer retains operational requirement evidence and independently reviewed references.
It also executes three additional gates from the verified source extraction: public controls, SQLite recovery, and redb recovery.
A fresh native recovery host runs against the selected SQLite profile.
Supplemental acceptance executes the extracted package checker, packaged AI consumer, and named diagnostic tests.
Execution targets remain outside the final artifact. Selected regular evidence and produced crate archives are retained.
Source-context mappings preserve the original report bytes; they do not rewrite execution results.
Older 0.0.x Studio manifests use version 2 and verification profile `rom-studio-v2`.
They bind the frontend package version and lock identity to the exact extracted source.
They bind all extracted production asset files to the recorded eighth gate.
Historical version 1 manifests retain their six-gate `source-only` contract.
Their verification does not claim Studio acceptance. A Studio payload cannot use that earlier contract.

The producer builds a private sibling stage and verifies the complete directory before moving it.
GNU `mv -T --no-copy --update=none-fail` performs the no-replace move.
A late collision preserves the existing output and retains the losing stage.
Failure evidence has `completed: false`; its directory is not a completed release.

## Focused tests

```sh
node --test scripts/packages/*.test.mjs scripts/release-artifacts/*.test.mjs
```

Tests use private committed fixtures and the real archive, assembler, extraction, and admission code.
They inject a finite trusted gate runner directly into the module API.
The CLI does not expose this injection.
These tests prove artifact behavior; they do not replace the real complete producer invocation.
Publication fixtures use `/var/tmp`, which is ext4 on the tested host.
Set `ROM_RELEASE_TEST_TMP` to another existing ext4 parent if necessary.
Tests do not change maintained source and retain their small fixture directories for diagnosis.

## Standalone Studio package check

Use an unused output directory and a source checkout with the required native and browser prerequisites:

```sh
node scripts/check-studio-package.mjs ROM_ROOT OUTPUT_DIR
```

This command runs the same fresh locked build and actual-host acceptance on extracted production assets.
It rejects an existing output directory. It retains package and command evidence.
It has no argument to skip browsers or substitute an existing build.
Its success is a Studio package check. It does not replace complete producer acceptance or the additional 0.1.0 requirements.

## Verify completed artifacts independently

From the supplied source checkout, run the verifier against the completed artifact directory:

```sh
node --input-type=module - "$ARTIFACT_DIRECTORY" <<'JS'
import { verifyArtifacts } from './scripts/release-artifacts/verification.mjs';
const manifest = await verifyArtifacts(process.argv[2]);
console.log(JSON.stringify({
  complete: manifest.complete,
  source_revision: manifest.source.revision,
  manifest_version: manifest.manifest_version,
}));
JS
```

The verifier reconstructs source and asset identities and validates the recorded gate contract.
It does not rerun browsers or claim that checksums alone establish application behavior.
