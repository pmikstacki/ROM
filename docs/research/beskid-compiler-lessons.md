# Beskid compiler lessons for ROM

Reviewed 2026-10-02. This is a focused source review and implementer judgement, not a reproduced Beskid build or a universal architecture ranking.

## Verdict and evidence boundaries

Retain ROM's selected architecture: one Resource defines generic persistence, CRUD, custom actions, endpoints and reactive behavior; derive plus fluent Rust composition feeds a shared runtime; Tokio and Rayon execute its work. Beskid's strongest transferable lesson is that convenient syntax needs a precise semantic authority behind it. Sophisticated internals are justified when they make ordinary authoring reliable and understandable.

**Recommended additions:** explicit registration invariants, diagnostic provenance across generated and runtime paths, a classified unsupported-capability inventory, descriptor/codec parity tests, and evidence records tied to the exact executed checkout. These strengthen existing decisions without introducing another domain entity or replacing Rust authoring with a compiler DSL.

Public sources were retrieved at compiler commit [44a07aed5a13d41d57853445167c0f091cd10a7e](https://github.com/Cyber-Nomad-Collective/beskid_compiler/tree/44a07aed5a13d41d57853445167c0f091cd10a7e) and root commit [abfe7d6bf1628db1be75245693b699394eb1a23c](https://github.com/Cyber-Nomad-Collective/beskid/tree/abfe7d6bf1628db1be75245693b699394eb1a23c), matching the [quality baseline](beskid-quality-baseline.md). Beskid source and tests were read, not executed. An assertion described below establishes intended coverage, not a passing run.

The [ROM architecture](../../openspec/changes/establish-rom/design.md), [authoring research](rust-resource-authoring.md), [quality baseline](beskid-quality-baseline.md) and [prototype results](prototype-results.md) supply the local design context. Current disposable derive and builder trial READMEs were also read: they report successful Rust/Cargo 1.99.0 checks, source-local derive diagnostics, manual-trait compatibility and builder negative controls. Those reports were not independently rerun for this note. Their absence of a production codec and distinction between Rust optional construction and JSON PATCH semantics remain material limits.

No original implementer was contacted. Desktop thread-reading tools were unavailable to this delegated reviewer. The coordinating reviewer read the selected completed OpenSpec companion discussion and supplied a bounded factual summary: it reported source-backed semantic-legality/diagnostic slices, left later corelib/runtime verification open, and encountered an npm DNS failure during specification validation. These are reported results, not reproduced checks. Public source and the public task ledger independently support the architectural lessons below; other unreviewed discussions are not used. No private transcript, implementation or host information is reproduced here.

## 1. One semantic authority for every entry point

**Observed:** Beskid's `lower_syntax_program` calls `check_items` before resolving module items and again for newly discovered items. The gate gathers structured findings rather than reconstructing user errors from generic failure strings. Its scope is the requested items and discovered dependencies. [Orchestration](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_codegen/src/module_emission/orchestration.rs#L68), [legality gate](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_queries/src/semantic_contract/legality.rs).

**ROM recommendation:** derive and manual traits produce the same structural description; fluent methods attach behavior; one registration path validates the composed result. HTTP, in-process calls and reactions consume that accepted contract. Generated per-kind handlers must not implement independent field rules or mutation policies. Dependencies and capabilities discovered during composition must be checked before registration completes.

An internal `ValidatedRegistry` can restrict construction, but its name is not proof: native extensions remain trusted and per-action policy/state checks remain necessary. Do not copy compiler reachability literally: validate all registered, enabled declarations at startup; merely having a Rust type available does not register it.

**Verify:** equivalent malformed derived/manual descriptions fail with equivalent codes. The same rejected mutation through HTTP, Rust and reactions preserves the shared state/event/outcome contract.

## 2. Explicit phase invariants

**Observed:** Beskid binds syntax inputs and node keys to a generation. Its typed-program representation carries generation and authority information, while legality and backend acceptance remain separate gates. A typed representation is not a universal certificate of executable correctness. [Generation-bound inputs](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_queries/src/semantic_contract/model/typed_program.rs), [semantic preparation](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_analysis/src/services/semantic_facts.rs).

**ROM recommendation:** preserve identity and provenance through transformations, with narrow guarantees at each stage:

| Stage | Establish here | Must remain later |
| --- | --- | --- |
| Derive / Rust checking | Attribute syntax, duplicate local wire names, field trait bounds, typed signatures | Actual values, credentials, database state |
| Registration | Unique kind identity, codec/capability compatibility, complete bindings | Per-request authorization and revision checks |
| Action preparation | Presence/null/value decoding, trusted actor, permitted operation, valid input | State-dependent checks against committed revision |
| Transaction | Revision checks and atomic state/event/outcome persistence | External effects and observer delivery |
| Delivery / reaction | Projection, freshness and replay rules for that consumer | No blanket authority inherited from an old request |

Generated validators still execute for future values. Compiling them does not validate future inputs or establish a speedup. Ordinary authors should not need phase tokens or internal execution machinery.

**Verify:** accepted declarations still reject invalid values and stale revisions; cached schema identifiers or prepared requests cannot bypass current policy or compatibility checks.

## 3. Diagnostics retain cause and location

**Observed:** `SemanticDiagnostic` retains code, source, span, help, severity and extension origin; its wrapper preserves structured downcasting. Codegen tests inspect source excerpts and distinguish legality findings from internal missing-rule errors. [Diagnostic representation](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_analysis/src/analysis/diagnostics.rs), [source-excerpt tests](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_codegen/tests/isle_adapter/diagnostics_fail_closed.rs#L426).

**ROM recommendation:** retain resource, field/action, stable code, declaration location where available, and runtime value path. Duplicate mappings should identify both declarations. Unsupported fields should point at the field, not only `#[derive(Resource)]`. Separate author errors, expected request rejection, missing adapter capabilities and internal invariant failures. Do not describe a missing framework implementation as invalid user input.

Compiler source excerpts are not a template for copying request payloads: payloads may contain secrets. Public messages need safe corrective guidance; protected values, internal paths and detailed policy traces stay out of client errors and durable events.

**Verify:** stable code, responsible span/path and actionable help, including renamed and nested custom fields; protected values absent from client-visible failures. The derive trial's reported span regression reinforces this approach, without proving IDE usability.

## 4. Unsupported cases fail explicitly

**Observed:** Beskid tests require an unsupported construct after a clean legality gate to fail as internal E2102 at its site, without a user-error code. Unavailable semantic facts deliberately reach a later error boundary: a clean gate does not license invented fallback behavior. [Gate contract](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_queries/src/semantic_contract/legality.rs#L199), [missing-rule regression](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_codegen/tests/isle_adapter/diagnostics_fail_closed.rs#L426).

**ROM recommendation:** enumerate field/query/codec capabilities and reject unsupported combinations before serving them where possible. Ignore neither unknown attributes nor contradictory configuration. An adapter lacking exact decimal comparison or a query operator must reject that capability rather than approximate it. An advertised capability that fails its contract is distinct from explicitly unsupported functionality.

**Verify:** negative examples for capability boundaries and a deliberately inaccurate plugin; failure produces no successful mutation/publication. An unsupported-feature inventory should connect restrictions to tests and remove stale entries as support changes.

## 5. Metadata agrees with executable behavior

**Observed:** Beskid compares runtime symbol/signature maps with manifest provenance. Generator tests permute collections and expect unchanged artifacts; parser tests reject unknown, duplicate and contradictory fields. The export scanner assumes a constrained line-oriented source format, so this is not arbitrary language introspection. [Source/manifest parity](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_manifest/tests/abi_v5_source_authority/runtime_exports.rs), [determinism/rejection tests](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_manifest/tests/abi_v5_source_authority/parser_and_determinism.rs).

**ROM recommendation:** one contract governs wire names, optionality, defaults, validation and exported schema. Independent Serde `rename`, `skip`, `flatten` and default metadata can otherwise create a second authority. Keep the modest current rule: ROM adapters use codecs generated from the resource contract; independent Serde derives do not redefine ROM's protocol. A future direct-Serde mode needs a supported subset and rejects conflicting or unsupported representation settings.

Do not infer Rust semantics with a source scanner. Emit field-trait calls and let rustc resolve aliases, nested types and custom fields. Fluent configuration adds behavior to derived descriptors without repeating their field lists. Manual implementations need the same conformance suite because trusted Rust can return inaccurate metadata.

**Verify:** normalized derived/manual descriptors, then actual encoding/decoding/validation against them. Include renamed fields, omission/null, defaults, custom codecs and Serde conflicts. Permute registration order only where semantically irrelevant; canonical sorting must not reorder an action pipeline. Descriptor equality alone does not prove codec parity.

## 6. Cohesive Rust boundaries hide necessary complexity

**Observed:** Beskid's layout module groups aggregate, enum and field-access implementations and separates public specializations from narrower internal exports. This demonstrates a visibility boundary, not an ideal module count or function length. [Layout exports](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_queries/src/semantic_contract/layouts/mod.rs).

**ROM recommendation:** keep field contracts, descriptor composition and runtime execution reusable through ordinary Rust. The macro parses/emits implementations; the core owns semantics; adapters translate mechanisms. Internal modules can precede public crates. A small public API may sit over complex typed helpers: application authors should not spell builder state types simply to simplify ROM's implementation.

**Verify:** downstream derive and manual-only applications, a custom field using public contracts, and a human authoring walkthrough. Existing trials support feasibility, not final usability or production dependency selection.

## 7. Negative tests fail for the intended reason

**Observed:** Beskid's legality corpus requires named diagnostic coverage, checks the dependency unit owning the error, and rejects internal-error substitutes. It copies source inputs without generated `obj/` contents. The harness invokes shared preparation; it does not independently execute every CLI path. [Compile-fail corpus](https://github.com/Cyber-Nomad-Collective/beskid_compiler/blob/44a07aed5a13d41d57853445167c0f091cd10a7e/crates/beskid_tests_projects/src/spine/legality_compile_fail.rs).

**ROM recommendation:** compile-fail fixtures assert intended error and location, not merely nonzero Cargo exit. Cover renamed dependencies, aliases, unsupported fields, action signatures and codec conflicts. Pair failures with compile-pass counterparts and runtime controls. An isolated downstream smoke test catches assumptions hidden by workspace feature unification.

**Verify:** remove each fixture's intended error and ensure its expected-failure assertion stops passing. Separate compiler-sensitive wording from stable ROM contextual assertions. Preserve the builder trials' distinction between compile-time protection and runtime rejection.

## 8. Completion claims identify their evidence

**Observed:** the public task ledger records implemented slices with source/test references, while strict OpenSpec validation and several later full-suite checks remain unchecked. Earlier suite reports qualify environmental and baseline failures. This is recorded evidence, not execution by this reviewer. [Task ledger](https://github.com/Cyber-Nomad-Collective/beskid/blob/abfe7d6bf1628db1be75245693b699394eb1a23c/openspec/changes/add-reachability-scoped-semantic-legality-gate/tasks.md).

**ROM recommendation:** distinguish implemented, focused tests passed, full suite passed and spec validated. Record root/nested commits, relevant dirty changes, lockfile, tool versions, command and result. An offline dependency failure blocks validation; it neither passes validation nor proves the spec invalid. A later success identifies the exact checkout and command it supersedes.

**Verify:** completed tasks link behavior to tests or explicit evidence; fresh-binary claims require a rebuild from recorded source. Relevant Rust checks belong in the local verification scripts and NixOS release workflow; GitHub remains code storage with Actions disabled.

## Concrete effect on ROM

**Add before stabilizing the first library slice:** a written registration invariant; distinct error categories with provenance; descriptor/codec conformance cases; and a compact supported-capability matrix with negative tests. These close gaps between a convenient declaration and actual generated behavior.

**Reinforce:** one resource authority, ordinary Rust traits, derive/fluent/manual convergence, shared action semantics, trusted native plugins, explicit missing/null handling and bounded evidence claims. Tokio/Rayon is unaffected. Compiler query infrastructure does not establish durable events or live-query correctness.

**Do not import wholesale:** Beskid's IR, Salsa graph, ABI manifests, diagnostic catalog or source-extraction conventions. ROM needs the applicable invariants with simpler author-facing concepts. This review establishes neither Beskid's full test health nor a measured usability advantage for ROM.
