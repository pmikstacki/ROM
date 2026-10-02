# Executed authoring crate trials

Date: 2026-10-02. These are disposable experiments, independently rerun by the coordinator in the persistent NixOS container. They validate specific construction and diagnostic contracts, not a finished ROM API or a human usability study.

## Recommendation

Use a plain resource/field trait contract and normalized descriptor as the authority. Make a derive the usual authoring convenience, fluent methods the composition surface, and ordinary Rust functions the place for business behavior. Keep authorization, transactions and reactivity in the shared runtime. Sophisticated generated helpers are acceptable when application authors do not need to understand their machinery.

For generated required action inputs, **Bon is the provisional preference** based on the exercised error messages and optional setters. typed-builder remains viable and provides the same tested completeness guarantees. Conventional fluent builders remain useful for host configuration where conditional layering and explicit overrides are intended. These are complementary roles, not three competing framework implementations.

## Evidence and critique

| Trial | Executed result | What to borrow | Limit |
| --- | --- | --- | --- |
| [Resource derive](https://github.com/pmikstacki/ROM/tree/prototype/derive-contract/prototypes/derive-contract) | Five positive tests, seven compile-failure fixtures checking message and primary source line, and a manual-only dependency graph without macro dependencies. | Emit field-trait calls so Rust resolves aliases, nested options/lists and custom types. Preserve source spans. Keep manual and generated implementations on one contract. | No codecs, typed actions or live queries in this trial; explicit crate-path override, not automatic dependency-name discovery. |
| [Builder comparison](https://github.com/pmikstacki/ROM/tree/prototype/authoring-builders/prototypes/authoring-builders) | Three valid construction paths, twelve expected compile failures and four executed runtime controls. Both generated styles work through reusable helpers without caller-written state types. | Required input checks and fluent composition. Bon's missing/already-set diagnostics identify the member more directly in these fixtures. | Diagnostics still contain generated type names. No combined resource-derive/action-builder integration, editor trial or human usability study. |
| Conventional builder controls | Missing inputs compile and return a typed error at build time; repeated assignment compiles and the last value wins. Wrong Rust types still fail compilation. | Stable builder type and straightforward conditional configuration. | This control is not a manually implemented typestate builder; it does not prove compile-time protection is impossible without a macro crate. |

Both full verifier commands passed formatting and Clippy with warnings denied on Rust/Cargo **1.99.0**. Exact tested crates: Bon **3.10.1**, typed-builder **0.23.2**, Syn **3.0.6**, Quote **1.0.47**, proc-macro2 **1.0.107**. Source and lockfiles are retained on the linked branches. The container's old compiler was upgraded rather than constraining current-crate selection. Toolchain setup is recorded in [the NixOS environment](../../infra/nixos/README.md); ROM's production MSRV remains a separate decision.

The derive initially put an unsupported-field error on the derive annotation. A source-location assertion exposed this; field-spanned checking expressions now put it on the actual field type. This is a concrete improvement bought by more careful framework internals.

Both generated builders reject naïve conditional reassignment because a setter changes the builder type. Computing the value first or putting the branch in a framework-owned helper works. Neither requires the application author to write the complicated helper signature. Internal implementation effort is not a reason to reject a clean public API.

The builder trial distinguishes absent, explicit null and present values in Rust construction. It does **not** prove JSON PATCH decoding. typed-builder's convenience setter that strips Option needs an explicit fallback to accept None; the executed fallback succeeds. ROM should own clear patch semantics rather than inherit a helper crate's defaults accidentally.

Fresh-target checks observed approximately 0.08 seconds for the conventional builder, 2.25 seconds for typed-builder and 3.95 seconds for Bon in the coordinator's run. This is one small cached-source host observation, not a performance ranking, scaling prediction or adoption threshold. Detailed diagnostics and observations accompany the source.

## Responsibility split to carry forward

| Stage | Responsibility |
| --- | --- |
| Derive and generated Rust | Structural bindings, typed field references, thin invocation/codec helpers, local attribute checks and useful compile diagnostics. Trait resolution, rather than spelling heuristics, establishes field capability. |
| Fluent composition and startup | Combine behavior with generated fields without repeating them; reject duplicate identities, contradictory configuration and unsupported adapter capabilities; freeze a validated registry. |
| Shared runtime | Validate actual values, current permissions and revisions; commit state/events/outcomes; enforce admission, retries, policy freshness, live reads and reactions. |

Code generation emits code that will execute later. It cannot decide future permissions or database conflicts at compile time. Generated resource engines with independent mutation/security logic would undermine the shared contract and are not recommended.

## Remaining acceptance work

The reusable-library experiment addresses transport-free operations and filtered live queries separately. Combining that runtime with the preferred derive and action builders still needs a real downstream consumer. Before production dependency adoption, review advisories, licenses, features and MSRV, then test the packaged facade, renamed/reexported dependencies, documentation and editor navigation. Syn/Quote/proc-macro2 and both builder candidates were executed here; darling, proc-macro-crate, trybuild and cargo-expand were researched but not exercised as trial dependencies.

The production protocol must make metadata, field codecs and generated bindings agree. A separate Serde derive must not silently become another authoritative field schema. Neither small trial establishes that complete protocol contract.
