# ROM 0.0.3 descriptor contract review

Review date: 2026-10-05. Baseline: `091836e770752153a54e163191cf0fee5f0ab345` (`HEAD`). Scope: the working presentation, discovery, client, fixture, and Studio skill changes.

This is an independent source review. No test, compiler, or full verifier was executed for this review. Existing test code is evidence of intended checks, not evidence that those checks passed.

## Findings

### P3: presentation override error retention needs an explicit contract

Location: [Definition::presentation](../../crates/rom/src/resource/definition.rs#L114), lines 114–118. Initial trait metadata is also validated at lines 84–89.

An invalid `Resource::presentation()` sets `metadata_error` during `Definition::new()`. A later valid `.presentation(...)` replaces the metadata but leaves that error stored. The same behavior occurs with `.presentation(invalid).presentation(valid)`. Registration then rejects the final valid definition because [the builder consumes the stored error](../../crates/rom/src/execution/builder.rs#L116).

The backend implementer identified this as intentional error retention, consistent with other validated declaration setters. It is not a confirmed registration correctness defect. The public setter documentation does not explain that a valid replacement cannot correct invalid earlier presentation. State that behavior explicitly. If final-definition semantics are later selected, preserve codec and action-input errors independently.

Needed regression: define a Resource with invalid trait presentation. Apply valid metadata through the definition override. Assert the intended registration rejection. Add a second case where invalid codec metadata still prevents registration after a valid presentation override.

### P2: the discovery wire addition also changes public Rust struct literals

Location: [DiscoveredResource](../../crates/rom/src/discovery.rs#L29), line 29.

The new optional field is additive on the wire, but Rust callers must supply it in `DiscoveredResource` literals. Existing exhaustive destructuring also needs adjustment. `Option` and `skip_serializing_if` do not preserve Rust source compatibility. The canonical `Descriptor` and `FieldDescriptor` literals remain unchanged.

Record this source migration before release. Do not describe the change as unconditionally source-compatible because discovery remains version 1. An API constructor can reduce future literal dependence, but adding a constructor does not repair existing literals.

Needed regression: compile an external consumer that constructs or destructures the public discovery output type. Record the expected migration or compatibility boundary. Keep the existing canonical descriptor consumer check separate from this output-type check.

## Contract observations

| Boundary | Source observation | Limit |
| --- | --- | --- |
| Semantic authority | Presentation is stored separately from canonical `Descriptor`, codecs, and policies. | A settings classification grants no operation permission. |
| Authoring validation | Rust rejects unknown fields, absent title targets, non-string titles, and custom-codec title selectors. | Metadata is trusted author configuration, not executable client instructions. |
| Text and collection bounds | Rust bounds labels/selectors at 256 UTF-8 bytes, help at 2048 bytes, and field/group collections at 1024. | Resource registration must reject invalid authored metadata. |
| Authorized projection | Discovery projects hints through the fields already disclosed by discovery policy and visible-reference checks. | Resource labels and settings classification remain Resource-level metadata. |
| Hidden titles | A hidden field removes its title selector. The client title helper reads only a disclosed field present in the projected row. | A discovered field does not grant row disclosure or mutation rights. |
| Hidden groups | Projection retains groups only when an authorized field hint refers to them. | Shared visible groups can still expose their trusted group label. |
| Discovery bytes | The borrowed projected metadata is charged before strings are cloned. The additional comma, key, and colon cost 16 bytes. | Collection indexes allocate before charging, but authoring bounds them. |
| Client compatibility | `presentation` is optional under discovery version 1. A real legacy descriptor can pass the unchanged branch. | Unknown presentation properties are rejected; future hint expansion needs a compatibility decision. |
| Client validation | TypeScript bounds UTF-8 text, validates known fields/groups, and rejects custom-codec/non-string title targets. | Client validation does not authorize server operations. |

Sources: [presentation model and validation](../../crates/rom/src/resource/presentation.rs), [authorized discovery](../../crates/rom/src/discovery.rs), [Resource trait](../../crates/rom/src/resource/schema.rs), [client discovery validator](../../studio/src/lib/client/discovery.ts), [client types](../../studio/src/lib/client/types.ts), and [human presentation helper](../../studio/src/lib/presentation/resource-presentation.ts).

No hidden title-value or hidden-only field-group disclosure defect was found in these paths. Metadata labels are trusted static text. They are not a channel for fetching undisclosed fields or executing arbitrary JavaScript.

## Fixture and negative-case review

The [shared discovery fixture](../../studio/tests/fixtures/presentation-discovery.json) is consumed by Rust and TypeScript tests. The [Rust integration test](../../tests/persistence/tests/presentation_discovery.rs) compares actual discovered output with that fixture. It checks hidden field labels/groups, a hidden title selector, exact response-byte acceptance, and rejection one byte below that limit.

The [Rust model tests](../../crates/rom/src/resource/presentation_tests.rs) cover non-string/custom-codec titles, optional/nullable string titles, unknown fields/groups, duplicate groups, UTF-8 boundaries, empty settings classification, and unknown executable properties. The [TypeScript tests](../../studio/tests/unit/presentation-discovery.test.ts) cover matching negative cases and consume the same positive fixture.

The TypeScript test named “legacy descriptors omit presentation” currently supplies an empty resource list. It does not exercise a populated legacy descriptor. Replace that input with a real descriptor without presentation and assert its parsed shape. This is a coverage gap; source inspection shows the omission branch exists.

Add title-wrapper and help-boundary parity cases when extending the shared fixture. An arbitrary list of new tests is unnecessary. Regressions should cover the compatibility decisions above and actual defects found during verification.

## Studio skill and scope

All ten local Markdown links in [the project Studio skill](../../skills/project/rom-studio/SKILL.md) resolve to existing files. Its contract map links the Resource facade, authorized discovery, client types/validator, renderer registry, editor, controller, and independent author example.

The facade provides access to the new presentation module. Direct links to presentation authoring, the shared fixture, and Settings composition would improve discovery of this increment. Their absence does not create a permission grant or break the linked source map.

The skill requires explicit bundled custom code. It rejects remote JavaScript inferred from discovery. It keeps settings classification separate from host-secret configuration and operation permissions. The [AI development map](../ai-development.md) also distinguishes project guidance from executable release-bundle skills.

Control-format expansion remains deferred. Current field metadata accepts `label`, `help`, and `group`; it does not implement the researched date, color, decimal, or JSON control-format families. This review does not establish 0.0.3 release completion, packaged-consumer acceptance, or human usability approval.

## Follow-up disposition

The legacy test now parses a populated Resource descriptor without presentation.
It asserts preserved fields, unchanged discovery version, and absent presentation metadata.
The project skill now links presentation authoring, the shared fixture, and Settings composition.
