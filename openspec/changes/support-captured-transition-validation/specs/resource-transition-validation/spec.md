## ADDED Requirements

### Requirement: Runtime-owned captured transition dependencies

ROM SHALL accept a synchronous transition validator that captures owned application state and implements `Fn + Send + Sync + 'static`.
ROM SHALL retain the existing public setter path and ordinary named-function, function-pointer, and non-capturing-closure callers.
ROM SHALL reject captures that do not satisfy the declared thread-safety and lifetime bounds.

#### Scenario: Independent runtime catalogs

- GIVEN two Runtime instances with separate real SQLite or redb databases and conflicting captured catalogs
- WHEN each runtime receives the same proposed Resource value
- THEN each runtime SHALL apply its own captured catalog
- AND a later update of one catalog SHALL NOT change the other runtime's validation result

#### Scenario: Compatible and invalid callers

- GIVEN an external application using the public Definition setter
- WHEN it supplies a named function, function pointer, non-capturing closure, or owned thread-safe capture
- THEN ROM SHALL accept the caller
- AND non-Send, non-Sync, and non-static captured callbacks SHALL fail compilation at the supplied call or capture

### Requirement: Captured validation preserves commit and replay authority

ROM SHALL apply captured validation at the existing authorized revision-checked transition gate.
The callback SHALL remain synchronous, bounded, non-reentrant, and free from I/O and external effects.
The callback SHALL read one immutable dependency snapshot for each evaluation.
ROM SHALL preserve current authority, exact receipt replay, and atomic rejection semantics.

#### Scenario: Rejection and callback panic

- GIVEN a committed Resource and a captured validator that rejects or panics on a proposed action
- WHEN the action runs
- THEN ROM SHALL leave state, receipts, events, effects, and pending work unchanged
- AND a later valid action SHALL still commit

#### Scenario: Replay after catalog publication

- GIVEN an accepted action receipt and a later catalog snapshot that rejects its former candidate
- WHEN the caller repeats the exact request with current authority
- THEN ROM SHALL return the stored authorized outcome without revalidating the obsolete transition
- AND ROM SHALL create no additional event or work
- AND changed request input SHALL fail identity matching

#### Scenario: Revoked replay authority

- GIVEN a stored action receipt and a caller whose authority was revoked
- WHEN the caller repeats the exact request
- THEN ROM SHALL deny disclosure
- AND ROM SHALL preserve the earlier commit and receipt
