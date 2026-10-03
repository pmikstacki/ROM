# ROM decision and completion register

Updated: 2026-10-02. This is the inventory for the owner's active goal: complete
the planned research and prototypes and deliver one verified MVP. The normative
acceptance profile is [integrate-resource-mvp](../../openspec/changes/integrate-resource-mvp/design.md).
Separate passing experiments do not establish an integrated MVP, regardless of their count.

Status vocabulary:

- **accepted** means explicitly selected by the owner.
- **MVP default** is a reversible engineering choice for the authorized first implementation.
- **host policy** is configurable behavior with documented bounds.
- **deferred** has a reason and an unsupported-capability boundary.
 Recommendations
are not retroactively described as owner decisions. D = behavior, P = policy,
I = implementation.

| Question | Class/status | Recommendation and reason | Evidence / next proof |
| --- | --- | --- | --- |
| What is the application unit? | D, accepted | One Resource; actions and events are roles around it. User, provider settings and app settings use the same model. | [Framework research](state-of-art-resource-frameworks.md); maintained two-kind consumer verified. |
| Who owns structure and behavior? | I, accepted direction | Derive emits codecs and typed bindings, registration validates/freezes, fluent definitions attach behavior, runtime enforces dynamic guarantees. | [Authoring trials](authoring-crate-trials.md), [Beskid lessons](beskid-compiler-lessons.md). |
| Execution engine? | I, accepted | Tokio async plus Rayon CPU; framework-owned supervision outlives observers. | [Execution results](prototype-results.md), [transport trial](transport-trial-results.md). |
| Database abstraction? | D, accepted; I, MVP default | Core contract describes atomic bundles and conditional revision; SQLite and redb demonstrate different storage models. No SQL in the core. | [Adapter results](capability-prototype-results.md); [maintained SQLite/redb conformance](../storage-adapters.md) verified. |
| State versus event sourcing? | I, MVP default | Current state plus journal/receipts/effects. Mandatory full replay adds complexity without evidence that every app needs it. | [Persistence research](storage-relational-backends.md). Version persisted encodings, reject unknown versions. |
| Multi-Resource consistency? | D, accepted chain default | Commit one Resource, persist reaction, retry downstream step. Earlier commits remain. Explicit compensation is another authorized action. | [Event requirement](../../openspec/changes/establish-rom/specs/event-reactivity/spec.md); [chain experiments complete](reaction-chain-results.md). Cross-resource atomic profile deferred. |
| Can chains loop? | D/P, bounded behavior required | Combine semantic no-op, dependency filters, durable step identity and causal/work/retry budgets. Do not rely on dedup alone or forbid every revisit. | [Reaction comparison complete](reaction-chain-results.md); [maintained durable worker and restart tests](maintained-reaction-results.md) verified. |
| Which query language? | D, MVP default | Shared typed AST for ordinary/live reads; bounded equality/conjunction, stable identity order and explicit pagination contract. Additional operators are capabilities. | [Canonical equality/conjunction and bounded keyset pages](../queries.md) implemented; moving views, not snapshot pagination. Joins/aggregates deferred. |
| Relationships and deletion? | D/P, MVP default | Typed references; restrict deletion where integrity is promised. No hidden cascade. Cross-adapter integrity must not be advertised without an enforceable contract. | [Typed reference/field conformance](../fields.md) verified. References promise identity only; foreign-key enforcement and cascades are not advertised. |
| Fields, null and omission? | D, accepted direction | Canonical extensible codecs; missing leaves a value alone, null is an explicit nullable value, deletion is explicit. Preserve false/zero/empty. | [Presence/PATCH and wire/query tests](../presence-and-patch.md) verified, including absent-field authorization negative control. |
| Human usability? | D, accepted | Resource authors write domain behavior. ROM hides generic machinery. Errors identify author source/field and supported alternatives. | Compiler fixtures plus packaged consumer; real human walkthrough remains distinct evidence. |
| Authentication and local users? | D, accepted direction | Verified authority-qualified identity maps explicitly to User Resource. No email auto-merge; default deny. Provider configuration is Resource, verifier is implementation. | [Maintained auth](maintained-auth-results.md) and [User/provider/link mapping](maintained-identity-projection.md) verified with real synthetic signatures. |
| Authorization engine? | I, MVP default | Native Rust policy seam with action/row/field checks and validity context. No Cedar/Casbin dependency until a concrete advantage is demonstrated. | Native row/field/query policy and current User/provider checks verified. Custom actions inherit Resource write policy and changed-field grants; no declarative policy engine or separate action-role language is claimed. |
| First administrator and account recovery? | P, MVP default | Explicit host provisioning before public intake, without a public bootstrap actor/endpoint. Do not expose first-caller-wins. Automated one-use bootstrap and account-recovery ceremonies remain a separate host product profile. | Native identity resources support explicit provisioning. No automatic consumed-token bootstrap service is implemented or required for the local MVP. |
| Tenancy? | D/P, deferred profile | First conformance profile is one isolation scope; reject attempts to claim independent multi-tenant enforcement until every storage/query/effect path is scoped and tested. Tenant, when enabled, remains Resource. | No claim that deployment separation equals row-level multitenancy. |
| Transport foundation? | D, accepted; I, MVP default | Separate invocation, live and journal handles; focused Rust interfaces. HTTP is an extension. Tower is optional composition, never work ownership. | [Tower/direct trial](transport-trial-results.md), [generic HTTP](maintained-http-journal.md) and structured query/PATCH wire tests verified. |
| Configuration sources? | D, accepted; I, MVP default | config-rs narrow input adapter; source authority before precedence, provenance retained by ROM, validated reload through same definitions. Figment remains viable. | [24-test trial](configuration-trial-results.md); [maintained complete-Resource reload/provenance/restart](maintained-configuration-results.md) verified. |
| Secrets? | D, mandatory boundary | Protected fields/references from the start, redacted diagnostics and public descriptors. No raw loader error passthrough. | Both configuration libraries leak inputs in raw errors; secret resolver/provider profile remains explicit. |
| Blob storage? | D, accepted | Generic bounded BlobStore with folder/S3 implementations and orphan/cleanup lifecycle; blob upload is not a distributed DB transaction. | [Maintained Blob lifecycle](mvp-blob-results.md): actual folder/MinIO, current authorization, immutable reservation binding, shutdown/reopen/detach. |
| Notifications? | D, accepted | Named Rust function channels, durable intentions in resource commit, bounded retry and inspectable failures. Receiver idempotency determines duplicate external effects. | [Maintained typed channels](maintained-channel-results.md) combine obligations with full commit and supervised worker; real restart/lost-ack cases verified. |
| Cache and Salsa? | I, measured optional optimization | Keep durable idempotency authoritative. Single-flight and a private bounded completed cache are measured follow-on optimizations, disabled in this first maintained profile because unique-traffic overhead is not negligible by evidence. Salsa is optional derived-read research, not mutation memoization. | [22-test/288-sample results](cache-prototype-results.md). Maintained concurrent retries may compute a pure proposal twice but cannot commit it twice; no in-memory coalescing/cache performance claim. |
| Retention and old retries? | P, MVP default | Separate journal, receipt and resource retention. Initially retain receipts under a budget. At capacity, reject new obligations. Never treat a deleted receipt as proof of no prior commit. | Bounded journal/receipt/ledger exhaustion and cursor gaps verified; [native backup/restore](maintained-backup-results.md) preserves obligations and fences restored claims/cursors. |
| Multiple instances? | I, deferred | First profile has one write/invalidation owner. Later require durable claims, leases/fencing and journal reconciliation; notifications are wakeups only. | Transport report; single-owner deployment is a host precondition, not an enforced global registry of shared Storage handles. |
| Definition/plugin compatibility? | I, MVP default | Version descriptors and stored encoding; one registration gate, native Rust plugins. Unknown versions/capabilities fail explicitly. WASM later. | Derive and registration probes; persisted-format checks verified on both maintained adapters. |
| Operations and diagnostics? | P/I, MVP default | Payload-free runtime status/capacity counters, readiness/intake/drain states and safe errors; exported telemetry integrations remain optional. Backup includes receipts/pending work; blob completeness separate. | [Transport operations research](transport-trial-results.md); [native backup/recovery](maintained-backup-results.md), bounded runtime status and typed errors. |
| Library release? | I, MVP default | `rom` facade plus derive/adapters crates, pre-1.0 version, explicit tested Rust floor, lockfile and package smoke test. | [Quality gates](../quality.md); no production package published yet. |
| ROM Studio? | D, accepted direction; production deferred | Generic renderer registry plus curated user/provider/settings views, generated from authorized metadata. Svelte preferred; copied controls require maintenance review. | [Frontend](rom-studio-frontend-research.md), [controls](rom-studio-controls-research.md), existing mock. Typed descriptors and generic HTTP are foundations; an authorized public discovery endpoint and production Studio renderer remain future work. The mock is not production Studio. |

