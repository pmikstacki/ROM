# ROM research queue — 2026-10-02

The owner requested parallel research to continue independently of Studio/VPN
troubleshooting. This queue records assignments, not completed results. Accepted
premise: one Resource domain model, typed derive/fluent authoring, core-owned
semantics and interchangeable adapters. Research may refine mechanisms, not
replace that premise or silently select production dependencies.

| Workstream | State at dispatch | Deliverable |
| --- | --- | --- |
| Configuration trials | Complete, independently verified | [24 tests and recommendations](configuration-trial-results.md); production source reconciliation remains integration work. |
| Transport trials | Complete, independently verified | [15 tests and recommendations](transport-trial-results.md); actual transport bindings remain integration work. |
| Integrated core probe | Complete, independently verified; promotion review running | [18 tests plus compiler fixtures](integrated-core-probe-results.md); two typed Resources share the pipeline. Production gaps remain explicit. |
| Reaction-chain experiments | Complete, independently verified | [19 tests, 110 model result rows](reaction-chain-results.md); combined guards recommended. Actual durable worker remains integration work. |
| Authentication provider probe | Running: configuration_trials | Cryptographic JWT and loopback introspection profiles; verified identities, revocation, freshness and failure tests. |
| Maintained core foundation | Running: integrated_core_probe | Root Cargo workspace; bounded async reads/live, cancellation-safe drain, actor expiry and promotion defect regressions. |
| Dependency audit | Running: mvp_promotion_review | Pinned dependency licenses/features/MSRV and current advisory checks; distinguish actual tool output from source review. |
| Integrated MVP | Coordinator, active goal | Decision register, explicit acceptance profile, combined conformance and packaged consumer. Separate experiments alone cannot complete this goal. |

Each agent uses an isolated worktree and prototype branch. Reports separate
primary-source claims, executed evidence and proposals. A worker finishing is
not an integrated-core completion claim: coordinator review and independent
verification remain necessary. No visible standalone project conversations were
created; these are delegated agent tasks.

## Discussion inventory

These topics were unclosed or only briefly discussed, not necessarily absent from
all previous reports:

- Resource relationships, referential deletion and multi-resource actions.
- Query language: ordering, pagination, joins/aggregates and live-query scope.
- Field families, canonical codecs, presence/null/delete and custom field support.
- Tenant boundaries, initial administrator, account linking and revocation.
- Reaction cycles, delayed work, retry budgets and failed-work intervention.
- Event/receipt retention, expired cursors and protected historical values.
- Multiple application instances, worker ownership and recovery.
- Compatibility of definitions, codecs and extensions with stored data.
- Tracing, health, backup/restore and shutdown behavior.
- Rust MSRV, feature flags, API compatibility and application-author documentation.

This is not an expanded first-release commitment. Each report must recommend
what belongs in the first milestone and what should be explicitly deferred.
