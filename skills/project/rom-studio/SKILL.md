---
name: rom-studio
description: Develop ROM Studio controls, Resource presentation, plugin settings, and browser workflows against the generic backend contract.
---

# ROM Studio development

Use this project skill for Studio changes. It is not an assembled release-bundle workflow.

1. Read [quality gates](../../../docs/quality.md) and [writing rules](../../../docs/writing.md).
2. Read the affected descriptor, codec, client validator, renderer, and application component before selecting an editor.
3. Define the wire value and field-state cases. Preserve exact integers, absence, null, removal, and unknown outcomes.
4. Add a failing regression at the narrowest useful boundary. Use browser tests for focus, drawers, drag behavior, and layout.
5. Implement through shared descriptors and controls. Use explicit plugin composition; do not add a controller for each Resource kind.
6. Run the affected unit and browser checks. Run the full local verifier before integration after a structural refactor.
7. Record command, source identity, result, and remaining limits. Keep mockups separate from implemented screenshots.

## Contract map

- [Rust Resource model](../../../crates/rom/src/resource.rs): declaration and canonical value contracts.
- [Authorized discovery](../../../crates/rom/src/discovery.rs): bounded descriptor projection.
- [Frontend wire types](../../../studio/src/lib/client/types.ts) and [discovery validation](../../../studio/src/lib/client/discovery.ts): accepted client contract.
- [Renderer registry](../../../studio/src/lib/renderers/registry.ts): codec identity and component composition.
- [Value editor](../../../studio/src/lib/renderers/ValueEditor.svelte): candidate values and nested wrappers.
- [Application controller](../../../studio/src/lib/application/controller.ts): sessions, revisions, queries, and durable receipt recovery.
- [Studio author example](../../../docs/research/rom-0.0.2-studio-author-workflow.md): independent consumer and explicit custom renderer.

## Decision rules

Use semantic descriptors for meaning and validation. Use presentation metadata for labels, layouts, and control preferences.
A title selector can read only disclosed fields. A settings classification does not grant write access or configure a host secret.
A codec name alone cannot establish a safe editable representation. Unknown semantics remain read-only.
Custom code must be bundled explicitly; discovery is metadata, not permission to execute remote JavaScript.
Use Svelte/shadcn-svelte primitives already in the repository before adding dependencies.
Keep simple labeled controls compact on desktop and mobile. Keep larger editors behind an explicit expansion control.

## Verification

From `studio`, run `npm run check`, `npm run test:unit`, and `npm run build:components`.
Then run the affected Playwright specs with `tests/components/playwright.config.ts`.
Use the repository's existing browser executable configuration; do not report WebKit results from a Chromium-only command.
For a structural change, run `./scripts/check` with the declared Rust toolchain, then the complete affected Studio suite.
These checks do not replace packaged release acceptance or a human usability review.

## Presentation increment

Read [presentation authoring](../../../docs/studio-presentation.md) before adding labels or Settings classification.
Check [the shared wire fixture](../../../studio/tests/fixtures/presentation-discovery.json) when changing discovery.
Use [Settings composition](../../../studio/src/lib/application/SettingsPage.svelte) as the navigation reference, not as a separate write API.
