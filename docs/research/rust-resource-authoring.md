# Rust resource authoring: derive, fluent API, and shared runtime

Research date: 2026-10-02. Status: design recommendation based on current primary documentation. This document itself is a source review; subsequent executed prototype results are recorded in [the companion crate trials](authoring-crate-trials.md). No human usability study or performance benchmark was performed. All proposed ROM signatures and examples below are **uncompiled illustrations**, not existing APIs. This note preserves ROM's premise: one authoritative Resource definition produces persistence-backed CRUD, custom actions, endpoints, and reactive subscriptions. Tokio and Rayon remain runtime tools.

## Recommendation

Use **`#[derive(Resource)]` for structural boilerplate, a fluent typed API for composition, and one shared runtime for execution**. Both derive and manual authoring should implement the same public contracts and produce the same normalized resource descriptor. Business logic stays in ordinary Rust functions. Generate specialized accessors, field identifiers, argument codecs, and thin invocation helpers where they improve the experience; retain authorization, transactions, idempotency, events, and reactivity in one engine.

Move statically knowable responsibilities into generated Rust and let rustc check the resulting bounds and signatures. Do not equate “done at compile time” with “done by the procedural macro itself.” Startup validates the actual composed application and adapter configuration; each request still validates untrusted data and current authority/state. This is a division of responsibility, not a compromise on resource-driven behavior.

Generating a field validator at compile time does not validate future values at compile time: the emitted validator still runs for each applicable input. Generation can remove repeated authoring and provide typed calls without removing dynamic checks. No runtime speedup is claimed without measurements.

Human usability should decide the syntax: a developer should understand a resource by reading its Rust definition and configuration, discover operations through completion and Rustdoc, write/test a custom action without learning a second language, and diagnose a bad declaration without inspecting generated internals. Fewer declaration lines are useful only when they preserve that experience.

**Owner's explicit priority:** ROM exists to implement the hard machinery once so application authors can define and manage resources automatically. Internal generic/helper/codegen complexity is not, by itself, a reason to reject an approach. Assess whether that complexity leaks into application code; first explore framework-owned helpers, facades and generated bindings that contain it. Maintainer effort is a tradeoff to document, not a substitute for the human-facing acceptance criterion.

## Alternatives and implementer judgment

| Approach | Strongest property | Main cost for human authors | Recommendation for ROM |
| --- | --- | --- | --- |
| **Derive + fluent typed configuration + shared runtime** | Ordinary Rust structure, generated repetitive bindings, composable configuration, common semantics. | Requires thoughtful generated names, diagnostics, and a clear boundary between attributes and builder methods. | **Preferred.** Offer a short happy path and a documented manual trait implementation as an escape hatch. |
| **All-runtime registry** | Easy composition from configuration and dynamically selected resources; one inspectable data model. | More repeated field/type declarations, string mistakes, and setup-time failures if used as the main static Rust API. | **Keep the registry as the runtime foundation**, not as the only ergonomic authoring surface. A future dynamic schema API may intentionally use it. |
| **Heavy macro DSL** | Can validate relationships among all syntax captured by one invocation and produce compact declarations. | More custom syntax, weaker ordinary Rust navigation, difficult expansion diagnostics, and another vocabulary to learn. | **Avoid initially.** Small derives and explicit function references cover the known need. Add richer syntax only when it removes demonstrated friction. |
| **Generated engine per resource kind** | Potential specialization and statically typed fast paths. | Larger expansions and builds; fixes to authorization/commit behavior can become scattered templates. | **Generate bindings, not copies of the engine.** Specialization needs measured benefit and conformance evidence, not an assumption that generation is always faster. |

These are architectural judgments about ROM's goals, not benchmark conclusions about macro-based code or existing crates.

## Primary precedents worth borrowing

