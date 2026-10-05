# ROM 0.0.3 compatibility review

Status: local source, Studio artifacts, and permanent preview accepted.
The [completion record](rom-0.0.3-release-completion.md) identifies exact source and limits.

Use one matching Cargo and Studio source set. Recompile trusted extensions.
Native storage format 8, archive format 6, and query protocol version 1 remain unchanged.
Presentation and enum labels do not alter persisted field descriptors or enum wire members.

## Rust source changes

The Resource derive accepts bounded labels, title selectors, field hints, groups, and Settings navigation.
Manual implementations retain their default empty presentation.
Registration checks concrete field shapes and canonical names after macro expansion.

Field implementations can return advisory labels through `Field::enum_labels()`.
Resource implementations can return named bindings through `Resource::field_enum_labels()`.
Existing implementations use empty defaults.

Manual literals for `DiscoveredField`, `InputFieldDescriptor`, and `InputDescriptor::Scalar` must supply the new `enum_labels` member.
Use an empty label map when none is declared. Update exhaustive patterns or include `..`.
Labels must name exact enum members. Duplicate labels do not merge distinct members.
The optional discovery projection remains subject to current authority and byte limits.

The new `rom-fields` crate adds ten trusted version-one codec identities.
Changing an existing application's field codec is a schema change, not an automatic presentation upgrade.
Preserve historical request codecs when migrating persisted Resource definitions.

## Studio source changes

Custom renderers still register their exact codec name and version in a trusted application entry point.
The optional third registration argument selects `inline` or `expanded` layout.
Existing registrations default to expanded layout. Collection editing remains expanded.
Unregister handles affect only the registration that created them.

Optional `draft` and `onDraftChange` renderer props retain local invalid input across framework remounts.
They do not enter the mutation payload. Existing renderer components need no bindable prop.
Unknown codecs preserve existing values and block unsafe edits.
The standard Studio bundle does not implicitly load the demo renderer.

## Deliberate limits

Decimal values remain exact strings. Generic ordering is lexical, not numeric arithmetic.
JSON documents retain exact source text, including numeric tokens and duplicate keys.
The JSON control is a source editor, not a structural tree editor.
Reference search filters at most twenty authorized candidates rather than searching an unbounded dataset.
Drag-and-drop has ordinary move controls as an alternative.
Work inspection shows authoritative snapshots; it does not invent a timeline without a history source.

See [field contracts](../studio-fields.md) and [presentation authoring](../studio-presentation.md).
The completed acceptance binds native/browser evidence and artifacts to the exact source revision.
