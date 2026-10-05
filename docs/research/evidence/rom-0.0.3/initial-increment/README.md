# Initial 0.0.3 development increment

Date: 2026-10-05. Baseline: `091836e`. Status: development increment, not a release artifact. Verification used the working source before integration.

## Implemented scope

- Bounded Resource presentation metadata, registration validation, authorized discovery projection, and Rust/TypeScript fixtures.
- Human Resource titles, field labels, accessible control labels, and exact secondary IDs.
- Shared right-edge chevron and right inspector for Resource filters/details and Work inspection.
- Settings groups from disclosed Resources, including plugin Resources, through ordinary query and mutation contracts.
- Readable Work snapshots and operator filters, preserving unknown-outcome recovery request identity and navigation guards.
- Focused Work presentation modules and shared presentation helpers.
- AI workflow routing, a project Studio skill, presentation authoring guidance, and expanded README.

## Verification

| Check | Result | Scope |
| --- | --- | --- |
| Full `./scripts/check`, repeated after final refactor | Exit 0 | Strict OpenSpec, script tests, native formatting, Clippy, workspace tests/docs, downstream checks, auth and identity verifiers |
| `npm run check` | 0 errors, 0 warnings | Svelte/TypeScript source |
| `npm run test:unit` | 98 passed | Client contracts and deterministic presentation helpers |
| `npm run build:components` | Exit 0 | Application and component browser fixtures |
| `npm test` | 66 Chromium scenarios passed | Existing browser suite plus labels, Settings, inspector, and Work regressions |
| WebKit direct upstream launcher | Launch failure | NixOS rejects the generic Linux dynamic executable; no scenario executed successfully |
| First WebKit accepted-runtime run | 65 passed, 1 failed | Found tooltip handler composition regression on immediate panel reopening |
| Final Chromium + WebKit suite | 132 passed | Full suite after handler fix; 66 scenarios per engine |
| Independent final source review | No blockers | Canonical field names, actual opener focus, Work extraction, and pending recovery identity |
| Documentation and AI source links | 135 targets exist | README, AGENTS, AI map, presentation guide, project skill |

Native execution used the existing `rom-dev` container:

```sh
systemd-run --machine=rom-dev --wait --pipe /run/current-system/sw/bin/bash -lc 'cd /workspace/ROM && CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 ./scripts/check'
```

The [first native log](full-local-check.log) records the complete command output and successful service exit.
The [final native log](full-local-check-final.log) records the repeated verifier after the refactor.
The [unit log](studio-unit.log) records all 98 assertions.
The [first WebKit log](studio-webkit.log) preserves the launcher failure.
The [patched runtime log](studio-webkit-patched.log) preserves the browser regression result.
The [final browser log](studio-browser-final.log) records 132 passing scenarios after the fix.
The final source identity is recorded in `source-manifest.json`.
The source review did not independently execute these commands.

## Refactor boundaries

`ResourceSummary` and presentation helpers own human title and exact identity display.
`InspectorToggle` and `ResponsiveInspector` own the shared inspector surface.
Settings reuses `ResourcePage`; it has no separate configuration mutation API.
`WorkDetails`, `WorkFilters`, and `WorkRecovery` own presentation responsibilities.
`WorkPage` retains request orchestration, confirmation identity, pending outcomes, and navigation guards.
Existing Rust public paths remain exported through module facades.
No historical source, worktree, prototype, or evidence was removed.

## Remaining limits

This increment does not implement the complete semantic control catalog or publish 0.0.3.
Date/time, color, decimal/unit, Blob semantics, relation pickers, nested object editing, and sortable-list adoption still require conformance work.
Derive metadata annotations and independent packaged custom-renderer composition remain pending.
Declared field groups are projected but do not yet arrange form sections.
Settings-kind navigation replaces local form drafts; closing the inspector and resizing preserve drafts.
External custom renderers must adopt the optional human-label prop to expose it in accessible control names.
The whole-screen audit lists additional acceptance cases; not every listed case was executed in this increment.
The current Work contract supplies snapshots, not a history source for a timeline.

## Regression fix

The inspector child Button had overridden the Tooltip trigger click handler.
Moving the callback to `Tooltip.Trigger` preserves the library's composed handler and closes the tooltip before opening the Sheet.
The repeated mobile Work test asserts focus entry, Escape, immediate reopening, and focus return.
No timeout was added to the component.
