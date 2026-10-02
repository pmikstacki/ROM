# Resource derive contract: disposable crate trial

Date: 2026-10-02. This is a small **executed trial**, not a stabilized ROM API or implementation of the entire framework. It tests whether a readable Rust declaration can generate the same ordinary trait contract as a manual implementation while preserving types and useful diagnostics.

The criterion is the application developer's experience. ROM maintainers should absorb necessary macro/type machinery; internal implementation effort is acceptable when it produces a simple, inspectable public interface. A small line count alone does not make an API pleasant.

## Run

From the project directory inside the development container:

```sh
./verify
```

From the host:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/derive-contract/prototypes/derive-contract && ./verify'
```

The script checks formatting, runs Clippy, executes five positive tests, then compiles seven deliberately invalid binaries and checks both the expected error and its primary source span. It also checks that the manual-only application's dependency graph contains no procedural-macro dependencies. All dependencies are locked. No server, database, HTTP stack, toolchain installation or extra test framework is needed. Expected-failure fixtures are excluded from ordinary workspace test/Clippy runs by the script.

## Actual crates and versions

| Crate | Exact version used | Declared MSRV from downloaded package manifest | Role exercised |
| --- | --- | --- | --- |
| `syn` | 3.0.6 | Rust 1.71 | Parse derive input, generics and structured attributes; construct source-spanned errors |
| `quote` | 1.0.47 | Rust 1.71 | Emit ordinary trait implementations and source-spanned typed field calls |
| `proc-macro2` | 1.0.107 | Rust 1.71 | Token stream representation used by the expansion function |

These were the current versions exposed by the public documentation/package registry at research time, rather than an older Syn 2 compatibility pin. All three declare `MIT OR Apache-2.0`. Exact versions and transitive `unicode-ident` are retained in `Cargo.lock`. The initial full trial passed on the existing Rust/Cargo 1.82 toolchain. After the user-requested shared container upgrade, final verification also passed on **rustc 1.99.0 (b940084d7 2026-09-28)** and **cargo 1.99.0 (5f94df478 2026-08-27)**, with matching rustfmt/Clippy. The package MSRV declaration was inspected, not independently established by running Rust 1.71. [Syn documentation](https://docs.rs/syn/3.0.6/syn/), [Quote documentation](https://docs.rs/quote/1.0.47/quote/), [proc-macro2 documentation](https://docs.rs/proc-macro2/1.0.107/proc_macro2/).

`proc-macro-crate` was researched but **not trialed or added**. It discovers the dependency's actual Cargo name; this experiment instead uses an explicit crate-path attribute so renamed-dependency hygiene is executable without an additional discovery mechanism. Automatic discovery may be preferable for final application ergonomics, and should be evaluated rather than dismissed because it makes framework internals more complex. [proc-macro-crate's contract](https://docs.rs/proc-macro-crate/3.5.0/proc_macro_crate/).

## What the application author writes

The facade has one opt-in `derive` feature; the trait and macro share the `Resource` name in their respective Rust namespaces.

```rust
use resource_contract::Resource;

type MaybeCode = Option<UpperCode>;
type Nested = Vec<Option<Vec<MaybeCode>>>;

#[derive(Resource)]
#[resource(name = "sensor")]
struct Sensor {
    #[resource(rename = "deviceCode")]
    code: MaybeCode,
    groups: Nested,
    enabled: bool,
}
```

This is a normal Rust struct, with only stable identity and an external-name override in attributes. Nullable and list behavior comes from actual `Option<T>`/`Vec<T>` trait implementations. There is no stringly typed field-type annotation and no separate copy of the field schema. The author can continue writing normal Rust methods, constructors and validators. The custom field shown in `fixtures/pass/src/lib.rs` implements `Field` with its own shape and ordinary validation code.

The generated code adds only:

- `Resource::NAME`, a stable name.
- `Resource::descriptor()`, using `FieldDescriptor::of::<ActualFieldType>(external_name)` for each field.
- `Resource::validate(&self)`, delegating to ordinary field validation with resource/field error context.
- Appropriate `Field` bounds for generic resource declarations.

It does not introduce generated global registries, names derived from abbreviated resource identity, hidden network calls, persistence code or attribute-encoded business programs. `core/src/lib.rs` is the readable contract; `fixtures/manual/src/lib.rs` demonstrates the implementation without a macro. This is transparency of responsibilities, not a claim that the full expanded token stream is pretty printed by this probe.

## Observed results

Five positive tests passed:

1. A custom Rust field, aliases to optional fields, and nested lists/options produce the exact expected recursive descriptor. Valid custom values and nulls validate; false is an ordinary boolean.
2. Invalid custom values report the resource and **external** field name; errors in nested containers include indices.
3. Two Rust types with the same name in separate modules coexist without generated-symbol collisions. A generic resource instantiated with `i64` produces integer metadata.
4. A hand-written `Resource` implementation supports the same generic consumer contract with derive disabled.
5. A Cargo-renamed facade dependency works with `#[resource(crate = "::rom")]`.

