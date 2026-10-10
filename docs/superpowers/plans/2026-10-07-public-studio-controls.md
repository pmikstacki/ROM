# Public Studio Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Let independent Svelte applications compose existing ROM controls through public package entries.

**Architecture:** Add a controls-only export facade and reexport it from the stable root. Preserve existing component implementations and distinct prop semantics. Package metadata and narrow relative imports make the controls-only graph independent of consumer source aliases.

**Tech Stack:** Existing Svelte 5, bits-ui, Vite, Tailwind and Playwright dependencies.

**Spec:** `openspec/changes/expose-studio-control-api/design.md` and `specs/studio-controls/spec.md`.

## Global constraints

No new dependencies, browser auth exports, commits, consumer modifications or deployment.
Use isolated fixture build directories and browser ports. Keep failed evidence.
Coordinate commands and ownership changes with the root agent before execution.
The full verifier and independent artifact checks remain integration gates.

## Ownership and interfaces

This worker owns `studio/src/index.ts`, `studio/src/controls.ts`, `studio/tests/public-controls/`, this plan and the OpenSpec change.
Package exports and nine public components' private utility imports require root approval before edits.
The fixture imports only `rom-studio/controls` and `rom-studio/styles`.
The public catalog is Input, Button, Textarea, NativeSelect, NativeSelectOption, NativeSelectOptGroup, Checkbox, Slider and Label.

## Task 1: Independent consumer red/green

- [x] Add a Svelte consumer binding text, select, checked and single-slider values, plus native/bits-ui refs.
- [x] Add isolated fixture preparation and Svelte check/Vite build commands with no source aliases.
- [x] Run the fixture before exports; record the missing-entry or missing-export failure.
- [x] Add the controls facade and root reexport; retain Input's public name and all existing APIs.
- [x] Apply approved package exports and narrow relative utility imports.
- [x] Repeat fixture type check and production build; record passing output and CSS files.

## Task 2: Browser public-contract acceptance

- [x] Test text/select/checkbox/slider bindings with keyboard and native inputs.
- [x] Check element ref targets, disabled/invalid states and focus movement.
- [x] Check NativeSelect wrapper styling and explicit inner data-slot styling.
- [x] Check bounded textarea geometry at 1280x900 and 390x500.
- [x] Use isolated preview on port 43281; run Chromium and the configured actual WebKit executable.
- [x] Record engine-specific results; missing WebKit is a visible acceptance limit.

## Task 3: Verification and handoff

- [x] Run `npm run check` and `npm run test:unit` after command allocation approval.
- [x] Validate the OpenSpec change.
- [x] Request independent code review and full verifier integration from the coordinator.
- [x] Record source identity, dirty paths, tool versions and retained logs.

## Review focus

Root imports must not broaden browser auth support.
A public-export test must execute the copied package, rather than inspect export text.
NativeSelect wrapper class must not be misrepresented as inner select class.
Source package CSS requires the documented Svelte/Tailwind pipeline; browser CSS assertions must detect a missing stylesheet.
Single and multiple Slider props retain their existing discriminated representation.

## Current state

Targeted implementation and acceptance passed. Independent review, full verifier and release artifact acceptance remain pending.

## Targeted execution evidence

Producer HEAD was `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470` with the combined R1/R2/R3 working changes.
The intended red run installed the extracted source package, then failed only because `rom-studio/controls` was absent.
Its log is `/var/tmp/rom-public-controls-red-fresh-20261007/type-check.log`.
The accepted fresh installed run is `/var/tmp/rom-public-controls-browser-20261007/result.json`.
Svelte check reported zero errors and warnings. The Vite/Tailwind production build generated CSS and JavaScript assets.
Six browser tests passed: three actual WebKit and three Chromium cases.
Producer `npm run check` reported zero errors and warnings. Producer `npm run test:unit` passed 146 tests.
The pinned OpenSpec 1.14.0 validator accepted `expose-studio-control-api`.
Detailed source hashes and limits are in `/var/tmp/rom-public-controls-evidence-20261007.json`.

The accepted run used a fresh dependency installation with ordinary registry/cache retrieval. It did not copy node_modules.
Initial offline-cache failures and a preview working-directory failure remain retained as fixture setup evidence.
The stylesheet has no font asset URL. Package CSS imports and compiled browser styles were exercised.
No root App alias-free, SSR, new font asset or released-artifact support is claimed.
The coordinator owns full verifier, independent review and release admission. Those gates remain open.

## Supported integration command

```sh
ROM_PUBLIC_CONTROLS_BROWSER=1 ROM_WEBKIT_EXECUTABLE=/path/to/pw_run.sh node studio/tests/public-controls/verify.mjs /var/tmp/new-public-controls-output
```

The optional positional argument selects a new evidence directory. Without it, the verifier creates a fresh temporary directory.
Each child command has a 180000ms timeout and an 8MiB output limit. Preview uses fixed port 43281 with strict binding.
Playwright uses one worker. No shared component build or server is used.

## Independent review corrections

The initial accepted fixture graph cloned producer development tools. Its fresh install was seeded, not enforced as a locked replay.
The retained earlier evidence remains valid for that limited profile and remains unchanged.
Its source record identifies `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`, correcting the stale producer identity in the earlier paragraph.

### Task 4: Correct dependency and admission boundaries

- [x] Write negative unit cases for existing evidence, dependency drift, absent engine/browser work, and source mismatch.
- [x] Observe failure before introducing the admission helper.
- [x] Make shadcn-svelte and tailwindcss runtime stylesheet dependencies with matching producer lock metadata.
- [x] Declare a minimal consumer manifest and review its independent frozen lock.
- [x] Reserve evidence output exclusively and retain failures in fresh directories.
- [x] Add provided-archive/extracted-source selectors, locked npm ci, installed-source hashes, and realized dependency checks.
- [x] Distinguish bootstrap, authoring, locked verification, and complete release browser admission.
- [x] Run minimal bootstrap type check/build and fast admission tests.
- [x] Run locked supplied-artifact Chromium/WebKit admission on the coordinated port.
- [ ] Integrate fast tests, independent consumer checks, and release admission through coordinator-owned verifier modules.

The corrective verifier rejects dependency copying. It records realized fixture locks separately from source locks.
No root App alias-free, SSR, or arbitrary installation-layout support is added by these corrections.

- Raw archive handling reuses shared release extraction after bounded member/path/link and compressed/expanded byte admission. Two negative archive tests failed before the helper and now pass.
- Seven total fast admission tests pass. Candidate archive includes ROM license/notices; installed source hashes include those files.
- Locked supplied-candidate admission passed: `/var/tmp/rom-public-controls-locked-admission-20261007/result.json`. Three Chromium and three actual WebKit tests passed.
- Explicit WebKit executable: `/var/tmp/rom-studio-webkit-2364/pw_run.sh`; runtime revision 2364. Chromium reports 138.0.7204.49.
- Candidate artifact evidence does not establish completed release distribution. Coordinator integration and full verification remain open.
