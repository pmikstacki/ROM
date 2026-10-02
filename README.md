# ROM — Resource Oriented Meta Framework

A backend-first Rust library built around one domain entity: the **resource**.

Declare a resource once; the framework supplies standard storage, operations, endpoints and reactive reads. Applications add domain behavior without writing a repository, controller or broadcaster for each resource kind.

Actions request changes. The core validates and commits transitions. Events describe committed facts. Reactions can submit further actions through the same core.

**Status:** architecture and specification stage, with executable experiments on separate prototype branches. No production ROM core is released.

## Design

- One main ROM library with typed resources and extensible field types.
- Tokio for asynchronous execution and I/O, Rayon for CPU-heavy work; integration is still being designed.
- Persistence contracts owned by the core, database implementations supplied by adapters.
- Rust extension interfaces first; optional WASM implementations later.
- HTTP and RabbitMQ integrations remain separate extensions.
- Every mutation follows the same validation, concurrency and event guarantees.

Start with the [OpenSpec proposal](openspec/changes/establish-rom/proposal.md), [design](openspec/changes/establish-rom/design.md), and [tasks](openspec/changes/establish-rom/tasks.md). See [planning](docs/planning.md) for the workflow inspired by Beskid.

## License

[MIT](LICENSE). Copyright © 2026 Piotr Mikstacki.

## Design research

Research informs the design; candidate crates are not an approved dependency list.

- [Framework comparisons and Rust crate candidates](docs/research/frameworks-and-rust-crates.md)
- [Six-framework comparison and ranked recommendations](docs/research/state-of-art-resource-frameworks.md)
- [Prototype findings and implementation tradeoffs](docs/research/prototype-results.md)
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

Use `./scripts/check` for local verification and `./scripts/build` once the Rust workspace exists. See the [persistent NixOS environment](infra/nixos/README.md). GitHub Actions is disabled. The [demo](demo/README.md) is reserved for an application built after the first core milestone.

[Research and prototype program](docs/research/prototype-program.md) records the fixed premise and the experiments used to validate implementation choices.
