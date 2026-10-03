# ROM 0.0.2 reusable Studio controls

Date: 2026-10-03. Status: implemented controls and frontend scaffold; full Studio integration remains incomplete.

## Implemented boundary

The `studio` package provides generic Resource forms, tables, action forms, query controls, and a codec renderer registry.
Its current application shell does not simulate a successful backend connection. The parent integration task supplies the actual session and data flow.

The actual shadcn Button, Input, Dialog, and Table sources come from the tested official `vega` registry.
[Component provenance](../../studio/upstream-components.json) records upstream hashes, imported source hashes, aliases, and formatting changes.
The unchanged upstream MIT license is retained with the component sources.

The shared client owns wire parsing, integer normalization, operation presence, and mutation encoding.
Controls call `normalizeValue`, `objectInput`, `patchInput`, `parseWire`, and `stringifyWire` instead of creating another wire protocol.
The browser SDK and its unit tests are owned by the parent task.

## Exact component contracts

| Component | Public props and meaning |
| --- | --- |
| `FieldHost` | `descriptor: FieldDescriptor`, `intent: FieldIntent`, `onchange(FieldIntent)`, optional `readonly`, `error`, and `onerror`. |
| `ResourceForm` | `descriptor`, optional initial `value`, `mode: create/replace/patch`, and asynchronous `submit(ResourceFormSubmission)`. |
| `ResourceTable` | `descriptor`, `rows: ProjectedView[]`, and `onselect(ProjectedView)`. |
| `ActionForm` | `descriptor`, `action: ActionInput`, and asynchronous `oninvoke(WireValue)`. |
| `QueryEditor` | `descriptor` and `onchange(QuerySpec)`. It emits a filter, optional field ordering, and a bounded limit. |
| `registerRenderer` | Registers a trusted component under exact codec name and version. It returns a removal function and rejects duplicate registrations. |

`ResourceFormSubmission` is a discriminated union:

```ts
{ type: 'create' | 'replace'; input: WireObject }
| { type: 'patch'; input: Record<string, FieldUpdate> }
```

This shape is compatible with the client's `Operation` contract. Omitted fields do not become null or remove operations.
The parent must key a form by the selected Resource identity. A live value refresh must not recreate an open draft implicitly.
A descriptor-version change preserves the draft and blocks submission until explicit reconciliation.

Registered renderers receive descriptor, value, change callback, read-only state, and optional display mode.
The same registry supports editor, detail, and table-cell use. Unknown codecs display an explicit diagnostic and do not silently offer an editor.
Registration changes are reactive through SvelteMap. Runtime metadata never downloads executable frontend components.

## Presence and collections

The controls keep omit, null, value, and optional remove distinct. False, zero, and empty strings remain values.
Integer text is converted to BigInt before shared normalization. The components never pass an integer boundary through JavaScript Number.
The shared serializer writes bigint wire numbers without `JSON.stringify` on bigint values.

Recursive list and map editors have a six-level UI limit and a 100-item limit per collection.
The UI reports unsupported editing beyond those limits. It does not truncate the stored value into a new mutation.
Scalar input has an explicit text bound. Integer input has an additional short bound before BigInt conversion.

Enum choices come from the descriptor. Reference controls accept an ID through the same field machinery.
Remote reference lookup is a later integration enhancement; this task did not invent kind-specific lookup endpoints.
Opaque action metadata remains visible. Its explicitly labeled raw JSON editor uses the strict shared parser with byte and depth bounds.

## Executed evidence

The [identity record](evidence/rom-0.0.2/frontend-controls/verification-identity.json) records the executable toolchain, lock hash, and frontend source hashes.
Client sources were in development during this task. Their executed hashes are included; this is not clean release acceptance evidence.

| Check | Observed result |
| --- | --- |
| `npm ci --ignore-scripts` | Clean locked install completed. |
| `npm run check` | Zero errors and warnings. |
| `npm run build` | Production application build completed. |
| `npm run test:components` | Eight Chromium component browser tests passed. |
| `npm audit --json` | Zero vulnerabilities reported at execution. |

[Install](evidence/rom-0.0.2/frontend-controls/npm-ci-final.log), [types](evidence/rom-0.0.2/frontend-controls/type-final.log), [build](evidence/rom-0.0.2/frontend-controls/build-final.log), [browser](evidence/rom-0.0.2/frontend-controls/browser-accepted.log), [audit](evidence/rom-0.0.2/frontend-controls/audit-final.json).

The component suite tests u64 maximum, i64 minimum, false, null, remove, omit, and empty string behavior.
It also tests custom renderers, list edits, enums, references, prototype-shaped map keys, and opaque integer parsing.
Duplicate raw JSON keys are rejected before invocation. Generic row selection and filter generation use the same controls.
Keyboard interaction and axe A/AA scans passed for the assembled harness. These results do not certify accessibility or human productivity.

## Resilience correction found in the UI

A nested numeric error initially survived deletion of its list entry. The form remained disabled after the offending entry disappeared.
A [regression test failed](evidence/rom-0.0.2/frontend-controls/nested-red.log) for that exact expected state.
The correction gives list editors stable identities. It removes only the deleted entry's error and updates remaining error paths.
The test also proves that a valid sibling edit does not clear another entry's invalid input.
The expanded accepted suite includes this regression.

## Deployment and remaining work

Tailwind scans only declared source. Generated test output and evidence cannot add classes to the production build accidentally.
The component harness has a separate build mode and output directory. It is excluded from the ordinary production entry points.

The component tests use system Chromium 138 through an explicit executable path. A configurable path supports another compatible test host.
WebKit's NixOS runtime repair remains an acceptance obligation. The [compatibility report](rom-0.0.2-frontend-compatibility-results.md) preserves its exact failures and repair recommendations.

Actual OIDC login, current authorization, discovery, live subscriptions, unknown outcomes, and SQLite/redb journeys remain integration checks.
The full local verifier remains necessary after integration. This controls task ran no Cargo build and makes no native-release completion claim.
