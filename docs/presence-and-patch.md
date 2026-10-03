# Presence and partial changes

`Option<T>` is a required field whose value can be null. `Presence<T>` is a field that can be absent. `Presence<Option<T>>` represents all three cases without conflating them:

| Rust | Resource JSON |
|---|---|
| `Presence::Missing` | key omitted |
| `Presence::Value(None)` | key present with null |
| `Presence::Value(Some(value))` | key present with value |

Presence is permitted only on a top-level Resource field. Registration rejects presence nested in lists, maps, nullable or other presence types. Derive uses the same field list for descriptors, codecs and selectors. Custom manual codecs must honor those descriptors. Action input uses an explicit `{"presence":"missing"}` or `{"presence":"value","value":...}` envelope because an argument has no containing field to omit.

```rust,ignore
let command = Command::patch("note-1", Patch::new()
    .set(Note::title_field(), "Revised".to_owned())
    .remove(Note::memo_field()))
    .at_revision(3)
    .idempotency("edit-42");
let updated = runtime.execute(&actor, command).await?;
```

Omitted patch entries preserve their values. `set` with an explicit nullable value writes null. `remove` accepts a Presence selector. The wire equivalent rejects removal of required fields. `set` with `Presence::Missing` also removes the field. Repeated fluent assignments to one field use the last assignment. Empty patch is an authorized no-op with a receipt, not a new event.

Patch uses the standard operation namespace, separate from custom actions named `patch`. Accepted field codecs normalize values before fingerprinting. Revision checks, current authorization, durable receipts, events and reactions use the same runtime path. Even if its value is unchanged, every selected field needs write access. A complete read needs access to every declared field, including absent fields. Projected reads omit unauthorized fields.

Wire invocation example:

```json
{"kind":"notes","id":"note-1","expected":3,"idempotency":"edit-42","operation":{"type":"patch","input":{"title":{"op":"set","value":"Revised"},"memo":{"op":"remove"}}}}
```

`Note::memo_field().equals(Presence::Missing)` and `QuerySpec::absent("memo")` test absence. Equality to null only matches a present null. Structured query endpoints accept `{"kind":"notes","query":{"filters":[{"field":"memo","value":null,"absent":true}],"limit":20}}`. Absence requires an optional field and a null placeholder. Unknown predicates, mixed structured/legacy query envelopes and unknown properties are rejected. Existing single-equality envelopes remain supported.

The focused consumer tests cover codec/action roundtrips, patch/removal/retry/no-op, live queries, missing-field read protection, unchanged forbidden patch fields and required-field removal. A loopback TCP test covers structured query/patch/live parity and ambiguous envelope rejection. These are bounded MVP semantics, not arbitrary JSON Patch or schema editing.

Validation: six consumer tests and one new real-TCP test pass. The full local `scripts/check` passes on Rust 1.99.0. Removing the complete-field authorization check makes `complete_reads_cannot_reveal_that_a_forbidden_field_is_absent` fail; restoring it passes. This negative control proves the regression covers absent-field disclosure, rather than merely serializing the expected shape.
