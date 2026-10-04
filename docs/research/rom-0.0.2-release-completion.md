# ROM 0.0.2 release completion record

Date: 2026-10-04 UTC. Inspected baseline revision: `dbcf23d` on
`codex/rom-0.0.2-studio`. Status: incomplete. This record does not accept a
release artifact or a persistent preview.

The [scenario index](rom-0.0.2-scenario-index.md) links focused evidence.
The [plan acceptance map](rom-0.0.2-plan-acceptance-map.md) records an earlier
inspection cutoff. Historical failed producer images and their evidence remain
unchanged.

| Requirement | Current evidence | Completion boundary |
| --- | --- | --- |
| One Resource contract and generic operations | [Full browser workflows](rom-0.0.2-full-browser-workflows-results.md), [field-editor checks](evidence/rom-0.0.2/field-ux/README.md), and [source review](rom-0.0.2-integration-review.md) | Current frontend workflows pass. The final extracted package must run from one accepted source revision. |
| Exact mutation, current authority, and recovery | [Resilience results](rom-0.0.2-resilience-results.md), [SDK results](rom-0.0.2-sdk-resilience-results.md), and [provider maintenance](rom-0.0.2-provider-maintenance-journey.md) | Focused checks pass. The final native and browser gates remain required. |
| Generic Studio, human login, and reusable controls | [Field design and verification](rom-studio-field-ux-proposal.md), [shared filter workspace](rom-0.0.2-filter-workspace-verification.md), and [external author workflow](rom-0.0.2-studio-author-workflow.md) | The current frontend passed Chromium and WebKit host diagnostics with an existing native binary. The accepted extracted build remains pending. |
| Dependency and source notices | [Dependency checks](rom-0.0.2-dependency-check-results.md), [runtime notice source](rom-0.0.2-runtime-notices-results.md), [maintained inventory](maintained-dependencies.md), and [current package fixtures](evidence/rom-0.0.2/release-readiness/package-node.log.gz) | Current lock identities are recorded. All 56 package and artifact fixture tests passed on the inspected revision. Final emitted notices need artifact-bound verification. |
| Skills and external consumers | [Studio author workflow](rom-0.0.2-studio-author-workflow.md), [packaging results](rom-0.0.2-studio-packaging-results.md), and [release skill](../../skills/rom/rom-release/SKILL.md) | The fixed producer must execute skill examples and the independent extracted consumer. |
| Complete local artifact | [Release procedure](../../scripts/release-artifacts/README.md) | No current completed eight-gate artifact, manifest, checksums, or independent extraction verdict exists. |
| Protected durable preview | [Preview plan](rom-0.0.2-preview-deployment-plan.md) and [scoped activation](rom-0.0.2-scoped-preview-activation.md) | The temporary VPN preview uses current frontend assets and an older native executable. A release-bound persistent service and restart acceptance remain pending. |
| Main integration and publication | Current branch `codex/rom-0.0.2-studio` | Integrate the verified source and publish its exact release identity after artifact and preview acceptance. |

The last bounded producer failed at gate 8. Its retained record reports 16
browser passes and eight failures. The source later corrected the destructive
alert contrast and browser workflow. A current frontend diagnostic passed 12
Chromium and 12 WebKit real-host scenarios across SQLite and redb. These runs
used the maintained native binary. They do not replace gate 8's fresh native
build, extracted frontend, and external author test.

Commit `1700156` added compact generic fields and responsive action controls.
Its [frontend browser gate](evidence/rom-0.0.2/field-ux/studio-browser-runtime-1700156.log)
passed 108 component cases across Chromium and WebKit, 83 unit tests, and 35
Node component tests. Svelte reported zero errors and warnings. This gate used
fixtures and local frontend assets. It did not run the current native host or
the final extracted-asset acceptance.

An independent review then found a read-only inspection regression and
incorrect action-input wording. Commit `5cee8fb` corrected both. Its
[browser gate](evidence/rom-0.0.2/field-ux/studio-browser-runtime-readonly-5cee8fb.log)
passed 114 Chromium/WebKit component cases. The reviewer confirmed the source
fix and inspected the new tests. This does not change the pending release
artifact or preview boundary.

Commit `948bd70` made the mobile Create action the same square size as the
other icon controls. The full Studio check passed with zero Svelte errors or
warnings and 114 Chromium/WebKit component cases. The VPN preview served the
built `index.html` with SHA-256
`80b809e279e0c3948b078542142a60e05de6cd5b4e3573f826ae6cd5b49a16f3`.
The preview still uses an older native executable and is not the accepted
release deployment.

Commit `1e6c8dd` corrected UTF-8 decoding across process-output chunks in
the skills runner. A regression reproduced the replacement-character failure
before the change. The [skills-tooling test log](evidence/rom-0.0.2/release-readiness/skills-process-1e6c8dd.log)
records nine passing tests after the change. Its SHA-256 is
`52e0b1fed83d9a1d83a7fe0a0e629945ea676909d51edc46d2a6bbd13d77a1bf`.
This check does not execute all four bundled skill examples or the full
release producer.
The [package and artifact fixture log](evidence/rom-0.0.2/release-readiness/package-node-1e6c8dd.log)
records 56 passing cases in the Rust-equipped ROM container after that change.
Its SHA-256 is
`f5c51ad3627801c8b1e39139354a787f91ea26705232ba698f5b107e66e7b26a`.
These fixtures inject a finite gate runner. They do not replace the complete
eight-gate release run.

The shared disk has a 92 GiB safety floor. The most recent approved producer
plan requires a finite reservation and advance launch notice. A source change
requires a fresh admission and source identity check. Do not start an
unbounded producer because a read-only snapshot shows enough free bytes.

A [read-only retained-cache diagnostic](rom-0.0.2-retained-cache-diagnostic.md)
identified all four attribute names in the selected failed image and preserved
its before/after hash. It did not accept that image as a build-cache lower.
The previous complete-producer reservation still does not fit above the floor.

On inspected baseline `77cc178`, the ROM NixOS container ran
`node --test scripts/packages/*.test.mjs scripts/release-artifacts/*.test.mjs`.
All 56 fixture tests passed. The retained [compressed log](evidence/rom-0.0.2/release-readiness/package-node.log.gz)
has SHA-256 `c80b2289f7813e9ee611db1abe7a57b9de919b0cab1a7f4d1969227e1b36de9d`.
The uncompressed log has SHA-256
`e12171d034bb75bfa08c93eae8373a1cccdeba0b445599c947c6a13956c406ec`.
This is source-level package behavior, not a produced archive.

## Actions before release acceptance

1. Recheck source, locks, dependency inventories, and final documentation.
2. Obtain a fresh finite producer admission with the shared disk floor.
3. Run the unchanged eight-gate producer from the clean source revision.
4. Verify the completed archive, manifest, original notices, and extracted consumers independently.
5. Deploy the accepted native executable and frontend assets to the protected NixOS preview.
6. Verify HTTPS login, mutations, streams, attachments, restart, and client access.
7. Integrate the accepted source and publish the recorded release identity.

Keep the failed images and their evidence. A passing focused test must not
replace a failed or unexecuted release gate in this record.
