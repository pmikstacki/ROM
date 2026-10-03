---
name: rom-native-extension
description: Use when implementing or checking a native ROM module, custom codec, Storage adapter, or optional BlobStore adapter.
---

# Test a native extension

Read the [native contract](../../../docs/native-extensions.md) for the selected port and its limits.
Use the [bundle procedure](../README.md) to run `native` profile and feature preflight.

1. Run the [external fixture example](../assets/native/run.mjs) with `native --run`.
2. Use its generated integration test and local fixture owner as the starting point.
3. Keep inspection and reopen methods on that owner type.
4. Return fresh, exclusive storage for every factory call.
5. Admit the requested profile before any fixture operation.
6. Test explicit codec input, canonical output, typed value, and rejected vectors.
7. Shut down and release Runtime and storage clones before reopening the same backing store.
8. Make one deliberately broken fixture fail its named shared assertion.
9. Retain the adapter's fault, ownership, process-exit, and recovery tests separately.

For BlobStore, select the optional `blob` feature and the guide's disposable namespace and object-bound prerequisites.
Run real provider acceptance only through its separately selected command.

Completion requires a passing public baseline and executed wrong-profile and broken-fixture negatives.
Report the vectors and ports tested.
A baseline pass covers those cases; it is not complete crash or provider certification.
