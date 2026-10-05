# ROM 0.0.3 control adoption experiments

Date: 2026-10-05. Scope: generic sortable lists and exact-value JSON document editing.

These are isolated adoption probes. They do not certify maintained Studio integration or release readiness.
No maintained Studio, core, Cargo, or global browser preview files were changed by this experiment.

## Recommendation

Adopt `svelte-dnd-action` through one generic list adapter, with manual keyboard move controls available at all times.
Use `lossless-json` with a bounded text editor for exact JSON documents.
Keep `svelte-jsoneditor` optional and lazy loaded if a later use case requires its tree and query interface.
Its successful precision tests do not justify its much larger default bundle.

The adapter must own stable row records: `{ id, value, draft, error }`.
A row ID is editor identity, never a wire value. Duplicate values must have different row IDs.
Use keyed Svelte rows. Move the row record, so its recursive editor, draft, error, and focus move together.
Keep `consider` changes local. Emit candidates from `finalize` after removing shadow rows.
Use a unique zone type per list and reject drops from other lists.
The library's keyboard reorder sends `finalize` while moving, so this callback must only update the form candidate.
It must not invoke a backend mutation.

Place this adapter below `ValueEditor`, alongside other shared renderer adapters.
Give it generic rows and recursive child content. Do not add Resource-specific list components.
Manual moves and drag completion must use the same row reorder and candidate projection functions.
Readonly state must disable both routes.
Escape ends library dragging; it does not promise to undo earlier keyboard reorder candidates.
If cancellation must restore order, the adapter needs an explicit starting snapshot. Component destruction must destroy the library action.

For JSON, retain source text independently of the parsed value.
Run the size and depth preflight before parsing. Run strict parsing before accepting a candidate.
Preserve source number tokens, including large integers, decimal trailing zeroes, and exponent notation.
Convert parsed numeric wrappers to ROM's declared wire representation at the shared codec boundary.
Do not pass `LosslessNumber` instances directly through `WireValue`, or convert them through `Number`.
A backend string representation may retain the source text exactly.
A backend numeric representation may canonicalize tokens; the descriptor and codec must state that behavior.

This seam applies to the generic document representation. It does not infer JSON semantics from a string field name.
The expanded text editor can use existing shadcn-svelte Textarea and Button primitives.
Keep ordinary field rows compact, with a labeled expansion control.

## Candidates and primary-source review

| Candidate | Source-reviewed fit | Decision |
| --- | --- | --- |
| Existing Svelte/shadcn move controls | Native controls need no new library. Studio already separates temporary list keys from values. | Keep as the accessible fallback. The isolated manual probe verifies the row-record strategy. |
| `svelte-dnd-action` | Svelte actions support mouse, touch, keyboard, and drag handles. Each item requires a unique ID. Both lifecycle events need handlers. | Executed candidate; recommended. |
| Atlassian Pragmatic drag and drop | Framework independent and maintained; its low-level core leaves accessibility flows to the consumer. | Source-reviewed only. More integration work than the passing Svelte action for this scope. |
| React DnD | The project supplies React drag-and-drop interfaces. | Do not add React to Svelte Studio for this feature. No execution claimed. |
| `svelte-jsoneditor` | Svelte 5 package with custom parsers and text/tree modes. Its text-size warning can be overridden. | Executed precision candidate; optional only. ROM needs hard external bounds. |
| `lossless-json` plus text editor | Numeric values retain source tokens. Duplicate keys fail by default. | Executed smallest useful default. |

