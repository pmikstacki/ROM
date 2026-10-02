# Planning and evidence

ROM adopts Beskid's spec-driven workflow: proposal, capability deltas, design, and checkable tasks. Requirements use SHALL/MUST and GIVEN/WHEN/THEN scenarios. Informative documents link to the requirements rather than becoming a second normative source.

The reference inspected was [Beskid's OpenSpec configuration](https://github.com/Cyber-Nomad-Collective/beskid/blob/main/openspec/config.yaml) and its change directory structure on 2 October 2026. ROM does not inherit Beskid's compiler-specific requirements or its entire process.

## Lifecycle

1. Describe an observable change in a named OpenSpec change directory.
2. Write requirements and failure scenarios before product code.
3. Resolve interface and compatibility decisions in the design.
4. Implement small, testable tasks; check them off only with verification evidence.
5. Validate the change; archive it and promote deltas to baseline specs when delivered.

The initial change remains proposed. Empty baseline specs do not mean the proposal is implemented.

## Local commands

The planning tool is pinned to OpenSpec 1.14.0 for reproducibility:

```sh
npm exec --yes --package=@fission-ai/openspec@1.14.0 -- openspec validate establish-rom --strict
npm exec --yes --package=@fission-ai/openspec@1.14.0 -- openspec status --change establish-rom
```

Node is planning tooling, not a ROM runtime dependency.

## Origin of the design

The project develops the owner's original vision of a reactive resource framework. The earlier Go implementation was deliberately narrowed to a schema/metadata generator after a scope reduction. Its lack of runtime, persistence and event-delivery machinery is not classified as a defect. The ROM review concerns the standalone generator, not applications built on it.

Static source review suggests preserving an explicit resource/field descriptor model, centralized type capabilities and field policies. ROM should strengthen recursive type fidelity, stable identity, validation of ambiguous definitions, immutable metadata and consumer-level conformance tests. No legacy code was executed. Private source and detailed source evidence remain outside this public repository.

Confirmed direction: backend only, one main Rust library, resource/action/event model, persistence, extensible fields and Rust extension contracts, with Tokio and Rayon for execution. HTTP/RabbitMQ are separate; WASM is future work. The production storage engine, state/history strategy, dependency releases and remaining crates still need decisions. Tested prototype pins do not freeze the production dependency graph.

## Local verification

Run `./scripts/check` before committing and `./scripts/build` when a Rust workspace exists. GitHub is used for code hosting only; Actions is disabled. The check script explicitly reports when Rust checks are not yet applicable. Build exits with an explanatory error until implementation begins.

## Research-led development

Before locking a substantial core abstraction, compare primary-source precedents and test alternatives in a small, disposable experiment. Record the question, acceptance criteria, findings and selected tradeoff in the relevant OpenSpec design. An experiment does not become production code merely because it runs. Favor the smallest interface that makes downstream application development straightforward.

The `demo/` application begins only after the first core milestone is ready. Releases are built and verified locally in NixOS; GitHub hosts source, not the build pipeline.
