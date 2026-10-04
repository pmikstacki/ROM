# Offline package recovery for ROM 0.0.2

## Failure evidence

The confined release producer ran source `626cf6ec1c776c25a296a41bee55a8399a2155f4` on 2026-10-04. It passed gates 1–5 and failed at gate 6.

The package command attempted a crates.io index update. The execution namespace permits loopback traffic only. DNS resolution failed before package creation completed.

A separate offline trial used the same 14 maintained crates and source patches. It used Cargo 1.99.0 and a 128 MiB temporary filesystem. Source and Git were read-only. Cargo cache writes used a temporary overlay. The trial did not compile Rust code.

The trial exposed four missing dependency version requirements in `rom-studio-host`. After correction, archive inspection exposed a missing MIT license file in that crate.

## Corrections and verification

The four dependencies now specify `=0.0.2`. The Studio host includes an exact copy of the repository MIT license. The package command now passes `--offline` explicitly.

The corrected trial produced all 14 archives without network access. Each archive contains license bytes identical to the repository license. All 20 package-helper tests passed.

These results prove archive creation and license inclusion for the tested inputs. They do not prove extracted-consumer compilation or completion of the eight release gates. The next producer must run all eight gates against the corrected clean source.

## Retained evidence

The failed producer image remains at `/var/tmp/rom-bounded-producer-Wl7QiD/build.ext4`. Its guardian confirmed unchanged source and cache markers, terminated owned controllers, and detached both loops.

The coordination directory is `/root/ipi/research/disk-coordination-2026-10-04/`.

- `ROM-ca-producer-terminal-status.json` records the terminal result.
- `ROM-offline-package-probe.json` records the failed offline manifest trial.
- `ROM-offline-package-green.json` records all 14 corrected archives and license comparisons.
- `ROM-offline-package-probe.mjs` and `ROM-offline-package-probe.sh` contain the bounded trial.
- `ROM-offline-package-independent-review.md` records the independent analysis before the trial.

The first exploratory trial omitted the last package because its shell input lacked a final newline. It is not evidence for all 14 crates.
