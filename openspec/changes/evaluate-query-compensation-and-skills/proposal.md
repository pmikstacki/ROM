# Query authoring, compensation and extension research

## Why
The owner accepted moving query views, schema-driven filters/sorting and restrict as the default relationship-deletion policy. They requested three work areas:

- Executable comparison of code generation, hybrid, and runtime query authoring.
- A compensation test mechanism with useful cases.
- Separate research for a ROM skills library and WASM extensions.

## What Changes
- Compare query authoring and adapter translation on the same semantics/data, then promote the justified filters/sorting design with maintained conformance.
- Build a disposable native compensation harness over ordinary Resources/actions and the real runtime, distinguishing domain outcome routing from missing generic failure hooks.
- Research a version-aware skills catalog and a bounded WASM extension boundary, with a small engine probe if feasible.
- Preserve reports, immutable experiment code and explicit executed/unexecuted boundaries.
- Compare a small strategy selector and database-planner adapter boundary after reviewing DataFusion and egg; keep local physical planning in the database.

## Impact
Queries affect the shared typed/wire/live selection path and must preserve authority, exact field semantics and moving-view behavior. Compensation and WASM trials do not silently become maintained features. Aggregates, reports, production Studio and relationship-integrity implementation are outside this change; restrict is recorded as the accepted policy for future integrity work.
