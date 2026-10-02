## 1. Validate and inventory
- [x] 1.1 Independently verify and publish configuration, transport, integrated-core, auth-provider and reaction-chain trials. Evidence: docs/research/*-trial-results.md, integrated-core-probe-results.md, auth-provider-probe-results.md and reaction-chain-results.md.
- [ ] 1.2 Complete the decision register: behavior decisions, configurable policy, tested implementation choices and deferred profiles.
- [x] 1.3 Review the probe for promotion defects and record the integrated acceptance plan. Three reproduced findings have maintained regressions and independently verified fixes; docs/research/integrated-core-promotion-review.md.

## 2. Introduce maintained authoring and execution
- [x] 2.1 Promote a root Cargo workspace with rom, optional derive, a reference database adapter and an external consumer; preserve one accepted descriptor. Additional adapter conformance remains 3.1.
- [ ] 2.2 Implement bounded asynchronous reads, actions, live queries and shutdown; verify caller cancellation, actor expiry and field projection.
- [ ] 2.3 Verify codec/presence/custom-field conformance and compile-failure diagnostics through the public API.

## 3. Integrate durable behavior
- [x] 3.1 Run atomic state/revision/receipt/event/effect conformance on SQLite and redb, including lost replies and restart.
- [ ] 3.2 Implement recoverable reaction work with service identity, step idempotency, dependency/no-op suppression and causal/retry/work budgets.
- [ ] 3.3 Integrate notification delivery and blob lifecycle contracts with their tested adapters and explicit external-effect limits.
- [ ] 3.4 Integrate configuration Resource ingestion, source authority/provenance and visible activation state; test failed reload and restart.
- [ ] 3.5 Integrate User/provider configuration Resources and verified identity mapping without a second domain engine.

## 4. Connect and verify
- [ ] 4.1 Implement generic HTTP invocation/live/journal profiles and run shared embedded/wire conformance vectors.
- [ ] 4.2 Verify retention, capacity exhaustion, diagnostics, backup/recovery and explicit unsupported-profile failures.
- [ ] 4.3 Run fmt, Clippy, tests, doctests, rustdoc, declared MSRV, dependency/advisory/license checks and packaged-consumer verification.
- [ ] 4.4 Document the author walkthrough and actual limitations; expose the post-core demo through the completed library only.
- [ ] 4.5 Independently review the combined implementation, resolve findings, publish evidence and update the research/decision inventory.

No production data migration or deletion applies. Existing broader design changes
remain open for deferred requirements; this scoped MVP must not silently mark
them all implemented.
