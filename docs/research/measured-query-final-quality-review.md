# Measured query quality review

Date: 2026-10-03. Baseline: `9db4b2ab4bb090a940f452eb1039c77c9d1c8aab`; reviewed the uncommitted measurement and selector changes.

## Resolved finding

**P2 — Duplicate dataset sizes could reuse a measured database.**
The runner derives each database path from distribution and size alone.
Repeated sizes could reopen modified data and replay write receipts, invalidating fresh-workload measurements.
[Settings validation](../../tools/query-measure/src/settings.rs:64) now rejects duplicate sizes before the runner creates its output directory.
[The regression](../../tools/query-measure/src/tests.rs:161) checks duplicate rejection and acceptance of distinct sizes.
This review confirmed both changes in source; it did not execute the test.
The coordinator reports the regression failing before the fix, then passing with six default-feature tests, five all-feature tests, and Clippy.
The preserved measurements use distinct sizes and are unaffected.

## Confirmed boundaries and evidence

No unresolved concrete standards or measurement blocker was found in the reviewed scope.
Library roots remain facades. Named modules separate predicate encoding, bounded probes, costs, materialization, and observations.
Probe and materialization SQL share encoded predicates. Observed and ordinary reads share implementation.
The harness compares complete ordered views outside its timers and shares the write workload with the historical comparison.

This review independently executed archive checksum verification and recalculated all three query/heap summaries from their raw records.
Both source archives verified. All summaries matched exactly.
Raw records contain 1,728 observed and 576 production samples per query run, plus 48 separate heap scopes.
The allocation table now explicitly identifies its skewed distribution and seed.

The held-out run changes the fixture seed to 203; sizes, distributions and query shapes remain unchanged.
Fixed production-control ordering and shared-host timing limit causal performance conclusions.
No build, test suite or benchmark was executed by this reviewer.
The coordinator reports passing full verification and demo checks before this tool-only guard; final full verification remains planned.

## Final report follow-up

The final held-out and heap comparison tables match their summaries within displayed rounding.
Independent recalculation reproduced the baseline validation and tuned heap summaries exactly.
All 1,728 paired validation samples have matching result counts and digests.
The worst automatic-to-alternative ratio is 1.201778, consistent with the reported 1.202.
The acceptance conclusion preserves fixture, allocator, shared-host and release-readiness limits; no new finding remains.
The coordinator now reports that final full verification passed after the guard.
No additional test or benchmark was executed by this reviewer.
