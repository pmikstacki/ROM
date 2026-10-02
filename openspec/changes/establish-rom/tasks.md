## 1. Validate the design

- [x] 1.1 Compare relevant frameworks and Rust crates using primary sources; document the smallest suitable stack.
- [x] 1.2 Audit representative Beskid code and quality gates; propose measurable ROM gates.
- [ ] 1.3 Resolve core public signatures, field encodings, reference database and reaction semantics in follow-up specifications.

- [x] 1.4 Validate Tokio/Rayon integration, async I/O and CPU completion with a bounded execution probe and retain a tested dependency lockfile.
- [ ] 1.5 Select the production dependency versions and supported toolchain; prototype pins are experimental.
- [ ] 1.6 Prototype generic live queries, including filter membership, snapshot/setup races, authorization changes and bounded slow consumers.

## 2. Introduce the core

- [ ] 2.1 Implement resource identity, versioned kinds and field validation with resource-model scenarios.
- [ ] 2.2 Implement the Rust field registry and conformance harness with custom-type round-trip and capability rejection tests.
- [ ] 2.3 Implement the action pipeline with presence, authorization, no-op, conflict and idempotency tests.

- [ ] 2.4 Implement the Tokio/Rayon execution seam with host-owned pools, bounded admission, nonblocking CPU completion, cancellation and stable domain identity tests.

## 3. Introduce durable reactivity

- [ ] 3.1 Implement a reference persistence adapter and verify rollback, concurrency and restart durability.
- [ ] 3.2 Implement committed-event subscriptions and durable reaction progress; test interruption and duplicate delivery.
- [ ] 3.3 Demonstrate a reaction changing a second resource exclusively through the core action interface.
- [ ] 3.4 Implement live resource reads from the shared resource/query definition with dependency invalidation and explicit recovery semantics.

## 4. Extend and verify

- [ ] 4.1 Specify separate HTTP and RabbitMQ integrations; verify core dependency independence.
- [ ] 4.2 Add schema-evolution and compatibility tests before any persisted format is released.
- [ ] 4.3 Verify all capability scenarios and adopted quality gates before marking implementation complete.
- [ ] 4.4 Archive the implemented change and promote requirements to baseline specifications.

No legacy data or implementation exists to migrate or delete in this initial change. WASM execution and frontend work are deferred.
