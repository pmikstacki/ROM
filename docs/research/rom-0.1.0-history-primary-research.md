# History composition research

The approved Task 3B contract keeps Resource IDs opaque. Hosts supply disclosed titles and navigation policy.
Investigation found no reusable history component or message catalog in Studio. The existing button wrapper already provides native semantics and package styles.

## Primary sources

[WAI's button pattern](https://www.w3.org/WAI/ARIA/apg/patterns/button/) describes accessible names, Space/Enter activation, and context-dependent focus.
History selection invokes a host callback. It therefore uses a native button rather than a link or clickable container.

The [aria-current reference](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Attributes/aria-current) distinguishes a current item from tab selection.
Only the host-confirmed current item receives `aria-current="true"`. This component is not a tab panel or listbox.

[Svelte boundary documentation](https://svelte.dev/docs/svelte/svelte-boundary) distinguishes rendering errors from unrelated asynchronous operations.
The history callback handles its own rejected Promise. It shows a host-catalog message, not private exception text.
These source conclusions inform the implementation. They do not establish screen-reader acceptance.

## Candidate contract

Import `HistoryList` from `rom-studio/ui/components`.
Entries contain exact IDs, supplied titles, integer Unix millisecond display instants, and optional nonnegative counts.
The default 200-entry ceiling is configurable. IDs and titles have 1024-byte and 4096-byte limits.
Capture reads each accepted field once. Validation and publication use that same detached snapshot.

The host supplies locale, messages, empty text, list label, authority token, and navigation callback.
Timezone defaults to UTC. Dates and numbers follow the explicit locale.
Full titles remain accessible. Narrow displays wrap long titles without rendering an ID as their name.
The host controls `selectedId`; callback completion does not optimistically change it.
A disabled prop supports the existing mutation/recovery navigation barrier. It does not infer commit status.

Pending selection disables further list selection. Failure shows a sanitized host message.
Authority comparison uses `Object.is`; changing authority invalidates old pending status and failure updates.
The host must also clear disclosed entries and fence its own awaited navigation effects.
No querying, saving, authorization, domain conversation inference, or local persistence is added.

## Executed evidence

- `/var/tmp/rom-010-ui-history-module-red.log`: focused tests failed because the module did not exist.
- `/var/tmp/rom-010-ui-history-module-green.log`: three initial behavior tests passed after implementation.
- `/var/tmp/rom-010-ui-history-public-red.log`: component build failed on the missing public HistoryList export.
- `/var/tmp/rom-010-ui-history-build-green.log`: corrected public-import build passed.
- `/var/tmp/rom-010-ui-history-browser.log`: twelve initial browser cases passed across Chromium and WebKit.
- `/var/tmp/rom-010-history-layout-capture-red.log`: three maintained capture regressions failed; nine earlier tests passed.
- `/var/tmp/rom-010-history-layout-capture-green.log`: twelve focused tests passed after snapshot corrections.
- `/var/tmp/rom-010-ui-history-nan-red.log`: the actual Chromium callback case remained pending before identity correction.

Corrective browser and complete unit results have separate logs. The independent review records their final source identity and result.
The fixture uses checkout source. Installed-source, original-consumer, and human acceptance remain open.
The complete local verifier remains required before integration.
