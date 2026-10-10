# Independent selection and layout composition review

Date: 2026-10-08. Scope: Task 3B repository-source candidate. No actionable correctness or standards defect was reproduced within this scope.

This review does not admit an installed consumer, the original application, human usability or a release artifact. Those acceptance gates remain open.

## Source identity and ownership

Reviewed `SelectionCard.svelte`, `LayoutControls.svelte`, their named TypeScript helpers, the existing `layout.ts` validator and `studio/src/ui-components.ts` facade. Reviewed the two focused unit files, browser spec and public-import harness.

The workspace HEAD was `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`. These files include uncommitted candidate source; HEAD alone does not identify the reviewed implementation. Node was `v22.16.0`.

The four product hashes matched the author's frozen manifest:

| File | SHA-256 |
| --- | --- |
| SelectionCard.svelte | `3bf110424466b9eac38845d5417f629ff49c33be2dd29c52d51a0d54d1ad1e10` |
| LayoutControls.svelte | `067cb63d682265fe6cc545ff8c2492258c1ec11a00224d52403a2535be6d64c6` |
| selection-card.ts | `da0099723f5c0d5e4d3fa28e02e10a01994023e977fd710f86136c8cf1689e76` |
| layout-controls.ts | `d9822ad0c8f356d7a2de0cb8770610c812b6d0a80d940fa0c400d24bb13427b9` |

The harness, browser spec and focused test hashes also matched the frozen manifest. Additional reviewed source and lock hashes are recorded in `/var/tmp/rom-010-selection-layout-review/source.json`. The final comparison is `source-after.json`.

Only this new report and `/var/tmp/rom-010-selection-layout-review/` were written by this reviewer. No source edits, dependency changes, commits, browser runs, native builds or server leases were made.

## Standards and specification review

The public facade contains exports only. Named helpers separate command lifecycle and layout proposals from Svelte rendering. Both components share one command barrier. The existing validator remains the bounds authority. Existing public paths remain present.

`SelectionCard` uses the existing Button primitive, which renders a native button without `href`. It exposes host-confirmed `aria-pressed` state. Independent actions render as siblings. The documented child-snippet restriction is necessary: consumers must place interactive children in `actions`.

`LayoutControls` provides native move, resize and visibility buttons. It disables invalid proposals through the shared validator. Proposals retain exact IDs and detach frozen values. Components do not mutate confirmed host state or select persistence. Domain catalogs and collision policy remain host-owned.

Thrown callbacks and unclassified results become unknown outcomes. They retain the barrier and suppress exception text. Explicit rejection permits another command. Disabled changes affect dispatch, while owned callbacks can still settle. Authority, recovery and confirmed identity transitions reset ownership. Token comparison uses `Object.is`.

A reset is a host statement, not evidence of backend resolution. Hosts must resolve or quarantine accepted work before replacement dispatch. Component fencing suppresses stale component status; it cannot undo callback side effects. Hosts must separately guard their continuations after awaits.

## Independently executed checks

The focused command passed seven tests, with zero failures or skips:

```sh
cd /root/ROM/studio
node --experimental-strip-types --test tests/unit/ui-selection-card.test.ts tests/unit/ui-layout-controls.test.ts
```

Twelve additional adversarial probes passed, with zero failures or skips:

```sh
node --experimental-strip-types --test /var/tmp/rom-010-selection-layout-review/adversarial.test.ts
```

Evidence is in `focused.log` and `adversarial.log` under the review directory. The independent probes establish these concrete results:

| Reproduction | Result |
| --- | --- |
| Return unknown, undefined, null, an object or an unrecognized string; dispatch again | One callback; pending then unknown; no retry. |
| Hold a callback; attempt a competing dispatch | One callback until explicit settlement. |
| Reset to another owner; reject new work; settle old work as accepted | New rejected state remains; old completion publishes nothing. |
| Use NaN authority and recovery tokens | Accepted completion settles normally under `Object.is`. |
| Reset/dispose during the pending ownership check | Obsolete callback does not dispatch. |
| Reset and dispatch replacement work inside pending publication | Only replacement callback dispatches and settles. |
| Disable during held work; settle unknown; enable again | Unknown barrier remains; enabling does not retry. |
| Apply all ten layout actions to `__proto__` | Exact target and other IDs survive; output is detached and frozen. |
| Capture getter-based catalog; mutate host configuration later | Each declared getter is read once; captured bounds remain unchanged. |
| Mutate columns inside a stored-value getter | Original captured bounds determine proposal validity. |
| Supply corrupt shapes, NaN, fractions, duplicate or trimmed IDs, unsupported commands | Invalid result; no fallback. |
| Increase a maximum-safe-integer width | Invalid result; no rounded or wrapped proposal. |

The disabled and NaN probes exercise helper ownership closures. They are not additional Svelte/browser evidence.

## Author-reported evidence and limits

The [author report](rom-0.1.0-selection-layout-evidence.md) records seven focused and 414 full unit passes, clean Svelte checking, a successful build and 64 browser passes. Twenty executions cover ten new cases across Chromium 138 and WebKit 2359. This reviewer inspected the final browser log; it ends with 64 passed. These broader commands were not independently rerun here.

Historical RED logs remain preserved. They distinguish missing exports/helpers, the fixture's mobile JSON overflow, stranded disabled completion, incorrect lost-acknowledgement rejection and stale dispatch after publisher reset/dispose. The focused rerun verifies the corrected lost-acknowledgement and publisher cases. Source inspection confirms the harness JSON output now wraps.

No severity-ranked implementation finding remains from this review. This conclusion is limited to the reviewed source and executed Node scenarios. Browser geometry, keyboard behavior and sibling-action behavior rely on the author's recorded runs.

Current-source installed public composition, durable layout reload, principal-bound mutations and real unknown-outcome recovery remain unverified by this review. Original application acknowledgement, human and assistive-technology acceptance, full local verification and release admission remain separate requirements.
