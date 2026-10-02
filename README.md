# ROM — Resource Oriented Meta Framework

A backend-first Rust library built around one domain entity: the **resource**.

Actions request changes. The core validates and commits transitions. Events describe committed facts. Reactions can submit further actions through the same core.

**Status:** architecture and specification stage; no runtime implementation yet.

## Design

- One main ROM library with typed resources and extensible field types.
- Persistence contracts owned by the core, database implementations supplied by adapters.
- Rust extension interfaces first; optional WASM implementations later.
- HTTP and RabbitMQ integrations remain separate extensions.
- Every mutation follows the same validation, concurrency and event guarantees.

Start with the [OpenSpec proposal](openspec/changes/establish-rom/proposal.md), [design](openspec/changes/establish-rom/design.md), and [tasks](openspec/changes/establish-rom/tasks.md). See [planning](docs/planning.md) for the workflow inspired by Beskid.

## License

[MIT](LICENSE). Copyright © 2026 Piotr Mikstacki.

## Design research

[Framework comparisons and Rust crate candidates](docs/research/frameworks-and-rust-crates.md) inform the design; they are not an approved dependency list.

[Quality gates](docs/quality.md) turn the [Beskid baseline audit](docs/research/beskid-quality-baseline.md) into concrete delivery criteria.

## Development

Use `./scripts/check` for local verification and `./scripts/build` once the Rust workspace exists. See the [persistent NixOS environment](infra/nixos/README.md). GitHub Actions is disabled. The [demo](demo/README.md) is reserved for an application built after the first core milestone.
