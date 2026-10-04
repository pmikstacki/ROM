# Independent review of wrapped codec metadata

This source review covers commit `6dcf27c`. It does not replace the full combined release verifier.

No important consistency defect was found in the reviewed changes.

The `Field` implementations propagate custom codec identity through `Option`, `Vec`, `BTreeMap`, and `Presence`.
Their wrapper paths follow the outer shape toward the custom codec. A custom codec owns the remaining shape.
The derive emits the same identity and wrapper path for Resource bindings and named input fields.
Standalone `Presence` input remains opaque because its envelope differs from a named field value.

Registration checks wrapper paths against their shapes. It rejects paths longer than 16 wrappers and paths without a codec identity.
Resource bindings remain presentation metadata. Persisted `FieldDescriptor` and catalog identity do not include these bindings.
Discovery charges the borrowed wrapper representation before it clones the disclosed metadata.
Shape visibility checks still inspect nested Resource references. A custom codec does not override hidden reference checks.

The review examined these source files:

- `crates/rom/src/resource/fields.rs`
- `crates/rom/src/resource/input_descriptor.rs`
- `crates/rom/src/resource/definition.rs`
- `crates/rom/src/patch.rs`
- `crates/rom/src/discovery.rs`
- `crates/rom-derive/src/expansion.rs`

The recorded wrapper tests and native reopen evidence remain in `rom-0.0.2-wrapped-codec-results.md`.
This review did not rerun those tests independently. The coordinator owns the full verifier for the combined release source.