Primary sources: [Svelte DnD README](https://github.com/isaacHagoel/svelte-dnd-action),
[Pragmatic drag and drop README](https://github.com/atlassian/pragmatic-drag-and-drop),
[React DnD README](https://github.com/react-dnd/react-dnd),
[JSON editor parser API](https://github.com/josdejong/svelte-jsoneditor#parser),
[lossless-json README](https://github.com/josdejong/lossless-json).
Local installed package source was also inspected, rather than relying only on current upstream branches.

## Dependencies and maintenance evidence

| Package | Exact probe version | License | npm `time.modified` observed on 2026-10-05 |
| --- | --- | --- | --- |
| `svelte-dnd-action` | 0.9.80 | MIT | 2026-10-05T02:14:27.473Z |
| `svelte-jsoneditor` | 3.13.0 | ISC | 2026-07-24T18:12:52.757Z |
| `lossless-json` | 4.3.1 | MIT | 2026-07-31T14:05:45.693Z |
| `@atlaskit/pragmatic-drag-and-drop` | 4.0.0, metadata only | Apache-2.0 | 2026-09-24T01:55:52.626Z |

Registry modification dates are maintenance signals, not proof of release quality or future support.
The installed DnD package has no runtime dependencies and accepts Svelte 5 in its peer range.
The JSON editor declares Svelte `^5.0.0` and has a much larger dependency graph.

The isolated lockfile audit reported zero known advisories across 137 dependencies.
This describes the npm audit response on this date, not a complete security assessment.
See [audit output](../../prototypes/studio-controls-003/audit.json) and [exact lockfile](../../prototypes/studio-controls-003/package-lock.json).

## Executed interaction results

The final run records 33 passing assertions and one explicitly unexecuted case.
Chromium and WebKit executables are recorded in [results](../../prototypes/studio-controls-003/results.json).
The Chromium touch check uses CDP touch input. A separate synthetic TouchEvent check also passed.
WebKit touch was not executed: its runtime rejected the synthetic Touch constructor.
This is not evidence of failed physical touch support, and no physical-device touch result is claimed.

Both browsers passed these cases:

- Manual keyboard move retains duplicate-value identity, invalid draft, row error, and input focus.
- Library keyboard handle reorder moves the duplicate row and retains its draft and error.
- Actual mouse dragging reorders from the handle and retains the invalid draft.
- Editing a nested input does not start a drag.
- Destruction during keyboard dragging permits remount and another move without a leftover drag element.
- JSON text preserves `18446744073709551615`, `1.2300`, and `1e+03`.
- JSON text/tree/text conversion preserves those numeric tokens.
- Malicious HTML source stays text; no image element is created.
- Invalid JSON remains a draft and does not become an accepted candidate.
- The smaller text editor rejects invalid JSON, depth overflow, and size overflow while retaining the earlier accepted document.
- No browser `pageerror` events occur during the final scenarios.

Node probes passed strict syntax rejection, duplicate-key rejection, UTF-8 size bounds, depth bounds, and quoted-bracket handling.
They also distinguish null, false, zero, empty string, empty list, and empty object.
The preflight uses a 65,536-byte limit and maximum container depth of six.
It scans string quoting before counting brackets; strict JSON parsing supplies syntax validation.

An initial probe used a native Button as the library handle. Keyboard and pointer dragging did not start.
The installed library ignores nested native controls, including targets with `value` or `disabled` properties.
Use the documented span/div handle with the action-provided button role and tabindex.
Style it with Studio tokens. Do not attach `use:dragHandle` to the shadcn Button primitive.

Initial focus failures also exposed a harness race: it waited for a manual row before the lazy sortable component mounted.
The final harness waits for the sortable handle before interaction.
Retained [initial results](../../prototypes/studio-controls-003/results-initial.json) record those failures.
They are not a claim that the final adapter fails.

## Executed bundle measurements

Vite 8.3.2 built the isolated Svelte 5.57.1 application with separate lazy entry chunks.
These are minified chunk sizes, not maintained Studio's exact incremental bundle delta.

| Lazy path | JavaScript | Gzip JavaScript | CSS | Gzip CSS |
| --- | ---: | ---: | ---: | ---: |
| Manual rows | 1.13 kB | 0.68 kB | shared | shared |
| Sortable rows | 39.52 kB | 13.12 kB | shared | shared |
| Exact textarea | 0.84 kB + 6.01 kB shared parser/bounds | 0.56 kB + 2.33 kB | 0.13 kB | 0.13 kB |
| Full JSON editor | 754.97 kB + shared parser/bounds | 246.42 kB + shared parser/bounds | 104.83 kB | 12.19 kB |

The JSON editor also emits an optional color-picker chunk: 18.47 kB, or 6.54 kB gzipped.
The shared application chunk is 55.81 kB, or 21.48 kB gzipped.
See [build output](../../prototypes/studio-controls-003/build.log).

## Execution and safety limits

Commands were executed from `prototypes/studio-controls-003`:

```sh
npm install --ignore-scripts --no-fund --fetch-timeout=30000
npm audit --json
npm run build
npm run dev
npm run probe
```

The prototype server used port 43323. It did not use the active Studio preview port 43173.
The prototype server was stopped after the final probe.
The npm install and lockfile belong only to the prototype.
Source identity starts from repository commit `6d3b6b24b1447a78abb96c13e5957b3654745f07` plus the recorded prototype files.
[Source hashes](../../prototypes/studio-controls-003/source-identity.json) identify the executed files and lockfile.
[Probe source](../../prototypes/studio-controls-003/probe.mjs) records the individual scenarios.

The JSON editor's default query language is `jsonquery` in installed source.
Its optional JavaScript and Lodash query plugins use `new Function`.
The probe does not configure them. The built document chunk has no `new Function` or `eval(` text matches.
This narrow inspection is not a full executable-code audit.
Do not configure executable query languages in a ROM document control.
No raw HTML rendering is used by the prototype adapters.

These probes do not test backend authorization, wire serialization, receipts, readonly integration, nested drag zones, or long drag sequences.
They do not replace maintained component tests or the full local verifier.
The isolated manual control has the same ownership strategy proposed for Studio; it is not a direct execution of maintained `ValueEditor`.
Physical assistive-technology review and physical WebKit touch remain useful acceptance checks.

## Maintained integration acceptance

The maintained Studio now uses `svelte-dnd-action` 0.9.80 through `ListEditor.svelte`.
`ValueEditor.svelte` retains its public path and delegates list ownership to that component.
Each row keeps its identity, canonical value, error, and editor draft outside the drag shadow.
Manual move controls remain available beside the accessible drag handle.
Readonly and bounded lists do not mount a drag zone.
Collection limits remain six levels and 100 editable items.

Executed regressions found three integration defects before their fixes.
Pointer shadow replacement lost invalid input drafts.
A new deep-state filter rule lost its identity and remounted its editor.
A map key named `__proto__` returned an inherited draft object and modified `Object.prototype`.
The final tests preserve invalid drafts and errors across reorder.
They also verify prototype-shaped map keys without changing global prototypes.
No row identity, draft, error, or shadow metadata appears in submitted values.

Enum metadata uses the shared native discovery fixtures.
Labels remain advisory and never replace accepted wire members.
Duplicate display labels retain distinct options.
The searchable list chooser appends members in schema order.
Existing order and duplicate values remain unchanged.
Closing the chooser discards its pending choices without a value emission.
Unknown stored enum members remain visible and block submission until a valid member is selected.
Resource, action, and filter tests execute these generic controls.
Nested display tests retain exact unknown members through optional, nullable, list, and map wrappers.

The renderer registry accepts an explicit inline layout for registered scalar controls.
A collection remains expanded even when its leaf renderer requests inline layout.
The browser test verifies that a stale unregister callback cannot remove a newer registration.
An invalid layout registration fails.
Unknown codecs retain the existing readonly fallback.

The final maintained frontend inputs were frozen before the component build.
[Source hashes](../../prototypes/studio-controls-003/final-maintained-source-identity.json) identify the recorded inputs.
The Chromium and WebKit runs use the same component build.
The final results are:

| Check | Executed result | Evidence |
| --- | --- | --- |
| Svelte check | 0 errors, 0 warnings | [Check log](../../prototypes/studio-controls-003/final-check.log) |
| Unit tests | 146 passed | [Unit log](../../prototypes/studio-controls-003/final-unit.log) |
| Component build | Passed | [Build log](../../prototypes/studio-controls-003/final-build.log) |
| Full Chromium suite | 106 passed, 34.6 seconds | [Chromium log](../../prototypes/studio-controls-003/final-browser.log) |
| Full WebKit suite | 105 passed, 1 skipped, 43.5 seconds | [WebKit log](../../prototypes/studio-controls-003/final-browser-webkit.log) |

The skipped case uses Chromium CDP touch input.
WebKit pointer and keyboard reorder tests execute and pass.
Physical WebKit touch and assistive-technology use remain outside this automated evidence.
The final logs contain process warnings about `NO_COLOR` and `FORCE_COLOR`.
They contain no browser console error or unhandled-error entries.

The maintained attachment tests also execute actual browser upload bytes in both engines.
A local HTTP receiver reads the submitted Blob body.
A destination-only fetch adapter connects the SDK upload to that receiver.
The receiver can destroy the first response after it reads the bytes.
The retry test then verifies the frozen body bytes on the second request.
These tests do not use a fake body echo or skip the byte assertions.
The copy tests read the exact opaque ID after the real Copy button is activated.

Earlier integration logs remain available beside the final evidence.
They include failed draft, filter identity, and prototype-key cases.
They also include a provisional suite whose inputs preceded later auth and attachment fixes.
Use the final logs above for the frozen frontend acceptance result.
The root release task owns the full native local verifier and release integration.
These frontend results alone do not claim completion of that verifier.

### Later dialog lifecycle correction

The full frontend results above precede the subsequent final review fixes.
A review found that the expanded field dialog unmounted its `ValueEditor` after the close animation.
An immediate reopen had not exposed this defect.
Three new tests wait until the dialog is fully absent before reopening it.
Numeric list, decimal list, and prototype-shaped map drafts then reverted to their canonical values.
The [red log](../../prototypes/studio-controls-003/dialog-draft-red.log) records all three failures.

`FieldHost` now owns the persistent `EditorDraft` outside `Dialog.Content`.
Both framework `ValueEditor` paths bind to that state.
An explicit field mode change resets it.
Custom renderers retain the additive draft callback contract without a bindable-property requirement.
The reopened controls preserve invalid text, invalid state, and submission blocking.

After this correction, all 18 sortable tests pass in Chromium.
WebKit passes 17 tests and skips the same CDP touch case.
Svelte check reports zero errors and warnings.
All 146 unit tests pass.
See the [Chromium log](../../prototypes/studio-controls-003/dialog-draft-green.log),
[WebKit log](../../prototypes/studio-controls-003/dialog-draft-webkit-green.log),
[check log](../../prototypes/studio-controls-003/dialog-draft-check.log), and
[unit log](../../prototypes/studio-controls-003/dialog-draft-unit.log).
These affected browser tests used an isolated development server on port 43324.
They do not claim a new full-suite result after other concurrent review corrections.

### Latest frozen 0.0.3 frontend acceptance

A fresh build includes the persistent FieldHost draft, full readonly semantic inspection, and the 0.0.3 version bump.
The combined Chromium and WebKit run executes 220 cases.
It reports 219 passes and one skipped WebKit CDP touch case in 1.3 minutes.
The same inputs pass all 146 unit tests.
Svelte check reports zero errors and warnings.
The component build passes in 3.31 seconds.
No browser console error or unhandled-error entry appears in the combined log.
The process emits the existing `NO_COLOR` and `FORCE_COLOR` warnings.

This result supersedes the earlier frontend acceptance snapshot.
[Latest source identity](../../prototypes/studio-controls-003/review-final-maintained-source-identity.json) records all Studio source, test, configuration, and lockfile hashes.
See the [combined browser log](../../prototypes/studio-controls-003/review-final-browser.log),
[check log](../../prototypes/studio-controls-003/review-final-check.log),
[unit log](../../prototypes/studio-controls-003/review-final-unit.log), and
[build log](../../prototypes/studio-controls-003/review-final-build.log).
The shared component preview stopped after the suite.
The root release task continues to own the native verifier and real-host release results.
