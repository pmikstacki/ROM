# Resource field contract

A declaration uses the same `Field` contract for metadata, input decoding,
persistence normalization and typed selectors. Supported built-ins are `String`,
`bool`, `u64`, `i64`, `FiniteF64`, `Option<T>`, `Presence<T>`, `Vec<T>`,
`BTreeMap<String, T>` and `ResourceRef<R>`. A native plugin implements `Field` for
its own newtype and declares its wire shape. `Shape::Enum` constrains custom
string codecs to a nonempty, distinct list of at most 256 variants.

```rust
use rom::{FiniteF64, Resource, ResourceRef};
#[derive(Clone, Resource)]
#[resource(name = "projects")]
struct Project { title: String }
#[derive(Clone, Resource)]
#[resource(name = "measurements")]
struct Measurement {
    project: ResourceRef<Project>,
    reading: FiniteF64,
    labels: Vec<Option<String>>,
}
let reading = FiniteF64::new(0.25)?;
let project = ResourceRef::<Project>::new("project-one")?;
# Ok::<(), rom::Error>(())
```

Use `FiniteF64::new(value)` or `TryFrom<f64>` for floating values. An early trial
with raw `f64` reproduced a concrete data-loss case: `Some(NaN)` encoded as JSON
null and decoded as `None`. The maintained wrapper stores a private value. Before construction, it
rejects NaN/infinity. It is ordinary binary floating point,
not a financial decimal type; decimal/date/UUID fields can be explicit custom
codecs without changing the resource runtime.

A `ResourceRef<R>` carries a nonempty target identity and advertises R's kind.
Registration requires that target kind to be registered. Mutual references are
allowed. This profile promises typed identity, **not** foreign-key existence,
delete restriction or cascade. If applications need enforceable cross-resource
integrity, they must use a later explicit capability. They must not infer this
integrity from the type.

Collections validate each element/value. Empty collections, empty strings, false
and zero are values. `Option<T>` uses JSON null. Registration rejects nested nullable shapes such as
`Vec<Option<Option<bool>>>` because that wire form cannot distinguish their states. Shape nesting is limited to 16. These are
structural constraints, not a memory allocator budget; the execution boundary
also limits serialized commands and snapshots.

Create and replace must include every field except top-level `Presence<T>` fields.
An `Option<T>` field must still have an explicit value or null. Unknown fields and
mismatched shapes are rejected. [Partial changes](presence-and-patch.md) use
`Command::patch` with `Patch::set` and `Patch::remove`. Omitted patch entries stay
unchanged. Nullable values can be set to null. Only Presence fields can be
removed. The same codecs, authorization and revision checks apply. No separate
Serde attribute schema is accepted by derive.

Validation evidence: eight public consumer tests exercise roundtrip through the
runtime, bad nested data, enum/codecs disagreement, invalid nullable shapes,
missing reference kinds, cyclic declarations and nonfinite construction. The
complete existing verifier passed after the codec changes; the final two added
cases were also independently run with Clippy. The negative NaN experiment failed
before replacing the raw-float encoding with `FiniteF64`.