## Remaining deployment choices

Counts, byte limits, retry delays/horizons, retention budgets, shutdown grace,
authentication issuer/audience, source authority, notification channels and
external credentials are host policy. The MVP must expose validated defaults and
fail unsupported configurations. No owner input is needed to execute deterministic
local conformance tests with synthetic identities and explicit test limits.

Self-service linking/JIT user provisioning, privileged account recovery policy,
cross-tenant sharing, independent multi-writer topology, arbitrary joins and
zero-downtime live type changes remain optional product profiles. The research
must explain them; this MVP must not claim to implement them.

## Completion ledger

| Work | Current evidence | Completion condition |
| --- | --- | --- |
| Framework/RIM/Beskid research | [RIM source assessment](rim-source-assessment.md) plus published framework/Beskid reports | Indexed findings and limitations remain traceable. |
| Execution, authoring, persistence/blob/notification/cache trials | Published executable evidence | Carry applicable cases into integrated tests; no wholesale prototype-copy claim. |
| Configuration and transport trials | Independently verified 24 + 15 tests, published prototype branches | [Maintained configuration](maintained-configuration-results.md) and [HTTP](maintained-http-journal.md) integrated and verified. |
| Integrated typed core probe | Independently verified 18 tests and five compiler fixtures | [Report](integrated-core-probe-results.md); promotion findings corrected with regressions. |
| Auth provider profiles | Independently verified 19 runtime tests and one compile-fail | [Report](auth-provider-probe-results.md); [maintained User/configuration lifecycle](maintained-identity-projection.md) connected and verified. |
| Reactive chains | Independently verified 19 tests and 110 deterministic comparison rows | [Report](reaction-chain-results.md); [maintained durable worker](maintained-reaction-results.md) integrated and verified. |
| Maintained reusable MVP | Core, SQLite/redb, auth, queries/patch, configuration, HTTP, reactions, channels, Blob, backup and assembled demo verified | [Final acceptance report](mvp-release-results.md) records the combined verification, packaging, review and supported boundaries. |

When evidence changes, update this register. It is a completion aid, not a
substitute for executable acceptance or an assertion that all broad OpenSpec
requirements are satisfied by a narrow release.
