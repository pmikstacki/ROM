---
name: rom-studio
description: Use when changing ROM Studio controls, Resource presentation, plugin settings, or browser interactions.
---

# ROM Studio development

Use this project skill for the `0.1.0` source candidate. It is not an assembled release-bundle workflow.
Read [release support](../../../docs/release-support.md) for candidate limits and accepted artifacts.

## Select the owner

| Change | Owner |
| --- | --- |
| Shared controls, styles, or generic UI helpers | The installed `rom-ui` package and its explicitly selected source checkout |
| Descriptor validation, semantic editors, reference lookup, or durable recovery | ROM Studio |
| Domain layout, prompts, persistence intent, or application policy | The consuming application |

Read [public compositions](../../../docs/studio-compositions.md) before selecting an import.
Read [Studio exports](../../../studio/package.json) for supported package paths and the installed `rom-ui` identity.
The upstream source is [rom-ui](https://github.com/pmikstacki/rom-ui). Select its matching revision before editing shared behavior.
Studio facades preserve existing public paths. Update generic controls in their owner, then verify the installed integration.
New upstream gallery exports require installed-package evidence before use in Studio.

## Implement the change

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
- [Standard semantic field contracts](../../../docs/studio-fields.md): exact representations and control limits.
- [Backend semantic codecs](../../../crates/rom-fields/src/lib.rs): validated dates, colors, text formats, decimals, and units.
- [Shared semantic vectors](../../../crates/rom-fields/tests/fixtures/semantic-codecs-v1.json): Rust/browser normalization agreement.
- [Value editor](../../../studio/src/lib/renderers/ValueEditor.svelte): candidate values and nested wrappers.
- [Application controller](../../../studio/src/lib/application/controller.ts): sessions, revisions, queries, and durable receipt recovery.
- [Studio author example](../../../docs/research/rom-0.0.2-studio-author-workflow.md): independent consumer and explicit custom renderer.

## Decision rules

Use semantic descriptors for meaning and validation. Use presentation metadata for labels, layouts, and control preferences.
A title selector can read only disclosed fields. A settings classification does not grant write access or configure a host secret.
A codec name alone cannot establish a safe editable representation. Unknown semantics remain read-only.
Custom code must be bundled explicitly; discovery is metadata, not permission to execute remote JavaScript.
Use supported installed `rom-ui` exports before adding dependencies or copying shared controls.
Keep simple labeled controls compact on desktop and mobile. Keep larger editors behind an explicit expansion control.

## Verification

Use the package manager declared by the affected package. Studio declares `pnpm@10.30.0`.
From `studio`, run `corepack pnpm run check`, `corepack pnpm run test:unit`, and `corepack pnpm run build:components`.
Use `corepack pnpm install --frozen-lockfile` when installation is necessary.
Then run the affected Playwright specs with `tests/components/playwright.config.ts`.
Use the repository's existing browser executable configuration; do not report WebKit results from a Chromium-only command.
For a structural change, run `./scripts/check` with the declared Rust toolchain, then the complete affected Studio suite.
For shared package changes, run the owner's verifier and an independent consumer of the exact package archive.
Then verify the affected Studio facade and host integration. Record archive and lockfile identities.
These checks do not replace packaged release acceptance or a human usability review.

## Presentation increment

Read [presentation authoring](../../../docs/studio-presentation.md) before adding labels or Settings classification.
Check [the shared wire fixture](../../../studio/tests/fixtures/presentation-discovery.json) when changing discovery.
Use [Settings composition](../../../studio/src/lib/application/SettingsPage.svelte) as the navigation reference, not as a separate write API.

## Editor state and authorization

For sessions and durable saves, read the [application skill](../rom-application/SKILL.md).

Keep invalid drafts separate from wire values. Use the optional renderer draft callback when a custom editor must survive a list remount.
Use own-property lookup for map keys. Prototype-shaped keys must not access inherited editor state.
Reference choices must use the session-owned lookup contract. Keep exact IDs when labels are unavailable.
Do not describe candidate-page filtering as a full-text query. Do not retain disclosed labels after session or permission changes.
A Blob reference does not perform upload or attachment completion. Use the existing attachment lifecycle for those operations.
Test both actual browser engines. A routed binary-body limitation does not prove that a real upload failed.
