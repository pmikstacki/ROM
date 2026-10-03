# Deferred research, core ergonomics and CLI results

Date: 2026-10-02. This completes the owner's revised follow-on scope after the [experimental MVP](mvp-release-results.md). That scope covers research of every deferred area, better API ergonomics/resilience, and a generic CLI. Studio implementation is excluded. The [37-topic roadmap](deferred-capabilities-roadmap.md) describes researched future work. It does not claim that those features are implemented.

## Result and rationale

The maintained Resource model now offers typed `Query::all()`/`Default`, rejects impossible concurrency configuration through `Result` instead of a semaphore panic, and supplies explicitly authorized metadata discovery. Discovery is default-deny, bounded, current-authority checked, deterministic and separate from operation permissions. Hidden references cannot reveal undisclosed kinds. Core still imports no concrete database or transport.

The optional `rom-cli` package supplies the `rom` executable over the shared HTTP protocol. The same commands operate Task and Inventory Resources on both maintained adapters. No client-side repository, schema copy or per-kind command is needed. Human output is escaped pretty JSON; machine output is complete JSON/NDJSON. Local arguments and requests are validated before transmission, and no credentials, pending requests or automatic retry state are persisted. See the [CLI guide](../cli.md) and [selection research](cli-implementation-research.md).

The key resilience result is explicit uncertainty. A failed response does not identify whether an action committed. A host policy can fail during post-commit observation. Every unsuccessful submitted mutation therefore yields unresolved outcome; recovery uses the original principal-scoped key, revision and semantic input. Live snapshots and journal batches retain different semantics. Contextual response validation rejects unrelated Resources, backward cursors, generation changes and invalid event ordering. Interrupt handling exits even when output destinations are blocked.

## Verification

Implementation source commits: core `36815b3`, discovery `b05cabf`, HTTP/demo `e858d7e`, CLI `1d7a640`; integrated CLI revision `1d1d622`. The independent exhaustive discovery budget regression is `4fab19d`. Later documentation/notices commits do not change those executable sources.

Environment: native persistent `rom-dev` NixOS container, Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`. Locked Cargo SHA-256: `35a128d90a9c31bc03f0e073cdba52e819bffb565bfb05cfd34a8261bf112e7d`.

| Check | Executed result |
| --- | --- |
| Combined `scripts/check` at `1d1d622` | Passed: five strict OpenSpec changes, formatter, warning-free workspace Clippy, workspace tests/doctests, public documentation, compile-negative cases, external-style consumer, core dependency isolation and auth/identity verifiers. |
| Focused CLI verifier | Passed: 26 tests — one parser, three command/input, thirteen fault, three real TCP journey and six streaming tests. |
| Core independent review | Five API/capacity, eleven discovery, three HTTP/demo tests and an additional exhaustive catalog-byte-boundary probe passed. |
| Dependency notice collector | Two tests passed; generated inventory covers 270 external packages. All prior notice sections remain byte-identical; fifteen package additions contribute 29 license texts. |
| Dependency audit | No vulnerabilities or warnings against local advisory database commit `117edb3bed98e9be112f277b7615eea3252e7c43` (1,280 entries), using `cargo audit --no-fetch`. This records that database snapshot, not a future security guarantee. |
| Extracted source packages | Passed: twelve archives built with all features outside the workspace; extracted `rom --help` ran, then the packaged external consumer passed. Root lockfile remained unchanged. |
| Independent final CLI review | Passed at review source `66864f2`, identical to integrated code/tests/Cargo `1d1d622`: formatter, Clippy, 26 tests and all three independent fault reproductions. No outstanding P1/P2; [independent report](core-cli-independent-review.md). |

Actual binary/TCP coverage includes complete operations on two kinds/SQLite/redb, custom action calls, live updates and revocation, journal continuation, null/omission/removal/false/exact `u64`, idempotent replay/mismatch, committed mutation plus lost HTTP reply, post-commit revocation and arbitrary host-gate errors, duplicate JSON, malformed and oversized inputs/frames, redirects, response identity and cursor checks, pipe closure, inactivity and interruption with saturated stdout/stderr. The fixture provider is explicitly synthetic; these tests do not establish a production login profile.

Retained local logs: `/tmp/rom-core-cli-full-check.log`, `/tmp/rom-core-cli-packages.log`, `/tmp/rom-core-cli-audit-notices-check.log` on the host; `/tmp/rom-cli-verify.log` and `/var/tmp/rom-core-cli-audit.json` in `rom-dev`. Extracted archives and build evidence are retained in `rom-dev:/var/lib/rom-package-checks/rom-packaged-consumer-OIS0qb`. The reviewer log is `rom-dev:/var/tmp/rom-independent-cli-review.log`. These paths are execution evidence, not repository dependencies. Reproduce the local checks with the commands below.

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && ./scripts/check'
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && ./crates/rom-cli/verify'
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && TMPDIR=/var/lib/rom-package-checks node scripts/check-packages.mjs'
```

## Scope limits

This remains experimental software. Automated correctness and diagnostics tests do not establish satisfaction for outside application authors or production performance. Linux was executed; cross-platform source/license inspection does not certify macOS/Windows binaries. Native callbacks remain trusted, terminating host code. There is no new storage format, multi-writer profile, interactive login, retained client credentials, automatic stream reconnection or mutation retry. Discovery action input remains opaque. Metadata visibility never grants mutation/read access. The deferred roadmap records the prerequisites for richer metadata, login, additional transports, retention/migrations, telemetry and other extensions. Studio remains deferred.
