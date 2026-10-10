# Installed public workspace candidate

Date: 2026-10-08. Status: executed installed-source candidate; release admission remains open.

The consumer imports components from `rom-studio/ui/components` and helpers from `rom-studio/ui` and `rom-studio/observe`.
It also imports the public controls and wire types. It uses no producer source alias.
The verifier archives the current Studio source and installs the extracted package with the frozen consumer lock.
It rejects symlinks to the checkout and dependency copying. Installed source must match the archive manifest.

## Actual command

```sh
ROM_PUBLIC_CONTROLS_BROWSER=1 ROM_CHROMIUM_PATH=/root/.nix-profile/bin/chromium ROM_WEBKIT_EXECUTABLE=/var/tmp/rom-studio-webkit-2359/pw_run.sh node studio/tests/public-controls/verify.mjs /var/tmp/rom-010-installed-public-workspace-corrected-20261008
```

The command exited zero. Type checking found zero errors and warnings. The build passed.
All 22 browser cases passed across Chromium and actual WebKit 2359. No case was skipped.
The result identifies the source archive, source manifest, source lock, consumer lock, and realized installed graph.
Evidence: `/var/tmp/rom-010-installed-public-workspace-corrected-20261008/result.json`.

The first run failed because the new consumer used the wrong HistoryList property and captured reactive generation implicitly.
The consumer now passes the documented label, empty text, locale, and message resolver.
The observation explicitly captures initial generation zero. Later revocation uses the public rebind operation.
That setup failure is retained in `/var/tmp/rom-010-installed-public-workspace-20261008/type-check.log`.

## Verified interaction scope

The equipment workspace composes ResponsiveDetails, HistoryList, ReferencePicker, SelectionCard, and LayoutControls.
Keyboard selection updates the host-controlled card. The secondary action remains outside the card button.
The reference editor accepts an exact ID without requiring a lookup provider.
History selection retains its exact inspection ID. Layout commands update only the host-confirmed presentation.
The workspace remains within its viewport at 390 pixels.
Existing cases verify retained editor drafts, late preview fencing, and synthetic authority-generation clearing.

The [independent component review](rom-0.1.0-selection-layout-independent-review.md) separately passed seven focused tests and twelve adversarial probes.
The reviewer matched the author-frozen source. It did not execute this installed consumer.

## Remaining acceptance

This run uses `authoring_checkout` provenance. It is not an admitted clean release artifact.
It does not verify actual server policy, reference lookup, durable Settings persistence, or the maintenance portal HTTP journey.
Complete those paths against both real database adapters through the shared mutation and recovery contracts.
Original Astral Plane acceptance and separate human authoring and accessibility assessments remain open.
The complete local verifier has not been rerun against all current 0.1.0 changes.
