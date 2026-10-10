# Support captured transition validation

## Why

Application validation needs runtime-owned catalogs. The current function pointer cannot capture those dependencies.
Astral Plane demonstrates the need without a process-global registry.

## What changes

Accept a synchronous `Fn` callback with `Send + Sync + 'static` bounds.
Store the callback behind private `Arc` ownership. Keep the shared transition gate and receipt semantics unchanged.
Test two actual Runtime instances against each supported native adapter.

## Compatibility and migration

Keep `rom::Definition::validate_transition` at its current public path.
Named functions, function pointers and non-capturing closures remain valid callers.
The method becomes generic; code naming its exact old method-function type needs recompilation or adaptation.
No data, archive, descriptor, or wire format changes. No new dependencies.
Reversion removes captured callback support but requires no data conversion.

## Scope

Do not change action, policy, field-policy, or query-policy callbacks.
Do not add asynchronous validation, external I/O, or process-global state.
