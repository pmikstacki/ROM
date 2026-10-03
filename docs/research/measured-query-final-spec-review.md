# Measured query release specification review

Date: 2026-10-03. Evidence category: independent source inspection only.

Reviewed the uncommitted query changes against `9db4b2ab4bb090a940f452eb1039c77c9d1c8aab`, including untracked planner, observation, materialization, test and measurement modules.
Authority: the current framework-readiness OpenSpec, adapter query contract, probe selection plan and repository quality rules.
No builds, tests or benchmarks were executed during this review.

No correctness, requirement or scope blocker was found. No required code fixes were identified.

- `crates/rom-sqlite/src/index/query.rs:38` admits the whole kind before probes. Descriptor validation, persisted bytes and canonical bytes remain enforced.
- `crates/rom-sqlite/src/index/planner.rs:48` separates exact counts from saturation. Four distinct predicates and two capped passes bound returned probe keys to 16,456.
- `crates/rom-sqlite/src/index/planner/predicate.rs:93` shares encoded conditions between probes and complete materialization. Only probe SQL has a candidate-count limit.
- `crates/rom-sqlite/src/index/planner/cost.rs:14` uses checked cost construction and rounded candidate-byte estimates. Saturation receives whole-kind cost; completed probe costs affect both alternatives.
- `crates/rom-sqlite/src/index/query.rs:99` restricts forced execution to test support. The shared eligibility and binding selector still runs before native execution. Materialization errors propagate.
- `crates/rom-sqlite/src/read_rows.rs:44` centralizes reference materialization. Ordinary reads do not collect SQLite status counters or probe clocks.
- `tools/query-measure/src/run.rs:76` establishes reference results outside measurement intervals. Each trial compares complete ordered views after timing and allocation capture.

The changes preserve public facade exports and keep SQLite planning outside core. Measurement writes use public Runtime Resource commands and retain normal receipts and events.

Boundary and parity tests were inspected, not executed. This review does not establish test success, measured speedups or release readiness. Those require the coordinator's final verifier and frozen-source performance evidence.
