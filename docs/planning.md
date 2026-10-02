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

The project begins with lessons from a prior Go resource framework: preserve explicit false values, prevent observer updates from overwriting desired configuration, and enforce protected-field handling across storage and events. The available historical evidence was partial, so complete legacy compatibility is not claimed. The public repository contains newly written generic requirements, not private source excerpts, operational records, or credentials.

Confirmed direction: backend only, one main Rust library, resource/action/event model, persistence, extensible fields and Rust extension contracts. HTTP/RabbitMQ are separate; WASM is future work. Storage engine, event-sourcing strategy and concrete crates remain design decisions, not accepted dependencies.
