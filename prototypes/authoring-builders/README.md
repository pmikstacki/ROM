# Disposable builder trials

This project compares authoring helpers for resource configuration and action inputs. It is not a ROM implementation or a dependency selection. Run `node verify.mjs` from this directory using the toolchain recorded in `evidence/observations.txt`. Both Cargo lockfiles are retained. The `fixtures` package deliberately contains broken programs: run its selected bins through the verification script, not `cargo build --all-targets` there.

The same small models appear in three modules: `bon_style`, `typed` (typed-builder), and `manual` (a conventional handwritten fluent builder). All construct `ResourceConfig { name, retries, description }` and `ActionInput { resource_id, expected_revision, note }`. Builders perform construction only; they do not enforce ROM authorization, persistence, field validation, or action transaction semantics.

## Versions and environment

Current registry releases checked on 2026-10-02: bon **3.10.1**, typed-builder **0.23.2**. Exact versions are pinned. bon's published manifest declares Rust **1.88.0** and edition 2024; typed-builder's manifest declares edition 2024 but no `rust-version`. Absence of a declaration is not an MSRV guarantee. Both actually passed on **Rust/Cargo 1.99.0** in `rom-dev`. `evidence/package-metadata.json` records the resolved manifests, including transitive dependencies. No release-level minimum was exhaustively tested.

The initial Rust/Cargo 1.82 environment rejected both latest manifests because edition2024 was not supported. The user then requested a toolchain upgrade; final comparison uses the latest pinned crates, rather than selecting old crates to fit the old environment. bon3.3.2 and typed-builder0.20.1 were briefly investigated before that instruction; they are not the tested selection or lockfile contents.

