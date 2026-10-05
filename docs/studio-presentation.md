# Studio presentation metadata

ROM separates presentation from Resource identity and field codecs.
Presentation provides readable labels, an optional title selector, field help, declared groups, and Settings navigation.
It does not change persisted values, grant permissions, or load executable frontend code.

## Declare presentation in Rust

Attach presentation to the Resource definition before runtime registration.
A native plugin uses the same definition builder as application code.

```rust,ignore
use rom::{ResourcePresentation, SettingsPresentation};

let definition = StudioSettings::definition()
    .presentation(ResourcePresentation {
        label: Some("Studio settings".into()),
        settings: Some(SettingsPresentation {
            group: "studio".into(),
            label: "Studio".into(),
        }),
        ..Default::default()
    });
```

`StudioSettings` represents the application's declared Resource type.
Keep its existing authorization and discovery policies on this definition.
See the [maintained demo registration](../demo/src/studio_application.rs) for the complete example.
Manual Resource implementations can also implement `Resource::presentation()`.
The default returns no presentation metadata.

Use `fields` to map canonical field names to `FieldPresentation` labels, help, and optional group names.
Declare referenced groups in `groups`.
Use `title_field` for a declared built-in string field, optionally wrapped in `Optional` or `Nullable`.
Custom codecs cannot currently supply a title selector.

Registration rejects unknown fields, invalid title shapes, unknown groups, duplicate groups, and oversized text.
Labels and selectors are limited to 256 UTF-8 bytes. Help is limited to 2048 bytes.
Each field metadata collection and group collection is limited to 1024 entries.
A rejected builder declaration remains rejected after another `.presentation(...)` call.
Construct a fresh valid definition instead of attempting to repair a rejected builder.

## Use the authorized discovery result

Discovery returns presentation only for disclosed Resources.
It removes title selectors and field metadata for withheld fields.
It removes groups without disclosed members.
Encoded presentation bytes count against the discovery response limit before cloning.
The Studio client validates the received shape and field references before use.

Studio shows the authorized title value when it is a nonempty string.
Otherwise, it shows a readable fallback without parsing the Resource ID.
The exact ID remains secondary, selectable, and copyable.
Field controls use the human label for accessible names while keeping canonical names for wire values and errors.

Settings groups come from `presentation.settings` on disclosed Resources.
Settings uses the same Resource queries, forms, actions, revisions, and receipt recovery as other Resource views.
A plugin adds a group through its registered Resource definition.
A group is navigation metadata. It does not add permission to edit configuration or secrets.
Changing the selected Settings Resource currently replaces its local form draft.
Closing its inspector or resizing the viewport preserves the mounted draft.

## Compatibility and current limits

Presentation is optional in discovery protocol version 1.
Existing Resources without presentation keep their original behavior.
This change does not modify persisted `Descriptor` or `FieldDescriptor` identity.
Rust code that constructs `DiscoveredResource` with a struct literal must now provide its optional `presentation` field.

Declared field groups are validated and projected, but the current form does not arrange controls into those groups.
Semantic formats such as dates, colors, decimals, and relation pickers need separate value contracts and editor conformance.
Presentation cannot make an unknown custom codec safe to edit.
Custom renderers still require explicit frontend bundle composition.

See [the 0.0.3 research](research/rom-0.0.3-studio-release-research.md) for the remaining control work.
See [AI development guidance](ai-development.md) for the source map and verification commands.

Studio declares its own favicon under `/rom-studio/rom-icon.svg`.
Without an explicit icon, browsers can use the shared host's `/favicon.ico`, which belongs outside Studio.
The current favicon is a neutral letter R. It is not a finalized branding system.
