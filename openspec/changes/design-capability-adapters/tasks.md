## 1. Validate contracts

- [x] 1.1 Research representative relational and non-relational transaction, consistency, journal and query contracts; evidence in docs/research/storage-*-backends.md.
- [x] 1.2 Research folder/S3 abstractions and named Rust notification channels; evidence in docs/research/storage-and-notification-adapters.md.
- [x] 1.3 Execute the same bounded persistence suite with SQLite and redb, including rollback, unknown outcomes and process recovery; evidence in docs/research/capability-prototype-results.md.
- [x] 1.4 Execute folder and actual loopback S3-compatible blob probes, including response loss and explicit unsupported conditions; evidence in docs/research/capability-prototype-results.md.
- [x] 1.5 Execute durable notification intent, retry, recovery and duplicate-effect controls using local receivers; evidence in docs/research/capability-prototype-results.md.
- [ ] 1.6 Stabilize async interfaces, profiles, typed query semantics, limits and retention with a downstream human authoring walkthrough.
- [x] 1.7 Compare cache-disabled, single-flight, Moka and quick_cache paths with shared correctness tests and repeated end-to-end measurements; evaluate Salsa separately for pure derived reads. Evidence and untested production limits: docs/research/cache-prototype-results.md.

## 2. Introduce production contracts

- [ ] 2.1 Implement core-owned contracts and frozen capability registration with driver-free core dependency checks.
- [ ] 2.2 Implement a reference persistence adapter and the complete atomic bundle including effect intentions.
- [ ] 2.3 Implement bounded blob adapters, access policy and orphan/cleanup lifecycle.
- [ ] 2.4 Integrate typed channels and durable work claims with shared execution supervision and delivery policy.

## 3. Compatibility and migration

- [ ] 3.1 Specify resource/receipt/event/payload version compatibility and adapter migration with cursor-generation reset/resync rules.
- [ ] 3.2 Specify expiry and cleanup without silently re-enabling old action or delivery identities.

## 4. Verify

- [ ] 4.1 Run combined conformance, resilience, privacy and dependency-boundary suites on each supported deployment profile.
- [ ] 4.2 Validate packaged consumer ergonomics and same-resource behavior after adapter substitution.
- [ ] 4.3 After production requirements are implemented and supported by evidence, archive the change.

No existing production code or data is deleted or migrated by this proposal. A completed experiment does not mark a production implementation task as complete.
