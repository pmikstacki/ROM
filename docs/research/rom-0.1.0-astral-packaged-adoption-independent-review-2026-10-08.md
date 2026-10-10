# Independent review: real Astral Rust package adoption

Date: 2026-10-08. Status: the scoped Rust adoption proof passed independent inspection. ROM 0.1.0 admission remains false.

The reviewer inspected the completed capsule `.superpowers/rom-010-astral-packages-US3g7u`.
This review ran no Cargo commands, native builds, test binaries, providers, or browsers. It changed no package or application source.
The previous runner cleanup review remains separate. This report evaluates the real execution evidence, not only `completed: true`.

## Result and scope

No blocker was found for the declared Rust package adoption subset.
The actual Astral Rust application passed 184 cases with candidate ROM packages. One diagnostic-export case was ignored.
Clippy and host compilation completed. Exact package, source, lock, and executable checks support the result.

The packages are dirty-source candidates whose version is still 0.0.3. They are not the accepted 0.0.3 release or an admitted 0.1.0 release.
The evidence identifies `current_dirty_source_candidate` and `release_admitted: false`.
The application retains its own Astrorust dependencies and domain workflows. No application source adaptation was needed for this trial.

## Archive and extraction proof

The capsule contains nine `.crate` archives and their extractions:
`rom`, `rom-auth`, `rom-backup`, `rom-blob`, `rom-derive`, `rom-http`, `rom-identity`, `rom-sqlite`, and `rom-studio-host`.
Each package is version 0.0.3.

The reviewer independently hashed each compressed archive. Every hash matched `archive-extraction-witness.json`.
It read the gzip/tar payloads without extracting or changing files. All 329 packaged file hashes matched both the witness and existing extractions.
The extracted inventories contain no unexplained extra file in the runner's recorded scope.

Cargo creates normalized manifests, package locks, and VCS metadata. These generated files must not be compared as unchanged workspace files.
For each package, `Cargo.toml.orig` matches the captured original manifest.
After this mapping and exclusion of the three generated files, all 302 source payload files match the producer preparation hashes.
The normalized-manifest difference is expected. It is not an unreviewed source patch.

The archive witness SHA-256 is:
`d31485822b21e1dca502679c5d447c1ce00f15155341ff297d5715fddd2704b5`.
This proof identifies the packaged payload. It does not prove final clean-source release artifacts.

## Actual consumer and resolved graph

The original application is `/root/astral-plane`, commit `86dca6fa514895f94fd09ea788ebf07fa21cf648`.
The isolated source copy is `.worktrees/astral-010-public-adoption`, at the same commit.
The executable trial uses a second isolated application under `adoption-evidence-retry-1/application`.

The runner replaces four direct ROM vendor paths with exact package-version dependencies.
Its Cargo patch configuration routes those dependencies to the witnessed extracted packages. It does not patch their source.
All nine resolved ROM manifests point into the exact capsule extraction directories. None points to the ROM checkout or Astral ROM vendor tree.
The final metadata records these crates as local patched sources, not registry downloads.

The reviewer repeated the graph audit using final metadata and the captured locks.
Registry entries match original-consumer provenance; `registryDrift` is empty. No unexplained version, source, or checksum drift was found.
Astrorust manifest paths remain inside the copied original vendor tree. Its original source inventory matches the input witness.

The resolved application lock SHA-256 is:
`a47f842f9b3427f052b6ba057f86239abd7d05188c4a5229a949400ce736946c`.
The captured candidate ROM lock SHA-256 is:
`a58a4ccca73fee85836e6f6a8052d59432f92f10dbd03fabfe09d6111b149ae4`.

## Source preservation and final audits

The reviewer recomputed selected original inputs for both the source copy and `/root/astral-plane`.
The Cargo files, 93 `src` files, 14 test files, 672 knowledge files, and 1,197 Astrorust files all match preparation.
The preserved application identity therefore refers to actual original code, not only a synthetic reference application.

All nine current extracted-package inventories match the recorded package inputs.
The final isolated application inventory matches its expected inputs, including the frozen resolved lock.
`final-input-audit.json` records unchanged original, package, and application observations.
The reviewer recalculated these observations instead of relying only on their boolean flags.

This scope omits build caches, Git internals, frontend files, and unselected operational files.
The runner writes its own new evidence and target paths. The proof does not claim a whole-host preservation scan.

## Execution results and disposal

The reviewer counted the actual `test result` summaries in `tests.stdout.log`: 184 passed, zero failed, and one ignored.
The ignored case is `interpretation::conversation_tests::export_exact_business_provider_request`.
It exports a synthetic provider request for explicitly requested diagnostics. It is not a passing executed case.

Seven command records have code 0, no signal, no stop reason, no spawn error, `groupAbsent: true`, and no output truncation.
They cover compiler identity, initial resolution, frozen metadata, application tests, Clippy, host build, and final metadata.
The test command uses `--offline --locked --all-targets`. Clippy uses those flags and `-D warnings`.
The host build uses `--offline --locked --bin astral-plane`.

Clippy succeeds for the application, but its logs contain Astrorust dependency warnings.
The dependency warnings also appear in build and test logs. This evidence does not support a warning-free entire dependency graph claim.
No ROM application compatibility error appears in the successful trial logs.

The compiler log identifies rustc 1.99.0 and matches its recorded hash.
The reviewer streamed the complete host executable through SHA-256. Its 359,280,904 bytes match the recorded executable identity:
`6dc6d80f4f5423e3477541ad3e6be122f5d5e06070063d053c8aad7e5eff707b`.

## Evidence identities

All paths below are relative to the capsule root.

| Evidence | SHA-256 |
| --- | --- |
| `adoption-evidence-retry-1/acceptance.json` | `1c052a7b7f3d63e2316f2ec93478a131735036c66d77f3ee31dc48fd0c576d3c` |
| `adoption-evidence-retry-1/final-input-audit.json` | `91ad1a21478777e61b0cbf5da6d414ac04329670c60cec65edfdc4222e8a7c7c` |
| `adoption-evidence-retry-1/tests.stdout.log` | `95e1ec5f415bcd7a03c8f5e3b565667b21174670b4f8b392019acc6b2d57dbe0` |
| `adoption-evidence-retry-1/clippy.stderr.log` | `986c0ffabe6a94d1872ee7bf2e29cddb2f2561da7275c3b1cbd86b23556ff804` |
| `adoption-evidence-retry-1/host-build.stderr.log` | `99c943d654e92232238a9534617646f73c6398e5409cb7a72ef64634fedd4d7d` |

Earlier failed and bootstrap capsules remain preserved. This report does not replace their results with the successful retry.

## Remaining release evidence

The application binary was compiled but not started as a deployed host in this trial.
Real provider, network, browser, frontend package adoption, and human usability acceptance remain separate requirements.
The trial does not exercise the optional ROM AI package because this application still uses its existing domain AI implementation.
Final version 0.1.0 packages require another matching artifact and consumer gate after source/version freeze.

The report follows `docs/writing.md` and the ASD-STE100 guide. It does not certify dictionary compliance.
