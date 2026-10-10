# Reference composition evidence

Date: 2026-10-07. Scope: Task 3B extraction and source-authoring verification. Installed and original-consumer acceptance remain open.

`ReferencePicker` accepts explicit lookup and disclosure identity props. The existing `ReferenceField` supplies its private context through a compatibility wrapper. One implementation owns editing, candidate admission, debounce, cancellation and selection. Existing renderer import paths and the context symbol remain stable.

## Public contract

The coordinator exports the component through `rom-studio/ui/components`. The independent form imports that public entry. It does not install private application or renderer context.

| Prop | Contract |
| --- | --- |
| `kind`, `value`, `label`, `path` | Existing reference field identity, wire value and canonical validation path. `path` defaults to `label`. |
| `onchange`, `onerror` | Existing value and validation callbacks. The exact string ID is retained. |
| `readonly`, `showLabel` | Existing editing and presentation controls. Defaults are false and true. |
| `lookup` | Optional `ReferenceLookup`. Missing lookup leaves manual exact ID editing available. |
| `authorityToken` | Required disclosure invalidation identity. The host changes it when owner, session or disclosure authority changes. |
| `messages` | Optional partial `ReferencePickerMessages` strings. Defaults preserve existing Studio labels. |

Lookup contracts and bounds live in `ui/components/reference-lookup.ts`. `renderers/reference-lookup.ts` reexports them. Component messages and bounded response admission live in the named `reference-picker.ts` helper.

`lookup.descriptor(kind)` must synchronously return the stable current disclosed descriptor or undefined. The host must make its descriptor changes reactive. Lookup receives an AbortSignal. Returning late after abort does not permit publication.

The token is not authentication, authorization or a permission grant. The host must enforce current backend authority. A label never replaces the exact stored ID. The compatibility wrapper uses current descriptor identity, preserving the existing session-owned renderer contract.

## Bounds and ownership

Candidate admission accepts at most 20 unique exact IDs. It validates candidate strings and copies their whitelisted fields. Lossless wire serialization bounds the admitted result to 65536 UTF-8 bytes. Search admits at most 1024 UTF-8 bytes. Nonempty search uses the existing 250ms debounce. Each opened flow permits eight lookup attempts.

Scope changes clear candidate results, selected titles and search text. They close the picker and abort pending work. Scope includes token, lookup identity, descriptor identity, kind and readonly state. Completion checks those identities before admission and publication. Selection checks identity after the host value callback to avoid assigning an old title to a new owner.

Denied, failed and malformed lookup responses leave exact ID editing available. Existing canonical normalization and manual input limits remain unchanged. The component does not add a full-text backend query.

## Executed evidence

Historical failures remain in `/var/tmp`:

- `/var/tmp/rom-010-reference-public-red.log`: actual component build failed because the public entry did not export `ReferencePicker`.
- `/var/tmp/rom-010-reference-unit-red.log`: focused unit failed because the bounded admission module did not exist.
- `/var/tmp/rom-010-reference-check-first.log`: first check found a nullable captured lookup. The implementation corrected the guard.
- `/var/tmp/rom-010-reference-browser-first.log`: 26 passed and six failed. The failures were ambiguous test status locators, because output elements also expose status roles. They were not product regressions.
- `/var/tmp/rom-010-reference-format.log`: formatter lacked the Svelte plugin. The corrected formatter command used `--plugin prettier-plugin-svelte`.

The corrected focused browser run passed 32 cases. It covered five public form cases and eleven existing renderer cases in each engine. `/var/tmp/rom-010-reference-browser-corrected.log` preserves the result.

The final source adds public cases for callback rebind and pending label/readonly changes. Its affected run passed 117 cases and skipped one existing WebKit touch-reorder case. The seven public reference cases passed in both engines. The run includes reference, direct form, semantic field, renderer regression and sortable field specs. `/var/tmp/rom-010-reference-browser-final.log` preserves the result. The owned source manifests matched before and after execution.

`/var/tmp/rom-010-reference-focused-unit-final.log` records eleven passing tests: two new admission/search cases and nine existing application reference cases. `/var/tmp/rom-010-reference-all-units.log` records 399 passing Studio tests. The final component build passed. The final Svelte check reported zero errors and warnings. Commands ran from `/root/ROM/studio`:

```sh
npm run build:components
npm run check
npm run test:unit
node --experimental-strip-types --test tests/unit/ui-reference-picker.test.ts tests/unit/reference-picker.test.ts
ROM_WEBKIT_EXECUTABLE=/var/tmp/rom-studio-webkit-2359/pw_run.sh npx --no-install playwright test --config tests/components/playwright.config.ts reference-composition.spec.ts reference-picker.spec.ts semantic-fields.spec.ts renderer-regressions.spec.ts direct-form.spec.ts sortable-fields.spec.ts --workers=1
```

The browser configuration uses Chromium at `/root/.nix-profile/bin/chromium` and the locked WebKit 2359 launcher. Tests run with one worker. Source manifests and preserved source are in `/var/tmp/rom-010-reference-evidence/`.

## Limits

The browser form is an independent composition compiled from repository source. It is not an installed package consumer. The verifier and installed public-consumer retest remain coordinator-owned. No native build or Cargo change was made in this task.

The suite does not establish screen-reader usability, original-consumer acknowledgement or AP reference-group closure. Full local verification is required before integration under the repository quality gates. Historical fixtures and evidence remain intact.
