# Quality gates

These are ROM's adopted delivery criteria. Gates for Rust and databases become executable when those components exist; this document does not claim they pass today.

- Run `./scripts/check` locally before sharing changes. GitHub hosts code only; Actions is disabled. Behavior changes carry concrete success and failure scenarios.
- The first Rust crate adds formatting, Clippy with warnings denied, workspace tests and doctests, documentation with warnings denied, and a declared minimum Rust version check.
- The domain core forbids unsafe Rust and denies ignored must-use results. Any future unsafe adapter requires separate justification and documented invariants.
- An external example must implement a custom field using only public interfaces.
- Every supported durable database adapter passes the same real-database tests for atomic state/event writes, rollback, stale revisions and restart recovery.
- Mutation tests distinguish missing, null, false, zero and empty values. Rejected actions produce no state change or success event.
- Reactive tests inject interruption before and after commit and acknowledgement, exercise duplicates and retries, and verify bounded work and documented ordering.
- Dependency adoption checks advisories, licenses, supported Rust versions and required features. Test the core without transport integrations.
- Releases require compatibility review, migration notes and a packaged-consumer smoke test.

The [Beskid baseline audit](research/beskid-quality-baseline.md) explains the evidence behind these criteria. This is a stronger target in specific areas, not a claim that an unimplemented library already has higher code quality.
