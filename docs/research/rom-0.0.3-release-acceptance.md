# ROM 0.0.3 release acceptance

Status: implementation acceptance passed; clean-source distribution and persistent preview remain pending.
Date: 2026-10-05. Source baseline: `6d3b6b2`, with coordinated release changes uncommitted.
Do not use this record as a completed release declaration.

## Implemented scope

The shared right inspector covers filters, Resource details, Work inspection, and Attachments.
Settings navigation selects authorized ordinary Resources, including plugin groups.
Human labels and title fields remain separate from exact Resource identities and authorization.

The standard catalog provides ten explicit version-one semantic codecs and their shared controls.
Enum labels preserve wire members. Ordered multi-choice lists support dragging and ordinary move controls.
Nested editors retain invalid drafts and private row identity without submitting editor metadata.
Reference choices use current disclosed kinds and bounded queries. Unknown codecs preserve values and block unsafe editing.

The Resource derive supplies presentation and enum metadata without a handwritten frontend schema.
Current backend validation remains authoritative for create, patch, and action inputs.
Module review and AI workflow guidance retain public paths and focused responsibilities.

## Executed development acceptance

| Check | Result | Evidence |
| --- | --- | --- |
| Frozen component bundle | 221 browser passes; one explicit WebKit physical-touch skip | [Full Studio gate](evidence/rom-0.0.3/catalog-integration/full-studio-check-003.log) |
| Frontend units and typecheck | 146 passes; zero Svelte errors or warnings | [Unit log](../../prototypes/studio-controls-003/review-final-unit.log) |
| Full local verifier | Passed after separate compiler-fixture locks were updated to 0.0.3 | [Final verifier](evidence/rom-0.0.3/catalog-integration/full-check-003-final.log) |
| Actual immutable native host | 28 passes across Chromium/WebKit and SQLite/redb | [Runtime log](evidence/rom-0.0.3/catalog-integration/actual-host-standard-003.log) |
| Independent specification review | Two implementation gaps corrected; final release gates remain pending | [Review](rom-0.0.3-final-spec-review.md) |
| Independent standards review | No hard violation in reviewed non-owned areas; two optional design notes | [Review](rom-0.0.3-final-standards-review.md) |
| Dependency review | Production npm audit reported zero vulnerabilities; installed dependency notices checked | [Review](rom-0.0.3-dependency-adoption.md) |

Failed runs remain preserved. A mutable shared native binary invalidated an early runtime attempt.
The repeated runtime used a private immutable copy; it did not weaken the host contract or bypass authentication.
Earlier descriptor/title changes also required equivalent assertions in the maintained browser workflows.

The final distribution must bind these checks to a clean revision, locks, production assets, and an independently verified manifest.
Screenshots and preview identity must identify implemented source and assets rather than design mockups.

Actual-host screenshots identify their source and assets in [the capture record](evidence/rom-0.0.3/studio-screens/source-facts.json).
The first capture exposed an obscured UnitValue magnitude in a narrow inspector.
The corrected control passed a browser font-width regression and the repeated semantic host journeys.
Earlier captures remain in `studio-screens/before-unit-width/`.

The human-title table column now reads `Resource`; an ID-only column retains `ID`.
The existing presentation regression first failed on the old header, then passed across both browser engines.
See [red evidence](evidence/rom-0.0.3/ergonomic-review/rom-003-resource-header-red.log) and [green evidence](evidence/rom-0.0.3/ergonomic-review/rom-003-resource-header-green.log).

The first clean-source producer passed gates one through five, then rejected external fixture paths during packaging.
The [fixture portability correction](rom-0.0.3-package-portability.md) passed extracted-package verification; the complete producer must run again.
