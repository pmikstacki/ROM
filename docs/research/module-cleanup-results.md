# Module and DRY review

Reviewed 3 October 2026 against `dcbb44f` in `codex/release-query-index`.
Three agents reviewed separate package groups. The coordinator reviewed their changes.

## Scope

All 14 tracked `lib.rs` and `mod.rs` files already follow the module rules in [the quality gates](../quality.md).
The procedural macro entry points remain at the crate root because Rust requires this location.
Their wrappers delegate to the expansion module.

The review also covered responsibility boundaries and duplicate behavior in the maintained crates, demo and consumer example.
Historical prototypes and worktrees remain unchanged.
The root `AGENTS.md` now directs future agents to the quality gates before code changes.

## Changes

| Area | Change | Contract to preserve |
| --- | --- | --- |
| Core live queries | Share admission, pending generations and acknowledgement in a private subscription module | A failed or cancelled query does not acknowledge a generation; changes during a query remain pending |
| Blob service | Separate configuration, lifecycle, operations and transfer types | Accepted work retains capacity after caller cancellation; shutdown waits for provider ownership to end |
| Blob reads | Share object retrieval and digest/length verification | Duplicate upload and read retain the same integrity checks and errors |
| Authentication | Share the audience representation in the claims module | JWT and introspection keep their feature gates and claim checks |
| CLI | Move response validation from the transport client to a private module | Shape, Resource identity, journal generation and cursor checks remain on both response paths |

Public entry points remain unchanged. The cleanup adds no dependencies.
The review did not combine native maintenance operations that have different validation and transaction guarantees.
The HTTP and CLI JSON parsers remain separate; sharing them requires a separate decision about package dependencies.

## Verification

Focused tests cover live-query cancellation and overload, blob lifecycle, application upgrade, authentication features and CLI response validation.
The coordinator ran `./scripts/check` on the combined changes in the `rom-dev` container. It exited with code 0.
This gate includes strict OpenSpec validation, formatting, Clippy, workspace tests, documentation, consumer fixtures and authentication/identity checks.
The compiler was Rust 1.99.0. The lockfile did not change.
The log is `/var/tmp/rom-module-cleanup-full-check.log` inside the container.

Independent review found no blocking changes to cancellation, generation acknowledgement or provider ownership at shutdown.
Two real-MinIO tests remain explicitly ignored; this run does not establish new MinIO evidence.

This review does not establish that every possible duplication is removed or that the full framework release is complete.
