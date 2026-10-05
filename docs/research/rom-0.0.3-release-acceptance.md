# ROM 0.0.3 release acceptance

Status: implementation acceptance passed; clean-source distribution and persistent preview remain pending.
Date: 2026-10-05. Initial source baseline: `6d3b6b2`. Earlier committed candidate: `5538323`.
Do not use this record as a completed release declaration.

## Owner's completion contract

The Studio and backend requests form one release scope. They are not optional polish after publication.
Completion requires the shared right inspector, readable Work inspection, plugin Settings, and browser ergonomics regressions.
It also requires generic backend support, focused DRY/module cleanup, AI workflows, and the neutral Studio favicon.
Mockups and implementation commits do not complete this scope.
The goal remains active until the complete artifact passes independent verification and the permanent preview passes acceptance.

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

The `5538323` producer passed gates one through six, then encountered a native WebKit compositor crash in gate seven.
The same browser case passed five consecutive repetitions without a source change.
The repeated complete producer then failed the provider deadline regression in gate six.
The failed assertion expects a fresh verification to finish within the same 30-millisecond caller deadline.
The [deadline investigation](rom-0.0.3-auth-deadline.md) separates response timeout from admission and completion.
The corrected regression retains the short deadline and proves re-admission through a second provider contact and bounded drain.
Do not treat the focused repetitions as complete producer acceptance.
Both failed stages and their logs remain preserved outside the accepted evidence set.

The `a7b48ff` producer passed gates one through seven and all 28 standard actual-host browser cases.
Its external-author browser workflow then failed on obsolete dialog and revision-summary selectors.
The example already registered an inline scalar editor. Its test still tried to open the former dialog.
This is a delivery defect in the maintained example, not permission to omit independent author acceptance.
The corrected example must pass its source-extraction workflow before the complete producer runs again.

The [external-author correction](rom-0.0.3-external-author-ergonomics.md) passed the fresh extracted SDK workflow and full local verifier.
Four browser cases passed across both engines and databases without skips or longer deadlines.
The corrected clean-source producer and permanent preview remain mandatory.