Seven compile-fail fixtures check source-local diagnostics:

| Invalid input | Expected developer-facing result |
| --- | --- |
| Unsupported field type | Rust `Field` trait error at the actual field type |
| Numeric resource-name attribute | Contextual message requiring a string literal |
| Repeated resource-name option | Error at the conflicting second option |
| Two fields renamed to the same external name | Error at the duplicate name plus a pointer to the first declaration |
| Unsupported field option such as `nullable` | Resource/field context and the supported option (`rename`) |
| Tuple struct | Explicit restriction to a struct with named fields |
| Repeated field rename option | Error at the conflicting rename |

The harness consumes Cargo's JSON diagnostics and checks message content and the primary span against the marked source line. It does not treat any arbitrary compile failure as success, and does not hide unrelated compile failures behind a stderr snapshot update.

The manual-only dependency-graph probe passes: the selected application's graph contains the facade and core, without `resource-contract-derive`, Syn or Quote. Workspace-wide tests can unify the derive feature; the graph check runs separately for the manual package to verify the opt-out contract.

## What improved during the experiment

The first macro scaffold generated no implementation, and positive compilation failed. The first real expansion had a punctuation error, also caught by compilation. More usefully, a diagnostic test caught an unsupported concrete field being blamed on `#[derive(Resource)]` instead of its declaration. Removing unnecessary concrete where-bounds and enforcing the contract through field-spanned typed calls moved the compiler's primary diagnostic onto the unsupported field. That negative probe now verifies the location, not merely the presence of an error.

Representative observed value diagnostic:

```text
resource `sensor`, field `deviceCode`: use nonempty uppercase ASCII letters
```

Representative observed compile diagnostic:

```text
resource Broken, field value: unknown option; use rename
```

The latter points to the offending field attribute. Framework implementation complexity bought the application author more useful feedback; keeping the macro implementation short would not justify retaining the worse error.

## Assessment for a fluent/derive hybrid

**Preferred direction from this trial:** keep a plain trait/descriptor contract as the semantic center, and provide derive as the normal convenience for Rust structs. The macro should delegate to field traits instead of interpreting type names. This directly preserved imported/aliased/nested types and made a custom Rust extension ordinary code.

Manual construction is a useful escape hatch and an implementation specification. It should not become the default application burden merely because avoiding code generation is simpler for ROM maintainers. Likewise, a final fluent layer should consume the same descriptors and typed callbacks; duplicating field semantics between fluent and derive paths would undo this benefit. The fluent layer itself is not implemented or compared here.

For a human adding a resource, the probe provides familiar struct syntax and small, local metadata attributes. For a custom field, there is a discoverable Rust trait. For a failing declaration or value, errors identify the local cause. Renaming a dependency currently requires one explicit path attribute, an acknowledged inconvenience that automatic discovery could remove. IDE completion, documentation discoverability and novice usability have not been measured with humans.

Custom actions and live queries are **outside this probe**. No claim is made that their authoring experience has been validated by a successful derive. A final ROM design still has to expose ordinary Rust action code, clear typed query APIs and resource/action context for failed mutations without forcing users to understand generated machinery.

## Static checks versus runtime guarantees

The compiler checks that field types implement `Field`; the macro checks attribute syntax and duplicate external field names within one struct. Value restrictions such as uppercase-only input are runtime validation, not something a derive can prove for every future value.

The trait contract is extensible, but a manual `Field` or `Resource` implementation can return inaccurate metadata or omit validation. There is no attempt to sandbox trusted native Rust extensions. Adapters would need a validated registration boundary and conformance tests.

Remaining limits:

- Supported built-ins are intentionally only `bool`, `i64`, `String`, `Option<T>` and `Vec<T>`; ordinary custom implementations extend this set.
- No serialization codec, PATCH presence representation, HTTP, storage, action dispatch or live query is generated. Nullability in a typed value is not a proof about omitted JSON fields.
- Structs with named fields only. The simple generic case is tested; lifetime-heavy, const-generic, associated-type and unusual where-clause cases are not exhaustively tested.
- Resource-name collisions across distinct declarations are not detected; that needs a registry boundary. The compile-fail collision probe concerns external field names within one declaration.
- `#[resource(rename)]` describes this contract's metadata. It is not automatically synchronized with Serde or other serialization attributes.
- Only explicit crate-path override and normal facade naming were tested. Automatic discovery, duplicate facade versions, reexport through a third crate and no-std support remain open.
- This is not a proc-macro hardening audit or performance benchmark. The macro does not deliberately inspect the filesystem or network, but procedural macros execute as compiler-side code. [Rust procedural-macro reference](https://doc.rust-lang.org/reference/procedural-macros.html).

Source-local error construction follows Syn's documented `Error`/`into_compile_error` approach. Normal trait-resolution errors are still produced by rustc, with spans carried into the generated field expressions. [Syn error API](https://docs.rs/syn/3.0.6/syn/struct.Error.html).
