# SQLite query instrumentation review

Date: 2026-10-03.

This independent source review found no blocker in the instrumentation change.
It does not establish release completion or validate benchmark conclusions.

## Scope and evidence

The reviewed worktree is `release-query-index`, based on `9db4b2ab4bb090a940f452eb1039c77c9d1c8aab`.
The review covers the uncommitted instrumentation changes, including the two new implementation files and dedicated tests.
The reviewer did not author these adapter changes.

The comparison used the baseline query and snapshot implementations, current source, and shared core selection and response validation.
No builds, tests, or benchmarks ran during this review because the coordinator was measuring performance.
The coordinator separately reported two passing public observation tests. That result was not reproduced in this review.

## Production behavior and admission

Both entry points acquire the same connection mutex and start a SQLite read transaction.
The transaction contains metadata reads, plan recognition, and row materialization.
See [the observed entry point](../../crates/rom-sqlite/src/query_observation.rs), lines 34–48,
and [the production entry point](../../crates/rom-sqlite/src/persistence.rs), `Storage::query_read`.

[The shared executor](../../crates/rom-sqlite/src/index/query.rs) uses `execute::<false>` for ordinary queries and `execute::<true>` for observations.
Automatic observation retains the baseline descriptor comparison, counter validation, whole-kind admission, profile read, plan selection, and candidate validation order.
There is no second read after a selected native operation fails.

Whole-kind row, stored-byte, and canonical-byte limits are checked before either forced mode takes effect.
A selective predicate or page limit cannot admit an otherwise oversized kind.
Candidate materialization retains its stored-byte, canonical-byte, key, kind, and live-row checks.
Its SQL still has no logical page limit.

[The extracted snapshot reader](../../crates/rom-sqlite/src/read_rows.rs), lines 37–69, preserves the previous snapshot loop.
The SQL order and defensive `max_rows + 1` limit are unchanged.
It still rejects an excess row before decoding it, and rejects excess stored bytes before JSON decoding.
The added operations occur after the cursor has been drained: release the cursor and collect optional metrics.

## Forced modes and profile binding

Forced reference mode returns after common descriptor, admission, and profile checks. It skips optional native planning.
Forced native mode still requires a supported predicate and a recognized SQLite physical plan.
If no permitted estimate exists, it returns the reference strategy.

Native forcing changes only the two cost values in an existing estimate.
It retains the estimate's complete-candidate flag and exact request/snapshot binding.
[The core selector](../../crates/rom/src/query_storage/selection.rs) still checks selection mode, semantics version, scalar fields, profile versions, and binding equality.
Thus `ReferenceOnly` requests cannot gain native execution through this control.

Returned native candidates retain the same admission and binding metadata.
[Core response validation](../../crates/rom/src/query_storage/validation.rs) remains applicable before filtering and final disclosure.
The adapter adds no application policy callbacks and does not perform Runtime authorization itself.

## Counters, errors, and feature boundaries

`decoded_rows` counts materialized rows in a successful read.
`decoded_bytes` counts the full stored Row JSON text passed to the decoder, including noncanonical whitespace.
It does not measure canonical JSON output or heap allocation.

The VM counter is read from the drained materialization statement.
It excludes descriptor reads, counters, profile reads, plan recognition, and Runtime work.
An empty result can still have nonzero VM steps.
Each read prepares a new statement, so counters are not accumulated across successful calls.

The public observation API, metrics export, and forced-mode variants require `test-support`.
Without that feature, metrics contain no fields and the SQLite status call is absent.
With that feature enabled, ordinary `Storage::query_read` still uses the constant `OBSERVED = false` path and skips the status call.
Feature unification therefore does not turn ordinary production reads into observed reads.

There is one instrumentation-specific error boundary.
[Metric collection](../../crates/rom-sqlite/src/read_rows.rs), lines 27–29, converts the signed status result to `u64` and rejects a negative value with `TooLarge`.
An observed call can therefore fail at metric collection after successful materialization, while an ordinary call skips that conversion.
The `u64` field does not establish a 64-bit range for SQLite's underlying counter.
This is a documented measurement limit, not a blocker for the bounded workloads reviewed here.
Do not extrapolate counter accuracy to arbitrarily large statements without separate validation.

Other decoding, SQLite, metadata, and native integrity errors retain their existing propagation paths.
Observation does not return partial counters on an error or retry through another strategy.

## Test coverage and remaining checks

[Public observation tests](../../crates/rom-sqlite/tests/query_observation.rs) cover automatic/ordinary result parity, forced costs, reference-only selection, absent plans, and whole-kind row admission.
They also cover descriptor mismatch, matching candidate rows, empty candidates, nonzero VM work, and decoded row/byte counts.

[The private regression](../../crates/rom-sqlite/src/index/query_tests.rs), `observed_controls_keep_plan_fallback_execution_errors_and_stored_byte_scope`, covers noncanonical stored bytes and repeat-call counter scope.
It also covers fallback when the physical index is unavailable and propagation of malformed selected Row data in all modes.

Two useful future additions would make forced-mode coverage more explicit:

- Exercise a whole-kind byte limit with a selective predicate and `limit(1)` in all three modes.
- Exercise an unsupported request semantics version under native forcing and assert reference fallback.

Both behaviors follow the shared source path in this review. These suggestions are coverage improvements, not observed failures.
The coordinator still needs the complete verification gates on the integrated source.

## Module and DRY assessment

`lib.rs` and `index.rs` remain facades.
Observation API and metric assembly belong in `query_observation.rs`; bounded snapshot materialization belongs in `read_rows.rs`.
The shared executor avoids duplicating selection or candidate validation for the benchmark.
Snapshot and candidate loops retain separate contracts: one reads the bounded whole kind, while the other validates native candidate membership and admission evidence.
Combining those loops solely to remove similar iteration syntax would obscure their different guarantees.
