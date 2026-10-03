# Query integration foundation results

Date: 2026-10-03. Baseline: `383b03e` plus the changes committed with this report.
This is structural preparation and semantic characterization for release stage 3.
It does not claim implemented native query selection or an index performance gain.

## Cohesive core modules

`resource.rs` now exports five responsibility modules: field codecs, typed query
authoring, definitions, commands and schema validation. Its public crate paths
remain unchanged. Definitions and their erased policy adapter stay together.

`execution.rs` now exports composition, shared state, lifecycle, authority and
command execution modules. The complete mutation sequence remains in one command
module. Receipt lookup, current authority, retained codec replay, proposal work,
race checks, atomic commit and disclosure keep their original order.

The runtime still drops adapter-owning references before it publishes the drain
condition. Work completion releases its permit before it decrements the active
count and notifies waiters. Visibility changes only restore the previous internal
execution-module scope between the new child modules. Public APIs do not expand.
The two former implementation files are facades with explicit exports. Child
modules use explicit imports. Root `lib.rs` no longer imports unused runtime types.

An independent comparison checked all 40 execution functions and eight type
definitions against the baseline. Bodies and shapes matched apart from formatting
and the necessary internal visibility. The resource split received the same
section-level comparison. These comparisons support the refactor; behavior tests
remain necessary.

## Query characterization

Three new regressions run through public APIs on SQLite and redb:

1. A readable row with a protected sort field still produces `Denied` when a
   predicate, anchor or page limit excludes it from the result.
2. An ID-ordered page stops before a later opaque-policy panic. When the next
   query reaches that row, the panic fails the runtime.
3. A predicate cannot hide a policy panic that occurs earlier in the reference walk.

These are characterization tests of existing behavior. They are not tests of an
implemented index. They bring the shared query-planning target to nine tests.

## Design findings

The [semantic audit](maintained-query-semantic-gates.md) identifies callback and
decoder evaluation as observable behavior. An arbitrary callback is not a proof
that native candidate pruning preserves errors. Existing opaque policies need the
reference path until a separate explicit contract permits omitted evaluations.

The [index layout proposal](maintained-index-layout-design.md) separates native
physical indexes from logical Resource descriptors. It proposes exact scalar keys
and separate counters for stored JSON text and canonical Row bytes. Their values
can differ for older noncanonical native JSON.

The selected direction in the [draft design](../superpowers/specs/2026-10-03-maintained-query-strategies-design.md)
is an adapter-owned coherent read operation. Native planning and materialization
run in one transaction. Application callbacks run after native guards are released.
This prevents policy reentry into a held storage mutex. The proposed actor-only
read contract is explicit; it does not silently change existing row callbacks.

## Executed verification

The coordinator ran `./scripts/check` in `rom-dev` on the combined changes. It
passed with Rust 1.99.0. The command covers strict OpenSpec validation, formatting,
Clippy with warnings denied, workspace tests/docs, core dependency isolation,
consumer execution, compile fixtures and authentication/identity verifiers.

Log: `rom-dev:/var/tmp/rom-query-foundation-full-check.log`.
Target: `/var/tmp/rom-release-relations-tests-target`.
Both `CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` were zero.
The lockfile has no changes.

The first full invocation caught invalid equality syntax in the new test for the
non-PartialEq `Access` enum. Pattern matching corrected the test; no production
behavior changed. The coordinator then reran the complete verifier successfully.

Focused coordinator runs also passed the core unit tests, downstream consumer
tests and all-target/all-feature core Clippy. Logs in the container:
`/var/tmp/rom-execution-split-focused.log` and
`/var/tmp/rom-execution-split-imports.log`.
The independent query characterization run passed all nine tests.

## Remaining work

The query design and physical-layout reviews are proposals. No native index,
new policy method, read-operation API or maintained selector is implemented here.
Stage 3.1 remains open until its interface, physical profile and executable plan
are settled. Stages 3.2–3.4 still require implementation, failure tests, measurements
and independent review. The broader operations and release stage remains open.
