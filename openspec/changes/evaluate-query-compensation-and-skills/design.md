# Research and implementation boundaries

## Owner decisions
2026-10-03: query results may change between pages. Filters and sorting should be pleasant for application authors and translate through generic adapters. Compare generated, hybrid and runtime approaches before selecting implementation. Aggregations/reporting are deferred. Relationship deletion defaults to restrict when integrity is implemented. Compensation remains explicitly declared, never implicit snapshot rollback. Skills and WASM are additional research tracks; Studio remains excluded.

## Query question
The mandatory Resource/action/commit/event semantics do not vary by adapter. Backend capabilities describe execution optimizations, not alternate meanings of a domain operation. A conforming adapter must satisfy the required persistence guarantees; registration rejects an incompatible mandatory profile. Queries retain the common evaluator as fallback where exact pushdown is unavailable, subject to declared common work bounds. Application authors do not write a second query for each backend.

Separate authoring from execution. Compare a generated typed surface, a hybrid typed surface lowering to a shared runtime representation, and a dynamic descriptor-validated representation. Equivalent queries must have identical results and authorization behavior. Measure construction/normalization, compile/build cost and actual execution separately; code generation alone does not establish faster database execution.

The descriptor and real field codecs remain authoritative. Values use parameters rather than SQL interpolation; only accepted field identifiers may be translated. Missing, null, false, zero, exact integers, Unicode and custom codecs require explicit semantics. Sort order must be deterministic with a Resource ID tie-breaker. Moving-view continuations need declared behavior when data/authority changes; no retained snapshot guarantee. Sorting needs authorization before scanning because order itself can disclose data.

Core owns semantics; an adapter owns vendor translation and advertises only validated capabilities. Arbitrary Rust row policies cannot be presumed SQL-translatable. Limit-before-policy is an explicit negative control; any refill remains bounded and must fail rather than return falsely complete results when its budget expires. Keep the current evaluator as a correctness oracle. Do not add joins, aggregates, implicit lossy numeric coercion or a concrete database dependency to core.

## Compensation probe
Use ordinary domain Resources recording confirmed outcomes and explicit actions which target a reservation/account identity. Compare no compensation, unsafe snapshot replacement and targeted compensation under concurrent unrelated changes. Existing reactions may route a confirmed business failure to compensation; this is not a generic callback for technical terminal failure of arbitrary work.

Cases: stock reservation plus confirmed payment rejection; seat reservation plus downstream booking failure; account provisioning plus entitlement failure. Provider outcomes are synthetic. Test transient retry, unknown outcome/reconciliation hold, duplicate delivery, reopened stores, a real process-exit failpoint where feasible, compensation failure/retry/manual state, revoked authority and bounded cycles. Preserve upstream commits and audit history. Count durable revisions/events/effects rather than only returned errors. No external exactly-once or financial-provider certification.

## Skills and WASM
Skills must teach the actual versioned public API, reference authoritative examples and run small verifiers. Research triggers, progressive disclosure, templates, failure cases and evaluation methods for creating Resources/actions/plugins/adapters, testing and diagnosing them. Do not write a parallel semantic specification in prompts or imply experimental features are available.

WASM research compares native extension boundaries with bounded pure guest validation/proposal execution. Define versioned inputs/results, capability imports, memory/fuel/time/output budgets and cancellation. Database writes, external credentials and authoritative mutations remain host/core responsibilities. Current fn-pointer/static codec seams must be inspected before suggesting a stateful sandbox integration. A standalone engine probe is not an integrated ROM plugin feature.

## Process and evidence
Use isolated worktrees for independent tracks. Preserve disposable code on named prototype branches and link exact commits from main reports. Record versions, commands, counts, negative controls and measurement limits. User authorization already covers experiments and query implementation; implementation details may be refined from evidence without repeated permission requests. Public maintenance changes require focused regressions, independent review and affected local checks. Reports distinguish recommendations from owner decisions and future work from executed capability.
