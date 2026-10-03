# Owned query read contract results

Date: 3 October 2026. Base: `28f932c`, branch `codex/release-query-index`.
This report covers the maintained core seam. It does not establish physical native indexes or a measured speedup.

## Delivered behavior

`Storage::query_read` now connects typed, projected and live selection to an owned adapter response.
The default performs one bounded snapshot read. Both maintained database adapters currently use that default.
The core evaluates residual predicates, anchors, order and page limits after response validation.

An explicit `Definition::read_policy` separates actor-only read authorization from row-dependent writes.
A later ordinary policy restores opaque reads. A later field policy removes the uniform field permission.
The new read contract applies to selection and existing disclosure paths; it does not replace field authorization.

The shared selector accepts only permitted scalar candidates with matching request and snapshot identities.
It requires complete candidate support and a strictly lower checked cost.
Absent, stale, unsupported, tied or overflowing estimates select reference execution.
Database plan inspection and coherent execution remain adapter responsibilities.

Response validation checks whole-kind admission, request binding, supported versions, kind identity, duplicate IDs and candidate budgets.
It rejects errors without a second read. Metadata does not independently prove candidate completeness or snapshot truth.
Native adapters must establish those obligations through their integrity model and conformance tests.

## Test evidence

| Evidence | Result |
| --- | --- |
| Explicit policy unit tests | Five tests first failed on missing APIs, then passed; read/write independence, override order and decoder boundaries are covered |
| Protocol and selector unit tests | Nine passed; default fallback, scalar support, stale bindings, arithmetic overflow and response errors are covered |
| Runtime conformance | Eight tests failed before integration, then passed; real SQLite with controlled response wrappers exercises the new operation |
| Existing shared query suite | Nine passed against SQLite and redb; opaque callback ordering and existing value semantics remain covered |
| Core unit suite | All 34 passed after integration |
| Independent review | No blocking findings in authority timing, fallback behavior, shared scalar classification or module boundaries |
| Full local verifier | `./scripts/check` exited with code 0 on the combined tree in `rom-dev` |

The runtime RED showed an explicitly denied empty query returning `Ok([])` and no calls to the adapter operation.
It also exposed the old decoder path and the bypass of fault responses.
After integration, the tests verify denial before query storage, final disclosure rechecks and exact response rejection.

The full verifier includes strict OpenSpec validation, formatting, Clippy, workspace tests, documentation, compile fixtures and authentication/identity profiles.
It used Rust 1.99.0 and the unchanged lockfile. The main log is `/var/tmp/rom-query-read-full-check.log` inside `rom-dev`.
The integration RED log is `/var/tmp/rom-query-read-runtime-red.log` in that container.
The coordinator's focused combined log is `/var/tmp/rom-query-read-combined.log` on the host.

## Design review outcomes

The evaluator and selector now share scalar-shape classification, so optional and nullable support has one definition.
The native response boxes its exact binding to keep the common reference response compact.
The adapter operation accepts no application callbacks; policies and codecs remain outside native connection guards.

The read-policy and adapter contracts are documented in [queries](../queries.md) and [query adapters](../query-adapters.md).
The [implementation plan](../superpowers/plans/2026-10-03-query-read-contract.md) records ownership and the concrete API.

## Remaining stage-three work

The response wrappers test the core boundary; they are not native query executors or performance evidence.
Physical index configuration, atomic index maintenance, native plan inspection, rebuild and recovery remain unimplemented in this slice.
File-backed performance trials must include planner, admission, write and memory costs after that integration.
Release tasks 3.1 through 3.4 therefore remain open, as does the overall release goal.
