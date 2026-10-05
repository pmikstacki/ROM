# Native alpha extension profile

Profile revision 1 covers trusted Rust extensions compiled with an application.
The current prepared package set is `0.0.3`, with Rust 1.99.
The historical accepted package set was `0.1.0-alpha.1`. Profile revision 1 remains unchanged.
Record the executed source and lockfile hashes with verification results.
The machine-readable contract is `extensions/native-alpha-v1.json` in the supplied ROM source tree.

This profile does not promise a stable binary ABI, dynamic loading, WASM, or compatibility with arbitrary remote providers.
Custom Field codecs belong to their enclosing Resource and retained request codecs.
There is no independent Field identity, registry, or cross-codec identity collision guarantee.
The broader independent Field requirements in the open `establish-rom` proposal remain open.
Resource versions and independent Field versions are not equivalent.

## Compatibility boundaries

| Boundary | Version owner | Mismatch behavior and upgrade obligation |
| --- | --- | --- |
| Resource and custom Field codecs | Resource descriptor; retained historical request codec | Registration rejects descriptor mismatch. Use explicit schema migration and historical request replay codecs. |
| Action input | Compiled Input codec within the action and Resource contract | Invalid input fails before mutation. Preserve historical request normalization when changing its codec. |
| Reactions and channels | Positive definition versions and persisted consumer bindings | Incompatible definitions stop old work. Retain payload, identity, and delivery profile during recovery. |
| Query adapter | Semantics, profile, and encoding versions, all currently 1 | Validate the complete query response contract. Unsupported collection ordering remains `Invalid`. |
| Operator transport | Operator protocol version 1 | Reject unsupported protocol versions. Unsupported adapter ports remain `Unsupported`. |
| Native storage | Format 8 | Ordinary open rejects incompatible markers. Use the explicit upgrade path. |
| Archive | Format 6 | Decode current archives strictly. Use explicit legacy conversion before restore. |
| Native source API | Exact alpha package set and executed source identity | Recompile against the selected source package set. This is not a binary ABI. |
| Author skills | Bundle and profile revision; declared workflow features | Reject an obsolete profile, missing feature, or missing asset before execution. |

Core and production adapters do not depend on the conformance crate or author skills.
The conformance crate is a development dependency and uses only public ROM interfaces.
Its default features cover Field codecs and storage. The optional Cargo feature `blob` adds blob assertions.
The source package publication policy remains disabled.

## Public assertions

Use `rom_conformance::profile::require(version)` to admit the requested profile.
`PROFILE_VERSION` is 1.
Failures return `ConformanceError` with static profile, case, and category metadata.
Errors do not retain arbitrary adapter errors, stored values, or credentials.

`field::codec<F: rom::Field + PartialEq>(cases: &[CodecCase<F>], invalid: &[rom::Value])` checks typed and canonical results.
Each `CodecCase` supplies `input`, `canonical`, and `expected` separately.
The canonical value need not equal the input.
The helper checks invalid input and canonical round trips.
These assertions cover only the supplied vectors, not complete descriptor or shape verification of arbitrary codec code.
Resource registration, missing/default behavior, and manual/derive equivalence require their separate application tests.

`storage::basic(factory)` runs the current profile through a fresh fixture for each scenario.
`storage::for_profile(version, factory)` admits the requested profile before calling the factory.
Both accept `impl Fn() -> rom::Result<Box<dyn StorageFixture>>` and return an asynchronous `ConformanceResult`.
The profile argument identifies the assertion contract, not a persisted adapter format.

The factory must create fresh, exclusive backing storage for each scenario.
Implement `StorageFixture` on a local owner type with `storage`, `facts`, and `reopen` methods.
`storage` returns `Arc<dyn rom::Storage>`.
`facts` returns `rom::Result<StorageFacts>` with counts in this order: rows, events, receipts, effects.
It also returns ordered event Rows and ordered `(receipt identity, Intent)` effects.
This is a trusted test inspection seam, not a production Storage port or an authorization boundary.
The runner releases all storage clones and shuts down and drops its Runtime before `reopen`.
The fixture must release its own backing handle before opening the same store again.

The shared two-commit assertion is:

```rust
pub fn assert_bundle(
    storage: &dyn rom::Storage,
    facts: &rom_conformance::StorageFacts,
    previous: &rom::Receipt,
    bundle: &rom::Bundle,
) -> rom_conformance::ConformanceResult;
```

This helper requires a fresh store with one created Row and two committed changes.
The first receipt must be `previous`; the second must be `bundle.receipt`.
The baseline update carries two effects, so counts must be `[1, 2, 2, 2]`.
It checks the current Row, second receipt, ordered events, and ordered effects.
It does not support arbitrary existing history.
Native fault tests reuse this assertion after recovery; they retain their interruption and process fixtures.
Passing the baseline does not replace crash, power-loss, migration, authorization, or full native recovery tests.

With the `blob` feature, `blob::basic(store: &dyn rom_blob::BlobStore)` returns an asynchronous `ConformanceResult`.
Use an exclusively owned disposable namespace and an adapter configured for a sixteen-byte object maximum.
The helper uses fixed digest keys and deletes them during the test.
It checks immutable conditional creation, concurrent creation, exact bounds, empty objects, and idempotent deletion.
Default acceptance uses the local folder adapter.
Real S3 acceptance is an explicit separate command with a disposable provider.

## Verification and source location

Use the explicitly supplied ROM source or extracted-package location for commands and API source inspection.
The source workspace calls these assertions from `tests/persistence` and `examples/consumer`.
Blob callers are in `crates/rom-blob-object-store/tests/conformance.rs`.
Engine fault injection, process recovery, descriptor mismatch, duplicate registration, and unsupported ports keep their existing tests.
The portable skill bundle contains its instruction assets; it does not contain compiled ROM or all release artifacts.
Agent evaluation does not establish human usability or a measured productivity improvement.
