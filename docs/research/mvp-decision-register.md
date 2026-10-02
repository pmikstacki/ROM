# ROM decision and completion register

Updated: 2026-10-02. This is the inventory for the owner's active goal: complete
the planned research and prototypes and deliver one verified MVP. The normative
acceptance profile is [integrate-resource-mvp](../../openspec/changes/integrate-resource-mvp/design.md).
No count of separate passing experiments constitutes an integrated MVP.

Status vocabulary: **accepted** means explicitly selected by the owner;
**MVP default** is a reversible engineering choice for the authorized first
implementation; **host policy** is configurable behavior with documented bounds;
**deferred** has a reason and an unsupported-capability boundary. Recommendations
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
| Can chains loop? | D/P, bounded behavior required | Combine semantic no-op, dependency filters, durable step identity and causal/work/retry budgets. Do not rely on dedup alone or forbid every revisit. | [Reaction comparison complete](reaction-chain-results.md); integrated durable worker must test crash/ack boundaries. |
| Which query language? | D, MVP default | Shared typed AST for ordinary/live reads; bounded equality/conjunction, stable identity order and explicit pagination contract. Additional operators are capabilities. | Integrated probe exercises equality only; joins/aggregates deferred, not silently approximated. |
| Relationships and deletion? | D/P, MVP default | Typed references; restrict deletion where integrity is promised. No hidden cascade. Cross-adapter integrity must not be advertised without an enforceable contract. | [Field/query follow-up](core-implementation-readiness.md); conformance and capability rejection pending. |
| Fields, null and omission? | D, accepted direction | Canonical extensible codecs; missing leaves a value alone, null is an explicit nullable value, deletion is explicit. Preserve false/zero/empty. | [Configuration trial](configuration-trial-results.md), authoring fixtures; integrated patch semantics pending. |
| Human usability? | D, accepted | Resource authors write domain behavior. ROM hides generic machinery. Errors identify author source/field and supported alternatives. | Compiler fixtures plus packaged consumer; real human walkthrough remains distinct evidence. |
| Authentication and local users? | D, accepted direction | Verified authority-qualified identity maps explicitly to User Resource. No email auto-merge; default deny. Provider configuration is Resource, verifier is implementation. | [Auth research](auth-architecture.md); JWT/introspection executable probe complete; maintained extraction under review. |
| Authorization engine? | I, MVP default | Native Rust policy seam with action/row/field checks and validity context. No Cedar/Casbin dependency until a concrete advantage is demonstrated. | Engine-independent contract; actor expiry verified; field projection and User mapping pending. |
| First administrator and account recovery? | P, MVP default | Host-provisioned explicit subject binding with consumed bootstrap authority. Do not expose first-caller-wins. Recovery is an audited privileged operation. | [Configuration trial](configuration-trial-results.md); durable bootstrap still to integrate. |
| Tenancy? | D/P, deferred profile | First conformance profile is one isolation scope; reject attempts to claim independent multi-tenant enforcement until every storage/query/effect path is scoped and tested. Tenant, when enabled, remains Resource. | No claim that deployment separation equals row-level multitenancy. |
| Transport foundation? | D, accepted; I, MVP default | Separate invocation, live and journal handles; focused Rust interfaces. HTTP is an extension. Tower is optional composition, never work ownership. | [15-test trial](transport-trial-results.md); real generic wire binding still pending. |
| Configuration sources? | D, accepted; I, MVP default | config-rs narrow input adapter; source authority before precedence, provenance retained by ROM, validated reload through same definitions. Figment remains viable. | [24-test trial](configuration-trial-results.md); actual persistence/activation pending. |
| Secrets? | D, mandatory boundary | Protected fields/references from the start, redacted diagnostics and public descriptors. No raw loader error passthrough. | Both configuration libraries leak inputs in raw errors; secret resolver/provider profile remains explicit. |
| Blob storage? | D, accepted | Generic bounded BlobStore with folder/S3 implementations and orphan/cleanup lifecycle; blob upload is not a distributed DB transaction. | [Real folder/MinIO probe](capability-prototype-results.md); integrate access and lifecycle. |
| Notifications? | D, accepted | Named Rust function channels, durable intentions in resource commit, bounded retry and inspectable failures. Receiver idempotency determines duplicate external effects. | [Outbox probe](capability-prototype-results.md); combine with full commit and worker. |
| Cache and Salsa? | I, measured optional optimization | Supervised single-flight first. Keep completed-result cache private and bounded, enable only with a demonstrated workload benefit. Salsa is optional derived-read research, not mutation memoization. | [22-test/288-sample results](cache-prototype-results.md); no production cache winner assumed. |
| Retention and old retries? | P, MVP default | Separate journal, receipt and resource retention. Initially retain receipts under a budget and reject new obligations at capacity; never treat deleted receipt as proof of no prior commit. | [Transport recommendations](transport-trial-results.md); durable exhaustion/restore tests pending. |
| Multiple instances? | I, deferred | First profile has one write/invalidation owner. Later require durable claims, leases/fencing and journal reconciliation; notifications are wakeups only. | Transport report; reject unsupported multi-owner registration rather than imply consistency. |
| Definition/plugin compatibility? | I, MVP default | Version descriptors and stored encoding; one registration gate, native Rust plugins. Unknown versions/capabilities fail explicitly. WASM later. | Derive and registration probes; persisted-format checks verified on both maintained adapters. |
| Operations and diagnostics? | P/I, MVP default | Bounded-cardinality metrics, protected correlation, readiness/intake/drain states, safe errors. Backup includes receipts/pending work; blob completeness separate. | [Transport operations research](transport-trial-results.md); integrated recovery checks pending. |
| Library release? | I, MVP default | `rom` facade plus derive/adapters crates, pre-1.0 version, explicit tested Rust floor, lockfile and package smoke test. | [Quality gates](../quality.md); no production package published yet. |
| ROM Studio? | D, accepted direction; production deferred | Generic renderer registry plus curated user/provider/settings views, generated from authorized metadata. Svelte preferred; copied controls require maintenance review. | [Frontend](rom-studio-frontend-research.md), [controls](rom-studio-controls-research.md), existing mock. MVP HTTP/descriptor supports later integration; mock is not production Studio. |

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
| Framework/RIM/Beskid research | Published earlier reports | Indexed findings and limitations remain traceable. |
| Execution, authoring, persistence/blob/notification/cache trials | Published executable evidence | Carry applicable cases into integrated tests; no wholesale prototype-copy claim. |
| Configuration and transport trials | Independently verified 24 + 15 tests, published prototype branches | Reports on main; integrated seams still pending. |
| Integrated typed core probe | Independently verified 18 tests and five compiler fixtures | [Report](integrated-core-probe-results.md); promotion findings corrected with regressions. |
| Auth provider profiles | Independently verified 19 runtime tests and one compile-fail | [Report](auth-provider-probe-results.md); provider verification must still connect to the maintained User/configuration lifecycle. |
| Reactive chains | Independently verified 19 tests and 110 deterministic comparison rows | [Report](reaction-chain-results.md); actual durable worker still to integrate. |
| Maintained reusable MVP | Foundation and SQLite/redb conformance verified; auth, reactions and transport integration active | All tasks in integrate-resource-mvp pass, packaged consumer, review, documented boundaries. |

Keep this register current when evidence changes. It is a completion aid, not a
substitute for executable acceptance or an assertion that all broad OpenSpec
requirements are satisfied by a narrow release.