Primary references: [bon crate](https://crates.io/crates/bon/3.10.1), [bon published manifest](https://docs.rs/crate/bon/3.10.1/source/Cargo.toml), [typed-builder published manifest](https://docs.rs/crate/typed-builder/0.23.2/source/Cargo.toml), [typed-builder derive reference](https://docs.rs/typed-builder/0.23.2/typed_builder/derive.TypedBuilder.html).

## What the executable verifies

- Both derive builders reject missing required fields and duplicate assignment. Both reject omitted `expected_revision` even when `resource_id` is present.
- All three reject a string supplied for numeric `retries`.
- The handwritten builder is deliberately conventional, not handwritten typestate. Missing fields compile but return a typed `MissingField` at runtime; repeated setters compile and the last value wins. These are executed negative controls for compile-time protection, not unexpected test failures.
- All construct defaults and conditional optional values, and distinguish absent, explicit null, and a value using `Option<Option<String>>`. This verifies Rust construction, **not JSON deserialization semantics**.
- External callers exercise reusable derive-builder helpers for both runtime branches, without naming state types.
- Naïve conditional reassignment fails for the derives because setters change the builder's type. Computing conditional values before setting them, or placing that logic in ROM-owned helpers, succeeds. The handwritten builder supports reassignment and layering directly.
- typed-builder's `strip_option` alone rejects explicit `None`; a separately executed `fallback = note_option` setter accepts it. Keeping `Option` in the main fixture's setter also works. ROM must choose deliberate public missing/null semantics rather than exposing accidental macro defaults.

`verify.mjs` requires each compile-fail program to produce an error for that specific fixture target and relevant field/type, not merely a failed Cargo invocation. It stores actual rendered compiler output in `evidence/*.txt`. Accepted manual controls also execute assertions. Formatting and Clippy cover the valid main crate; intentionally invalid fixtures are separate.

## Human-facing comparison

The common call site is small for both derives:

```rust
let config = retry_policy(ResourceConfig::builder().name("sensor"), production)
    .build();
```

The tested helper supports either runtime value for `production`. bon's implementation uses named state traits; typed-builder uses tuple positions. That difference is chiefly a maintenance concern for ROM's reusable helper layer. Neither requires the application author to write those generic signatures. Framework-owned helper complexity is acceptable when it preserves this simple authoring experience. These generic helper signatures should remain a framework implementation choice rather than becoming a contract every resource plugin must implement.

For optional configuration, bon supplies `maybe_description(option)`. The tested typed-builder and manual models accept `description(option)`; typed-builder can alternatively generate both a convenience setter and an Option fallback. For nested optional action input, bon's `note(None)` produces explicit null while omission produces absence. The other two main models spell explicit null `note(Some(None))`. None of these Rust signatures alone defines a good public ROM patch API; a named patch operation could be clearer and would need its own test.

The conventional manual implementation makes runtime layering easy and exposes a single stable builder type. It trades away compile-time completeness and duplicate protection; reproducing those protections manually would be a different, untested implementation. A fluent surface does not imply typestate, and a derive need not dictate ROM's public surface.

Official [bon typestate documentation](https://bon-rs.com/guide/typestate-api) explains when named builder types become necessary. Its [conditional-building guide](https://bon-rs.com/guide/patterns/conditional-building) describes value-first and branch-local construction. The executed fixtures substantiate the limited patterns used here; broader custom extension ergonomics were not evaluated.

## Provisional interpretation

For this tested surface, **bon is the provisional preference for generated, statically required action inputs**: its primary diagnostic describes an unset/already-set member directly, and its paired optional setters are convenient. The error still contains long generated type paths; it is not a fully domain-specific ROM diagnostic. typed-builder provides the same tested construction guarantees with less compilation work in this observation, and remains a credible choice. Its missing-field error additionally proposes supplying an internal error-marker argument to `build`, which is an unhelpful user-facing suggestion. These are implementer judgments about concrete diagnostics, not a human usability study or a universal crate ranking.

Prefer a ROM-owned fluent authoring layer with these details hidden behind reusable helpers. Conventional builders remain plausible for runtime configuration layering where overrides are intentional. Neither crate is required by the execution runtime, and no whole-ROM or performance winner follows from this experiment.

| Exercised mistake | bon 3.10.1 | typed-builder 0.23.2 | Conventional manual |
| --- | --- | --- | --- |
| Missing `name` / revision | E0277: member was not set; generated state bounds follow | Warning names missing field; E0061 asks for internal error-marker argument | Compiles; typed `MissingField` returned at runtime |
| Duplicate `name` | E0277: member was already set | Warning names repeated field; E0308 expects generated error-marker type | Compiles; executed assertion confirms last value wins |
| String for `retries` | E0308, expected `u32` | E0308, expected `u32` | E0308, expected `u32` |
| Conditional reassignment | E0308, state type changed | E0308, tuple state changed | Compiles; both branches executed |

All 12 expected compile failures, four runtime controls, and three valid construction paths passed. `cargo fmt --check` and `cargo clippy --locked --all-targets --all-features -- -D warnings` passed for the valid crate. Full compiler outputs are retained; this table summarizes rather than sanitizes their complexity.

Remaining gaps include larger resource declarations, custom domain field types, public documentation/autocomplete, interaction with ROM's resource derive, fallible validation, macro reexports, downstream dependency renaming, semver stability, editor diagnostics, and human usability testing. The project uses public fields for transparent assertions; it does not prove builders prevent direct invalid struct construction.

## Compilation observations

The script checks each feature separately with a fresh local target directory, then repeats the identical check incrementally. Downloaded sources are already cached. It records wall time and logical target-file bytes. It does not measure peak memory, installed binary size, clean-machine dependency downloads, release optimization, or application runtime overhead. Results are single-host observations sensitive to shared machine load and order, not a general benchmark.

See `evidence/observations.txt` for the actual run and `evidence/*_*.txt` for compiler diagnostics.

| Feature | Fresh-target `cargo check` | Immediate repeated check | Logical target-file bytes |
| --- | ---: | ---: | ---: |
| manual | 83 ms | 16 ms | 327,001 |
| typed | 2,220 ms | 18 ms | 47,237,773 |
| bon | 3,978 ms | 19 ms | 87,026,161 |

The table records the initial successful run, not a stable threshold. Future verifier runs replace the evidence observations with new values. Compile cost deserves monitoring as ROM declarations grow; this tiny trial does not establish scaling or a budget violation.

From the host checkout, run:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/authoring-builders/prototypes/authoring-builders && node verify.mjs'
```
