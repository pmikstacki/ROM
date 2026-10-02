# Research, experiment and maintained implementation index

This index connects the owner's questions to evidence. A source review, a disposable prototype and a maintained acceptance test answer different questions. The [decision register](mvp-decision-register.md) records accepted choices, reversible MVP defaults, host policy and deferred profiles. The [MVP checklist](../../openspec/changes/integrate-resource-mvp/tasks.md) is the completion gate.

## Current follow-on scope

The owner narrowed the follow-on goal to full deferred-topic research, core API ergonomics/resilience, and a generic CLI. Studio implementation is excluded. The [37-topic roadmap](deferred-capabilities-roadmap.md) records every deferred area with primary sources, alternatives, dependencies, proposed experiments and decision ownership; completing research does not implement those future capabilities. The [CLI choices](cli-implementation-research.md), [discovery contract](../discovery.md) and [current checklist](../../openspec/changes/harden-core-and-add-cli/tasks.md) track this work separately from the completed MVP below.

## Questions and recommendations

| Area | Research / critique | Executed experiment or maintained evidence |
|---|---|---|
| Original Resource premise and RIM | [Static RIM assessment](rim-source-assessment.md), [framework candidates](frameworks-and-rust-crates.md), [six frameworks](state-of-art-resource-frameworks.md) | [Resource flow and execution comparison](prototype-results.md) |
| Compiler, Rust quality and author ergonomics | [Beskid baseline](beskid-quality-baseline.md), [compiler lessons](beskid-compiler-lessons.md), [Rust authoring](rust-resource-authoring.md) | [Derive/builder trials](authoring-crate-trials.md), [fields](../fields.md), [presence/PATCH](../presence-and-patch.md), compile-failure fixtures |
| Runtime and parallelism | [Actors](runtime-comparison-actors.md), [Tokio comparison](runtime-comparison-bevy-tokio.md), [concurrency](rust-concurrency.md) | [Execution experiments](prototype-results.md), [maintained bounded execution](maintained-foundation-results.md), [shutdown correction and status](maintained-channel-results.md) |
| Relational and nonrelational persistence | [Relational families](storage-relational-backends.md), [nonrelational families](storage-nonrelational-backends.md) | [Adapter prototypes](capability-prototype-results.md), [maintained SQLite/redb conformance](../storage-adapters.md) |
| Reactive chains, convergence and round trips | [Chain report with technique comparison](reaction-chain-results.md) | 19 deterministic simulation tests and 110 result rows; [maintained actual durable execution](maintained-reaction-results.md) |
| Queries and live delivery | [Library slice assessment](prototype-results.md), [transport lifecycle](transport-layer-review.md) | [Shared canonical query semantics](../queries.md), [integrated probe](integrated-core-probe-results.md), [field projection](maintained-identity-projection.md) |
| Transport capabilities, Tower and direct services | [Transport review](transport-layer-review.md), [HTTP boundaries](http-integration-boundaries.md) | [Tower/direct trials](transport-trial-results.md), [real HTTP/journal binding](maintained-http-journal.md), [wire presence/query vectors](../presence-and-patch.md) |
| Auth, User/provider Resources and current policy | [Auth architecture](auth-architecture.md) | [Provider trials](auth-provider-probe-results.md), [maintained JWT/introspection](maintained-auth-results.md), [native identity mapping](maintained-identity-projection.md) |
| Configuration hierarchy, source ownership and activation | [Provider comparison](configuration-provider-research.md), [Beskid hierarchy](configuration-beskid-lessons.md), [Resource contract](configuration-resource-contract.md) | [config-rs/Figment trials](configuration-trial-results.md), [maintained ingestion/recovery](maintained-configuration-results.md) |
| Blob and notification capabilities / resilience | [Contract research](storage-and-notification-adapters.md) | [Filesystem/MinIO/outbox experiments](capability-prototype-results.md), [maintained channels](maintained-channel-results.md); [maintained Blob lifecycle and real MinIO](mvp-blob-results.md) |
| Pre-write deduplication and temporary caches | [Moka/quick_cache/Salsa critique](internal-cache-research.md) | [22 tests and 288 benchmark samples](cache-prototype-results.md); no unsupported universal cache winner or zero-overhead claim |
| Studio, generic rendering and controls | [Product](rom-studio-product-research.md), [Svelte](rom-studio-frontend-research.md), [controls maintenance](rom-studio-controls-research.md), [client contract](rom-studio-client-contract.md) | Preserved Studio mock; production Studio remains a separate milestone, not a completed frontend |
| Cohesion and integration quality | [Architecture review](architecture-cohesion-review.md), [technology fit](technology-fit-review.md), [readiness](core-implementation-readiness.md) | [Promotion defects and regressions](integrated-core-promotion-review.md), root workspace verifier and packaged-consumer checks |
| Release, maintenance and dependency risk | [Quality gates](../quality.md), [advisory/native baseline](mvp-dependency-audit.md) | [Native recovery](maintained-backup-results.md), [assembled author demo](maintained-demo-results.md); [dependency inventory](maintained-dependencies.md), [independent review](maintained-mvp-independent-review.md) and [final release evidence](mvp-release-results.md) complete the scoped gates |

