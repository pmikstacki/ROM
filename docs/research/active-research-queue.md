# ROM research queue — 2026-10-02

The owner requested parallel research independent of Studio/VPN troubleshooting. This queue records assignments, not completed results.

The accepted premise has one Resource domain model, typed derive/fluent authoring, core-owned semantics and interchangeable adapters. Research can refine mechanisms. It cannot replace that premise or silently select production dependencies.

| Workstream | State at dispatch | Deliverable |
| --- | --- | --- |
| Configuration trials | Complete, independently verified | [24 tests and recommendations](configuration-trial-results.md); production source reconciliation remains integration work. |
| Transport trials | Complete, independently verified | [15 tests and recommendations](transport-trial-results.md); actual transport bindings remain integration work. |
| Integrated core probe | Complete, independently verified; promotion findings corrected | [18 tests plus compiler fixtures](integrated-core-probe-results.md); two typed Resources share the pipeline. Production gaps remain explicit. |
| Reaction-chain experiments | Complete, independently verified | [19 tests, 110 model result rows](reaction-chain-results.md); combined guards recommended. Actual durable worker remains integration work. |
| Authentication provider probe | Complete, independently verified | [19 runtime tests plus one compile-fail](auth-provider-probe-results.md); synthetic signed JWT and real loopback introspection, not deployed IdP interoperability. |
| Maintained core foundation | Complete, integrated and independently verified | Bounded async reads/live, cancellation-safe drain, actor expiry and corrected promotion regressions; [verification](maintained-foundation-results.md). |
| Dependency baseline audit | Complete | [Executed audit](mvp-dependency-audit.md): zero RustSec findings for initial pins; final maintained lockfile delta and native SQLite patch decision remain gates. |
| Maintained persistence conformance | Complete, integrated and independently verified | [SQLite/redb conformance](../storage-adapters.md), real process exit and foreign-format rejection. |
| Maintained authentication | Integrated and verified | JWT/introspection plus native User/provider/link Resources, current revocation and protected field projection; [results](maintained-identity-projection.md). |
| Durable reaction integration | Integrated and verified | [Typed durable chains](maintained-reaction-results.md) run through current authorization and both storage adapters. Delivery channels and final drain correction are being integrated separately. |
| Generic transport integration | Integrated and verified | [HTTP](maintained-http-journal.md), structured queries and partial changes, 16 loopback tests; keepalive starvation corrected. |
| Field/presence/query integration | Integrated and verified | [Presence/PATCH](../presence-and-patch.md), canonical predicates, bounded conjunction/keyset pagination, typed and wire APIs. |
| Configuration ingestion | Running: configuration_trials | Native SourceActivation, authority/provenance, failed reload and restart reconciliation. |
| Notification channels | Running: integrated_core_probe | Atomic obligations, ordinary Rust async functions, durable retries and uncertain delivery evidence. |
| Blob lifecycle | Running: mvp_promotion_review | Maintained Blob Resource plus bounded folder/S3-compatible adapter, attachment/access and orphan semantics. |
| Backup, final demo and release gates | Coordinator, remaining | Preserve complete durable state, run packaged consumer and complete dependency review, then independent combined review. |
| Integrated MVP | Coordinator, active goal | Decision register, explicit acceptance profile, combined conformance and packaged consumer. Separate experiments alone cannot complete this goal. |

Each agent uses an isolated worktree and prototype branch. Reports separate primary-source claims, executed evidence and proposals. A completed worker task does not establish completion of the integrated core. Coordinator review and independent verification remain necessary. These are delegated agent tasks; no visible standalone project conversations were created.

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
