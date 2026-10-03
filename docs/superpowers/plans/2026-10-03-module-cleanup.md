# Maintained module cleanup

**Goal:** Remove duplicated behavior and separate responsibilities while preserving the release contracts.

**Architecture:** Keep current public paths through exports. Extract shared decoding and host-input validation without merging different admission policies.

**Baseline:** `f3fbed9`, with the provider deployment checks complete.
The user explicitly requested DRY, cohesive modules, and facade-only crate roots.
The maintained roots already meet the facade rule; the remaining changes concern named implementation and test modules.

## Contract

Preserve serialized names, descriptor versions, fingerprints, retry behavior, authorization order, limits, and storage formats.
Keep historical prototypes, worktrees, and evidence unchanged.
Put implementation and tests in named modules. Crate roots contain declarations, imports, exports, and necessary global definitions.
Procedural-macro entry points retain their existing delegating exception.

The shared JSON decoder returns a duplicate-free `rom::Value` from a complete byte slice.
Expose `rom::parse_json(bytes: &[u8]) -> rom::Result<rom::Value>` with a static invalid-JSON error.
It rejects duplicate object keys at every depth and trailing input. It retains the existing parser recursion limit.
It accepts any JSON top-level value. HTTP retains its separate object admission check; CLI retains its current generic decoding.
Callers continue to enforce byte limits before decoding and map typed schema failures as before.
This uses existing serde dependencies and adds no transport dependency to core.
The configuration syntax-only visitor remains separate unless an extraction preserves its different allocation and provenance contract.

Host text validation checks the same nonempty, byte-bound, trim, and control-character conditions as the current callers.
Credential bytes keep their distinct exact-byte validation. They must not be trimmed by the identifier helper.

## Ownership and tasks

### Task A: JSON decoding and query responsibilities

Owner: `operator_atomic_controls`.
Files: core JSON module and export; HTTP/CLI JSON wrappers; core `query_eval.rs` and named children.

- [x] Extract the shared JSON visitor and retain entry-point shape checks.
- [x] Cover nested duplicate keys, arrays, scalar roots, trailing input, and current safe errors with existing or focused contract tests.
- [x] Separate query normalization, value comparison, and storage-read evaluation without changing their order or public API.
- [x] Run HTTP/CLI/core query checks, strict Clippy, and formatting.

### Task B: Host inputs and configuration lifecycle

Owner: `operator_work_protocol`.
Files: `demo/src/provider_profile/` and its facade; `crates/rom-config/src/ingestion.rs` and named children.

- [x] Extract one host text validator. Retain separate secret-byte acquisition.
- [x] Separate private host configuration from command orchestration and serving composition.
- [x] Separate configuration activation declarations from ReloadTicket lifecycle and error translation.
- [x] Preserve public exports and run provider/config/default-demo checks, strict Clippy, and formatting.

### Task C: Reaction responsibilities and HTTP test organization

Owner: `operator_storage_upgrade`.
Files: core reaction implementation and named children; HTTP loopback test entrypoint and named children.

- [x] Separate reaction declarations, worker ownership, and claim routing while preserving the shared action resolver.
- [x] Extract one HTTP test-support module. Group cases by responsibility without copying fixtures or dropping assertions.
- [x] Run reaction/recovery/native tests and HTTP tests, strict Clippy, and formatting.

### Task D: Independent review and integration

Coordinator owns shared scripts, package gates, evidence, and integration.

- [x] Review each extraction against its baseline and public paths.
- [x] Run the full local verifier, demo acceptance, provider acceptance, and extracted-package consumer.
- [x] Record executed source identities, limits, and review results.
- [x] Integrate only after all checks pass. Continue release tasks 4.4–4.6 afterward.

## Review focus

- HTTP object admission must not become a CLI-wide policy.
- Query extraction must preserve authorization before limit and native candidate validation before disclosure.
- Configuration extraction must preserve source generations, reload ownership, and safe errors.
- Reaction extraction must preserve accepted work across caller cancellation and current-authority checks on recovery.
- Test extraction must retain every test and fixture lifecycle, including bounded child cleanup.

This is a behavior-preserving refactor under the user's existing cleanup and release authorization.
It adds no dependency, independent Field registry, new storage format, or deployment.

## Result

All three source slices passed independent review. The coordinator full, demo, provider, and extracted-package sequence exited with code 0.
The [cleanup report](../../research/release-module-cleanup-results.md) retains source hashes and evidence.
