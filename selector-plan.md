# Strategy selector experiment implementation plan

Goal: execute the same normalized query through an explicit bounded core path or
an adapter-controlled native path; the database retains physical plan ownership.
This is a standalone prototype, not a maintained query API or certified optimizer.

Use the frozen query-planning prototype as the semantic/execution fixture. Add
`selector.rs` with a pure cost comparison and a narrow planner adapter boundary.
Model setup, scan, decoding and sort work separately. Native estimates are hints,
never authorization or correctness evidence. Missing/stale/invalid estimates
fall back to the core path. The adapter keeps exact SQL and its SQLite planner;
the selector must not issue INDEXED BY or force a vendor physical plan.

Tests precede implementation: equal results under both paths; missing/stale cost
fallback; overflowing costs safely excluded; mismatched query estimates excluded;
no unsupported semantic candidate; mutated data invalidates estimates; indexed
and unindexed databases both work; deliberately wrong statistics can hurt ranking
but cannot change results. Re-run the negative limit-before-auth control.

Record repeated release timings for selected, core and native paths on selective,
broad and empty cases. Include planning overhead. Report how often the estimate
chooses the fastest measured path and regret versus the best passing alternative.
No universal-optimality claim; SQLite EXPLAIN text is version-specific diagnostic
input in this fixture, with unknown forms falling back conservatively. Statistics
are collected at dataset creation, not by scanning before every query.

Files: `prototypes/query-planning/src/selector.rs`, tests/selector.rs,
src/bin/selector.rs, and a report on main linking the immutable branch.
No new dependencies; use existing locked Rust/SQLite stack. Native host execution
uses a task-owned persistent target with two build jobs and debug information off.
