## 1. Validate the design

- [x] 1.1 Compare relevant frameworks and Rust crates using primary sources; document the smallest suitable stack.
- [x] 1.2 Audit representative Beskid code and quality gates; propose measurable ROM gates.
- [ ] 1.3 Resolve core public signatures, field encodings, reference database and reaction semantics in follow-up specifications.

## 2. Introduce the core

- [ ] 2.1 Implement resource identity, versioned kinds and field validation with resource-model scenarios.
- [ ] 2.2 Implement the Rust field registry and conformance harness with custom-type round-trip and capability rejection tests.
- [ ] 2.3 Implement the action pipeline with presence, authorization, no-op, conflict and idempotency tests.

## 3. Introduce durable reactivity

- [ ] 3.1 Implement a reference persistence adapter and verify rollback, concurrency and restart durability.
- [ ] 3.2 Implement committed-event subscriptions and durable reaction progress; test interruption and duplicate delivery.
- [ ] 3.3 Demonstrate a reaction changing a second resource exclusively through the core action interface.

## 4. Extend and verify

- [ ] 4.1 Specify separate HTTP and RabbitMQ integrations; verify core dependency independence.
- [ ] 4.2 Add schema-evolution and compatibility tests before any persisted format is released.
- [ ] 4.3 Verify all capability scenarios and adopted quality gates before marking implementation complete.
- [ ] 4.4 Archive the implemented change and promote requirements to baseline specifications.

No legacy data or implementation exists to migrate or delete in this initial change. WASM execution and frontend work are deferred.