| Reference | Verified mechanism | ROM lesson |
| --- | --- | --- |
| **Serde** | Derives implement `Serialize`/`Deserialize`; users can manually implement the same traits for unsupported cases. [Derive](https://serde.rs/derive.html), [manual implementation](https://serde.rs/impl-serialize.html) | Make the trait/descriptor contract primary. Derive is a convenient implementation, not the only door into the system. Do not make manual implementations bypass normal validation or authorization. |
| **clap** | Derive and builder APIs explicitly interoperate; static arguments can be combined with runtime configuration. Documentation also maps doc comments into help. [Derive reference](https://docs.rs/clap/latest/clap/_derive/index.html#mixing-builder-and-derive-apis) | Structural defaults plus fluent customization should compose into one definition. Carry resource/field/action documentation into generated API metadata and Rustdoc deliberately. |
| **Diesel** | `table!` emits table/column types. `Selectable` can generate backend-specific type checks for better error messages. Larger table feature limits have documented compilation costs. [Typed schema](https://docs.rs/diesel/latest/diesel/macro.table.html), [Selectable](https://docs.rs/diesel/latest/diesel/prelude/derive.Selectable.html) | Generate typed field handles and targeted checking code. Measure compile cost as schemas grow. A typed declaration does not prove the deployed database schema still matches. |
| **SeaORM** | Its documented entity-first workflow uses entity definitions, registry, and schema synchronization; dense entity derives reduce generated boilerplate. [Entity-first workflow](https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-first/), [entity formats](https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-format/) | Useful precedent for declarations feeding runtime schema machinery. Do not maintain independent ROM and ORM models by hand. Automatic schema synchronization is not a substitute for ROM's explicit migration/data-loss policy. Verify the exact SeaORM release/features before adopting this workflow. |
| **bon / typed-builder** | Both generate builders that reject missing required inputs and repeated setters at compile time. Bon also handles function/method builders. [Bon overview](https://bon-rs.com/guide/overview), [TypedBuilder](https://docs.rs/typed-builder/latest/typed_builder/derive.TypedBuilder.html) | Evaluate one for ergonomic action inputs/configuration instead of writing a generic builder generator. Apply typestate where it prevents common mistakes; do not encode every policy or arbitrary dynamic registry entry in public generic parameters. |

Serde and clap are the strongest architectural precedents. Diesel is especially useful for typed field references and deliberately improved errors. Bon/typed-builder are implementation candidates, not reasons to add another annotation to every resource. Neither ORM is selected by this authoring recommendation.

## What a macro can know

Rust's procedural-macro interface consumes and produces token streams. A derive receives its annotated item and appends generated items; helper attributes describe syntax available within that item. It is not a supported query interface into the fully type-checked application. Procedural macros also have hygiene/name-resolution concerns, so generated paths and identifiers need care. [Rust Reference](https://doc.rust-lang.org/reference/procedural-macros.html).

The practical inference is that a derive can inspect the written field type and attributes, but cannot generally resolve a type alias, inspect arbitrary traits implemented in another crate, discover every resource in the final application, or infer a business function's database dependencies. Emit trait-based Rust code so rustc performs semantic checking; use explicit registration and dependency declarations for information outside the item's syntax. This avoids fragile attempts to recreate compiler name resolution inside the macro.

Do not classify optionality by checking whether a type's last identifier spells `Option`. An alias can refer to the standard option type, and another type can use that name. Generate calls such as `<FieldTy as FieldType>::metadata()` and implement the local `FieldType` trait for supported primitives, `Option<T>`, collections, and application newtypes. The compiler resolves aliases and bounds. A custom plugin supplies metadata/codecs/capabilities through the contract; its name is not its behavior.

Keep value nullability separate from action-input presence: `Option<T>` does not by itself describe omitted versus explicit null versus a value in an update. Represent that distinction in the generated patch/input contract. Similarly, use stable logical type IDs and schema versions for persistence, rather than Rust type names or `TypeId` as a storage protocol. These are proposed ROM semantics.

## Check placement

| Responsibility | Compile time | Registration/startup | Request, commit, or delivery |
| --- | --- | --- | --- |
| Attribute grammar and conflicting local settings | Derive reports unknown keys, duplicates, invalid combinations, and malformed literals at source spans. | Validate equivalent manually supplied descriptors. | None for trusted installed metadata. |
| Field/action Rust types | Generated trait bounds and typed function references check codec/type and handler compatibility. | Validate dynamic type registrations, if supported. | Decode and validate actual values. |
| Required action inputs | Typed builders can reject omitted required arguments in Rust callers. | Check descriptor consistency and defaults. | HTTP/message callers still need missing/null/value validation. |
| Type capability versus selected adapter | Possible when a fixed adapter is explicitly represented in a typed API. | Required for runtime-selected adapters, mappings, indexes, and query capabilities. | Reject unsupported dynamic query operations. |
| Resource IDs, route names, references | Detect collisions/references inside syntax explicitly available to one expansion. | Check the complete registered graph, naming collisions, exposure, and adapter availability. | Enforce target existence and current resource relationship constraints. |
| Default expressions and validators | Check signatures/bounds; validate simple declared constants where feasible. | Evaluate/check configured defaults and limits. | Run data-dependent validators; ordinary user functions are not magically compile-time executable. |
| Persistence schema | Generate schema/migration metadata. | Compare deployed storage and validate migration compatibility. | Enforce database constraints, revisions, and atomicity. |
| Authorization | Check policy function types and structural declarations. | Validate required policies and policy configuration. | Evaluate current actor, fields, state, query, subscription, and cached-outcome permissions. |
| Reactivity | Generate declared dependencies and typed observation bindings. | Resolve dependency graph and supported live-query forms. | Track reads/invalidate results, handle changing permissions, ordering, backpressure, and resume gaps. |
| Idempotency and transaction guarantees | Generate required arguments/bindings. | Check adapter contract/configuration. | Compare actual identities/input; handle concurrent commits, retries, and durable work. |

This matrix is a recommendation. Compile-time success does not eliminate startup validation, and neither can validate a future untrusted request. Conversely, the runtime should not repeatedly reparse or rediscover immutable declaration structure that registration has already normalized.

## Illustrative authoring choices

Every example is an **uncompiled proposal**. Names, ownership, async signatures, and exact macro syntax remain open. `Id`, `Status`, and `close_ticket` are illustrative application types/functions.

**A — recommended hybrid: fields in Rust, behavior through discoverable methods.**

```rust
#[derive(rom::Resource)]
#[resource(name = "ticket", version = 1)]
struct Ticket {
    #[resource(id)]
    id: Id,
    /// Short description visible in the ticket list.
    title: String,
    status: Status,
    assignee: Option<Id>,
}

let ticket = Ticket::definition()
    .crud()
    .policy(ticket_policy)
    .action("close", close_ticket)
    .live_reads();

app.register(ticket)?;
```

The derive supplies structural metadata, field handles, and typed bindings. `close_ticket` remains a normal function accepted by a documented handler trait; it returns transition intent through the shared pipeline. If action-specific typed client methods are desired, action registration must carry their signatures through a typed builder or an explicit action macro; a struct derive cannot discover unrelated functions. `.crud()` installs generic actions, rather than generating separate persistence logic for Ticket. Missing permissions deny access.

**B — manual fluent definition: useful escape hatch and a way to test the underlying API.**

```rust
let ticket = ResourceDefinition::new("ticket", 1)
    .field(Field::<Id>::new("id").identity())
    .field(Field::<String>::new("title"))
    .field(Field::<Status>::new("status"))
    .field(Field::<Option<Id>>::new("assignee"))
    .crud()
    .action("close", close_ticket)
    .policy(ticket_policy)
    .live_reads();
```

This illustrates explicit descriptor construction; binding a concrete `Ticket` struct also needs accessors/codecs through the manual `Resource` contract. It must not be advertised as reflection over a struct. Use it instead of a derive for exceptional/generated/dynamic definitions, not alongside a second hand-maintained declaration of the same fields.

**C — more declarative behavior: plausible only if its tooling beats A.**

```rust
#[derive(rom::Resource)]
#[resource(name = "ticket", version = 1)]
#[resource(crud, live_reads, policy = ticket_policy)]
#[resource(action(name = "close", handler = close_ticket))]
struct Ticket {
    #[resource(id)]
    id: Id,
    title: String,
    status: Status,
}
```

C centralizes more visible configuration but moves discovery into attribute documentation and gives the macro more grammar to own. A is the better default hypothesis because behavior can be composed and navigated through normal methods. Test both with human authors before fixing syntax. Do not offer two independent ways to configure the same rule without defined conflict handling.

## One source of truth across generated and manual code

Treat `Resource`/`FieldType`/action contracts plus their normalized descriptor as the semantic boundary. The derive emits implementations and calls the same builders available to manual authors. Generated endpoint, schema, client, and live-query metadata are projections. There is one runtime rule for patch semantics, policy denial, canonical encoding, and event production; generated helpers route into it.

Combining `#[derive(Resource, Serialize, Deserialize)]` with separate rename/skip/default attributes can create two authoritative schemas: the resource registry may describe a field that serialization omits, renames, or defaults differently. No macro can simply assume it understands every other derive or custom serializer.

**Recommended initial rule:** ROM's resource declaration owns its public field names, exposure, presence, and defaults. ROM adapters use generated request/response bindings and `FieldType` codecs consistent with that descriptor; they do not serialize the domain struct directly just because it implements Serde traits. Separate Serde derives may remain useful to application code, but do not redefine ROM's protocol. This keeps behavior explicit without parsing the whole Serde attribute language.

If direct Serde representation is later supported, make it an explicit codec mode with a small documented subset of supported representation attributes. Reject unsupported or contradictory visible settings with a field-level error; custom serializers require an explicit schema/codec contract and conformance checks. Do not silently ignore `rename`, `skip`, `flatten`, tagging, or default semantics and claim equivalence. A declaration of display schema alone is not proof that the serializer or validator follows it.

Registration should produce an immutable validated registry with useful provenance: resource, field/action, declared capability, and available source location/documentation. Fluent configuration should amend one descriptor with explicit conflict rules; silently overriding an identity, codec, or security setting is poor ergonomics even if it saves one line.

For a derived resource, the fluent builder composes behavior and configuration onto its generated fields; authors do not repeat its field list. Example B is an alternative manual implementation route, not an additional schema required alongside A. The normal path must never ask humans to keep a struct, a builder field list, and wire metadata synchronized by hand.

## Smallest useful tooling stack

These are candidates, not production dependencies or a tested version combination:

| Tool | Proposed role and reason |
| --- | --- |
| [`syn`](https://docs.rs/syn/latest/syn/) | Parse Rust items/attributes, preserve spans, and produce structured syntax errors. Enable only needed parsing features. |
| [`quote`](https://docs.rs/quote/latest/quote/) + [`proc-macro2`](https://docs.rs/proc-macro2/latest/proc_macro2/) | Emit structured tokens; keep expansion logic testable outside a proc-macro entry point. Reuse syntax tools rather than assembling Rust source strings. |
| [`darling`](https://docs.rs/darling/latest/darling/) | Optional attribute-to-configuration parsing. Add if the attribute vocabulary warrants it; direct `syn` parsing may be simpler initially. |
| [`proc-macro-crate`](https://docs.rs/proc-macro-crate/latest/proc_macro_crate/) | Resolve the consuming crate's renamed ROM dependency. Keep an explicit crate-path override for reexports/unusual layouts; its docs list resolution edge cases. |
| [`bon`](https://bon-rs.com/guide/overview) **or** [`typed-builder`](https://docs.rs/typed-builder/latest/typed_builder/derive.TypedBuilder.html) | Evaluate one for ordinary input/configuration builders. Do not depend on both or expose their internal generated state types in ROM's promised API. |
| [`trybuild`](https://docs.rs/trybuild/latest/trybuild/) | Compile-pass/compile-fail fixtures and expected diagnostic output for the macro's deliberate errors. |
| [`cargo-expand`](https://github.com/dtolnay/cargo-expand) | Developer inspection of expansions. Its text output is explicitly lossy, so it is not a correctness oracle or generated source to maintain. |

A small internal proc-macro crate can be reexported from the main ROM library. Application authors should not need to understand that packaging split to declare a resource. Manual traits and macro-generated implementations must remain compatible under ROM's release policy.

### Companion crate trials completed

The [executed comparison](authoring-crate-trials.md) records pinned current releases, compiler versions, negative controls and critiques. Bon and typed-builder both passed reusable-helper and presence-construction trials. A Syn/Quote/proc-macro2 derive passed aliases, nested/custom fields, source-local diagnostics, an explicit renamed-crate path, and manual-trait equivalence. Bon is provisionally preferred for typed action inputs based on these diagnostics; this is not final dependency adoption or a combined ROM authoring/runtime proof.

The persistent container now uses Rust/Cargo 1.99.0. Both trials passed independently after the upgrade; old-toolchain limitations did not determine the selection. Conceptual fit, tested compatibility, compile cost, diagnostic quality and production MSRV remain separate decisions. Additional tooling in the table remains researched rather than tested unless the trial report says otherwise.

## Diagnostics, documentation, and tests are part of the API

For malformed declarations, point at the relevant attribute/field, name the problem in resource terms, and give an actionable repair. Prefer a message such as `Ticket.title: unknown resource option 'max_lenght'; use 'max_length'` over a parser panic or generated generic-type failure. Accumulate independent local errors where possible. For type incompatibilities, emit small targeted checking expressions with field spans rather than letting one enormous builder instantiation fail far away. Diesel's backend-checking derive is a useful precedent for deliberate diagnostic code generation. [Selectable diagnostics](https://docs.rs/diesel/latest/diesel/prelude/derive.Selectable.html).

A fluent API should expose a short chain of meaningful methods, typed field references, and descriptive return/errors. Avoid exposing long typestate names and forcing type annotations to build reusable helpers. Document which methods are generated and how to navigate to the business function. Preserve docs on fields/actions in Rustdoc and generated metadata, with explicit controls for internal-only documentation. IDE completion/navigation quality is a hypothesis to verify in actual rust-analyzer sessions, not something this research established.

At startup, report `Ticket.assignee: adapter X lacks required index capability Y` with the registration location and alternatives. At request time, distinguish stable machine-readable error codes from safe human messages. A policy denial can identify a public action and permissible remedy without confirming a protected resource exists or exposing another user's data. Detailed policy traces belong behind authorized diagnostics.

Use `trybuild` for a focused set of authored errors and successful edge cases: renamed dependency, generic field, alias of `Option`, a different type named `Option`, newtype plugin, missing trait, invalid action signature, and conflicting attributes. Keep compiler-version-sensitive snapshots deliberate. Pair these with runtime equivalence tests showing manually described and derived resources normalize to the same behavior. Test action functions as ordinary Rust units, then use an in-memory conformance harness for full authorization/validation/reaction behavior. [trybuild workflow](https://docs.rs/trybuild/latest/trybuild/).

Before accepting syntax, have a human add a resource, discover CRUD, add a custom action, restrict a field, subscribe to a derived read, and repair a deliberate mistake using only the normal docs and compiler output. Record points of confusion, generated API navigation, test setup, and clean/incremental build cost for growing resource counts. No timing targets or usability result are claimed here. Optimize time to understand and safely change the backend, not declaration line count.
