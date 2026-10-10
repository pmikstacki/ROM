# Astral Rust adoption from ROM packages

Date: 2026-10-08. Status: runner implemented; native adoption has not run.

Astral revision `86dca6fa514895f94fd09ea788ebf07fa21cf648` uses four direct ROM vendor dependencies.
They are `rom`, `rom-sqlite`, `rom-identity`, and `rom-studio-host`.
Its transitive dependencies also include other ROM crates.
The existing ROM package check uses a reference application. It does not execute the Astral application tests.

## Isolation and dependency contract

The new runner is `scripts/packages/astral-adoption.mjs`.
It takes an explicit consumer directory, extracted package root, candidate lock, evidence path, target path, and execution budgets.
Evidence and target paths must be new. They cannot be inside the original consumer or extracted package root.
Selected source files and extracted packages cannot contain symbolic links.

The runner copies `src`, `tests`, `knowledge`, both Cargo files, and the original Astrorust source tree.
It excludes build caches and does not copy ROM vendor sources.
Astral needs `knowledge/library.json` during compilation through `include_str!`.
Astrorust remains an unchanged application dependency. It is not a ROM source override.

Only the four direct ROM dependency lines change.
Each uses the exact extracted package version.
The local Cargo configuration routes registry resolution to these extracted package directories.
This configuration does not patch package source code.
The original features, development dependencies, workspace declaration, lints, and Astrorust paths remain unchanged.
Unsupported ROM dependency options cause an explicit error.

Every resolved ROM package must match the expected name, version, exact manifest path, and captured file inventory.
Registry dependencies must match the original consumer lock or the candidate ROM lock, including source and checksum.
The acceptance record lists dependencies that differ from the original lock and explains their candidate-lock origin.
Unknown local dependencies and ROM checkout paths are rejected.

The caller must first establish that package extractions match the witnessed `.crate` archives.
The runner records and rechecks extracted file inventories. It does not independently prove archive-to-extraction identity.
This prerequisite remains part of the source artifact workflow.

## Execution

The initial offline metadata command resolves a new isolated lock.
It does not use `--locked`, because the candidate dependency graph can differ from the original consumer graph.
After the graph audit, all commands use the frozen lock.

1. Run offline, locked Cargo metadata.
2. Run the actual Astral application tests with `--all-targets`.
3. Run Clippy with `--all-targets` and warnings denied.
4. Build the `astral-plane` host executable.
5. Audit the final graph, package inputs, original source inputs, and lock identity again.

Each command uses two build jobs and disables incremental compilation.
Each deadline is explicit and cannot exceed 30 minutes.
The output budget is explicit and cannot exceed 32 MiB per command.
A deadline, output overflow or parent SIGINT/SIGTERM terminates the owned process group and records failure.
Successful command exit also triggers descendant cleanup.
The command refuses success if its bounded TERM/KILL cleanup cannot confirm process-group absence.
A failed run records final source inventories without replacing the primary command error.
Successful acceptance requires a written, unchanged final source audit.
A failed run retains its evidence and cannot produce an acceptance record.
These bounds do not establish a filesystem quota or an in-flight allocation bound.
The coordinator must admit the native build and its disk allowance separately.

Use a JSON options file:

```json
{
  "consumer": "/absolute/path/to/unchanged-astral",
  "packageRoot": "/absolute/path/to/verified-extracted-packages",
  "candidateLock": "/absolute/path/to/candidate/Cargo.lock",
  "evidence": "/absolute/path/to/new-evidence",
  "target": "/absolute/path/to/new-target",
  "metadataSeconds": 120,
  "testSeconds": 600,
  "clippySeconds": 600,
  "buildSeconds": 600,
  "outputBytes": 33554432
}
```

Run the following command only after native admission:

```sh
node scripts/packages/astral-adoption.mjs /absolute/path/to/options.json
```

## Current verification and limits

Thirteen native-free tests pass, including independently reproduced process and audit regressions.
They check manifest preservation, isolated copies, path escapes, symbolic links, unknown packages, graph mutation, registry drift, and missing execution budgets.
They also cover successful-child cleanup, both interruption signals, deadline/output failures, and source drift during a failed command.
See [independent runner review](rom-0.1.0-astral-adoption-runner-review-2026-10-08.md).
These tests do not run Cargo or prove that Astral compiles with candidate ROM packages.
Native application tests, executable identity, and deployment remain pending.
This proof also does not replace the separate installed Studio package and browser acceptance checks.