## Immutable prototype sources

These commits preserve the actual experiments. Historical reports may refer to agent worktrees that now contain maintained code. Use the commit, not the old mutable checkout path. [Reproduction commands](reproducing-experiments.md) cover the newer trial group.

| Experiment | Immutable source | Report |
|---|---|---|
| Resource flow | `bdf69df2974abb6e08d99bb254ac9ddab025ebea` | [Results](prototype-results.md) |
| Execution contract | `fe0ae3382fdad89c2fe95b3ec85c5db54d619b94` | [Results](prototype-results.md) |
| Derive contract | `f969812c585c8454a92d7f901fa33f351516d192` | [Authoring](authoring-crate-trials.md) |
| Builder authoring | `e5705e1a4e2f2384076382e0f76856545a49ada5` | [Authoring](authoring-crate-trials.md) |
| Library slice | `9c609314a90033cd2f7ffe97cf17d29056cc54b1` | [Results](prototype-results.md) |
| Persistence adapters | `566c57458f6b4d7be725124a3ca2401731fae990` | [Capabilities](capability-prototype-results.md) |
| Blob adapters | `2a345265bf2a45edab9f0f1f0c1a72cd78a8fe53` | [Capabilities](capability-prototype-results.md) |
| Notification channels | `1dad6fb82e3ac67f7e697282735e941e1bfae81a` | [Capabilities](capability-prototype-results.md) |
| Dedup cache | `6153d99384148a7773a35dd0870435c553e7d8be` | [Cache results](cache-prototype-results.md) |
| Salsa derived reads | `5af9ca66f4cb76165e4b5895c8e93ab83de86f09` | [Cache results](cache-prototype-results.md) |
| Integrated core | `86780041a9824c434054fae25509dff0e56a7212` | [Integrated probe](integrated-core-probe-results.md) |
| Studio mock | `9be7c653f23e2a0e304be5d5f7c0f07cc2d194ef` | [Studio research](rom-studio-product-research.md) |
| Configuration trials | `66854f5708` | [Configuration](configuration-trial-results.md) |
| Transport trials | `789201201c6388a44d9d33fe2045d0dd075f33cc` | [Transport](transport-trial-results.md) |
| Provider trials | `58d4c9deb` | [Auth](auth-provider-probe-results.md) |
| Reactive chain simulations | `c95a2a2d` | [Chains](reaction-chain-results.md) |

Old engine research remains historical evidence of the comparison, not an active specification or dependency. The accepted engine is Tokio plus Rayon. RIM itself was inspected statically rather than executed. A client author study, production workload benchmark, arbitrary database certification and multi-host deployment were not performed.
