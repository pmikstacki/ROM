# ROM — Resource Oriented Meta Framework

A backend-first Rust library built around one domain entity: the **resource**.

Declare a resource once; the framework supplies standard storage, operations, endpoints and reactive reads. Applications add domain behavior without writing a repository, controller or broadcaster for each resource kind.

Actions request changes. The core validates and commits transitions. Events describe committed facts. Reactions can submit further actions through the same core.

**Status:** a maintained experimental Rust workspace now implements the typed Resource foundation. The complete MVP is still in progress; this is not a production release. The [MVP acceptance plan](openspec/changes/integrate-resource-mvp/tasks.md) and [decision register](docs/research/mvp-decision-register.md) distinguish verified behavior from remaining implementation.

## Design

- One main ROM library with typed resources and extensible field types.
- Tokio for asynchronous execution and I/O, Rayon for CPU-heavy work; the maintained core supervises accepted work independently of caller cancellation.
- Persistence contracts owned by the core, database implementations supplied by adapters.
- Rust extension interfaces first; optional WASM implementations later.
- HTTP and RabbitMQ integrations remain separate extensions.
- Every mutation follows the same validation, concurrency and event guarantees.

Start with the [OpenSpec proposal](openspec/changes/establish-rom/proposal.md), [design](openspec/changes/establish-rom/design.md), and [tasks](openspec/changes/establish-rom/tasks.md). See [planning](docs/planning.md) for the workflow inspired by Beskid.

## License

[MIT](LICENSE). Copyright © 2026 Piotr Mikstacki.

## Design research

Research informs the design; candidate crates are not an approved dependency list.

- [RIM source assessment and ideas carried into ROM](docs/research/rim-source-assessment.md)
- [Framework comparisons and Rust crate candidates](docs/research/frameworks-and-rust-crates.md)
- [Six-framework comparison and ranked recommendations](docs/research/state-of-art-resource-frameworks.md)
- [Prototype findings and implementation tradeoffs](docs/research/prototype-results.md)
- [Integrated typed core: executed evidence and promotion gaps](docs/research/integrated-core-probe-results.md)
- [Maintained bounded execution, expiry and live delivery results](docs/research/maintained-foundation-results.md)
- [SQLite/redb shared atomic bundle and process-exit conformance](docs/storage-adapters.md)
- [Independent promotion review and regression requirements](docs/research/integrated-core-promotion-review.md)
- [Reactive chains: 19 tests, 110 comparisons and loop-control recommendations](docs/research/reaction-chain-results.md)
- [Configuration loaders: 24 executed tests and critique](docs/research/configuration-trial-results.md)
- [Native User/provider mapping and field projection](docs/research/maintained-identity-projection.md)
- [Maintained HTTP and durable journal](docs/research/maintained-http-journal.md)
- [Maintained typed reactive chains](docs/research/maintained-reaction-results.md)
- [Durable notification channels](docs/research/maintained-channel-results.md)
- [Presence, null and typed partial changes](docs/presence-and-patch.md)
- [Maintained authentication: reviewed profiles and integration boundary](docs/research/maintained-auth-results.md)
- [Immutable experiment reproduction](docs/research/reproducing-experiments.md)
- [JWT and introspection profiles: executed verification and remaining integration](docs/research/auth-provider-probe-results.md)
- [Tower/direct transport contracts: 15 executed tests and critique](docs/research/transport-trial-results.md)
- [All MVP decisions, recommendations and completion inventory](docs/research/mvp-decision-register.md)
- [Interchangeable adapters and resilience: executed findings](docs/research/capability-prototype-results.md)
- [Internal cache, single-flight and Salsa: executed findings](docs/research/cache-prototype-results.md)
- [Cache crate contracts and selection criteria](docs/research/internal-cache-research.md)
- [Architecture cohesion and remaining integration gaps](docs/research/architecture-cohesion-review.md)
- [Technology fit and authoring friction review](docs/research/technology-fit-review.md)
- [Core-owned transport layer and extension contracts](docs/research/transport-layer-review.md)
- [ROM Studio: curated screens and generic resource views](docs/research/rom-studio-product-research.md)
- [ROM Studio: Svelte rendering and dependency compatibility](docs/research/rom-studio-frontend-research.md)
- [ROM Studio: component maintenance and selection](docs/research/rom-studio-controls-research.md)
- [ROM Studio: shared discovery and client contract](docs/research/rom-studio-client-contract.md)
- [Configuration Resources, source ownership and activation](docs/research/configuration-resource-contract.md)
- [Hierarchical configuration provider candidates](docs/research/configuration-provider-research.md)
- [Beskid's implemented configuration hierarchy and lessons](docs/research/configuration-beskid-lessons.md)
- [Generic persistence, blob and notification contracts](openspec/changes/design-capability-adapters/design.md)
- [Human-friendly Rust authoring: derives and fluent APIs](docs/research/rust-resource-authoring.md)
- [Executed derive and builder crate trials](docs/research/authoring-crate-trials.md)
- [Reusable core work packages and estimation readiness](docs/research/core-implementation-readiness.md)
- [Beskid compiler and Rust quality lessons](docs/research/beskid-compiler-lessons.md)
- [Concurrency experiments and guarantees](docs/research/rust-concurrency.md)
- [Provider-neutral authentication and authorization](docs/research/auth-architecture.md)
- [Auth specification proposal](openspec/changes/design-provider-neutral-auth/proposal.md)

[Quality gates](docs/quality.md) turn the [Beskid baseline audit](docs/research/beskid-quality-baseline.md) into concrete delivery criteria.

## Development

Use `./scripts/check` for local verification and `./scripts/build` to build. The initial tested Rust floor is 1.99.0. See the [persistent NixOS environment](infra/nixos/README.md). GitHub Actions is disabled. The [consumer](examples/consumer/src/main.rs) exercises two Resources through the public library; the [demo](demo/README.md) remains reserved for the completed first core milestone.

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && ./scripts/check'
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && cargo run -p rom-consumer --locked'
```

The maintained foundation now has bounded asynchronous reads, actions and live
queries, cancellation-safe shutdown, actor expiry, and SQLite/redb conformance.
Field projection, verified-identity mapping to User Resources, durable reaction
workers, notification channels and generic HTTP are integrated. Partial changes
preserve missing/null/removal, and queries share typed and wire semantics.
Configuration ingestion, Blob lifecycle, backup/recovery and the final assembled
demo are the remaining integration packages. The dependency and packaging checks
are rerun as those packages enter the workspace.
The [maintained verification record](docs/research/maintained-foundation-results.md)
and [storage contract](docs/storage-adapters.md) supersede the corresponding
limitations of the original disposable probe.

Package verification builds every extracted library archive and runs a consumer
outside this workspace: `node scripts/check-packages.mjs` inside `rom-dev`.
This does not publish packages to crates.io.

[Research and prototype program](docs/research/prototype-program.md) records the fixed premise and the experiments used to validate implementation choices.
