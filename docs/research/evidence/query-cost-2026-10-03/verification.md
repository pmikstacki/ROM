# Query measurement verification

Date: 2026-10-03. Base commit: `9db4b2ab4bb090a940f452eb1039c77c9d1c8aab`.
Workspace: `/root/ROM/.worktrees/release-query-index` on the host.
Container workspace: `/workspace/ROM/.worktrees/release-query-index` in `rom-dev`.

The [final source manifest](final-source.sha256) identifies the checked Rust sources, manifests and verification scripts before commit.
The measurement archives preserve the earlier frozen binaries' source separately.
The only later executable change is the harness guard against duplicate dataset sizes and its regression.
The measured trials all use distinct sizes. Production selector code did not change after measurement.

## Executed checks

Both final commands used Rust and Cargo 1.99.0 in `rom-dev`, with these environment variables:

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-release-measured-verification-target
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
```

| Command | Result | Evidence |
| --- | --- | --- |
| `./scripts/check` | Exit 0 after the duplicate-size guard | [Full verifier log](final-check.log) |
| `./demo/verify` | Exit 0 after the duplicate-size guard | [Reference application log](final-demo.log) |
| Duplicate-size regression before the guard | Failed on the intended assertion | [RED log](duplicate-size-red.log) |
| Harness tests and Clippy after the guard | Six default tests and five all-feature tests passed; Clippy passed | [GREEN log](duplicate-size-green.log) |

The full verifier includes strict OpenSpec validation, formatting, workspace Clippy, tests, doctests and documentation warnings.
It also checks the core without default features, the external consumer, compile fixtures, and the auth and identity profiles.
The demo verifier exercises SQLite and redb, then checks serve, reopen and SIGINT drain.
These commands do not establish stage-four operational ownership or final package readiness.

## Measurement checks

Each of calibration, baseline validation and tuned validation contains 1,728 observed queries and 576 ordinary production controls.
Each heap run contains 48 scopes. The write comparison contains 24 fresh-process workloads and 72 write phases.
The harness compares complete ordered projected views outside each measurement interval.
The coordinator also compared result counts and digests for all 48 workload keys across both seed-203 runs.

Both source archive checksums passed `sha256sum -c`.
The JSON summaries retain individual metric distributions and sample counts.
The report states the shared-host, ordering, seed, allocator and small-sample limits.

## Independent review

The [specification review](../../measured-query-final-spec-review.md) found no correctness or scope blocker through source inspection.
The [quality review](../../measured-query-final-quality-review.md) identified the duplicate-size issue, then confirmed its repair.
Reviewers did not claim to reproduce the full verifier or the performance runs.

Documentation uses the repository's ASD-STE100 guidance and declared technical vocabulary.
Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary.
Full compliance requires verification against the official standard. This report does not certify compliance.
