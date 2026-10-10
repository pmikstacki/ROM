# Controller session candidate review

Date: 2026-10-07. This review does not accept the full App workflow.

## Reviewed snapshot

The eight source hashes in `/var/tmp/rom-app-session-evidence/controller-evidence.json` matched current files at review time.
Independent execution of the targeted controller tests completed with exit code 0: 40 cases passed.
Evidence: `/var/tmp/rom-010-root-controller-targeted-20261007.log`.
The full Studio unit command also passed 240 cases during this increment.
Evidence: `/var/tmp/rom-010-browser-storage-unit-20261007.log`.
These results do not cover real App authentication, installed session recovery, or original-consumer acceptance.

## Finding: failed renewal becomes active

Source inspection found that `renewSession(preserve)` catches every query or read failure and clears rows and selection.
The caller then publishes `active`, `stale: false`, and `mutationAllowed: true` because renewal resolved.
A transient read failure after same-owner session renewal must not be treated as successful refresh.
Preserve context and editor state as stale; keep mutation dispatch disabled until authorized refresh succeeds.
A confirmed denial or missing Resource requires separate handling that does not expose stale private data.

The UI worker received this finding and a request for reproductions before implementation changes.
Required cases include transient discovery, query, and selected-row failures, plus a confirmed denial case.
At this review point, this is a source finding, not an independently executed failure reproduction.
The boundary remains unaccepted until the regression and correction pass review.

## Remaining acceptance

Inspect synchronous observer reentrancy during lane selection and editor publication.
Verify draft restore cannot overwrite a later queued edit after session replacement.
Then test actual App wiring with the installed package, real database adapters, and both browser engines.
Keep exact mutation identity, invalid draft text, current authorization, and principal isolation in those scenarios.
Do not close Astral Plane feedback groups from unit results alone.

## Revised boundary review

The worker reproduced the renewal finding and corrected the separate transient and denied outcomes.
Further reproductions covered reentrant publication, draft persistence, and numeric wire-token preservation.
The final metadata review reproduced two failures before the correction.
Discovery denial and selected-row denial could disclose an obsolete private descriptor.
The correction clears descriptors and legacy pending state while the denial fence remains active.

The coordinator verified eight frozen source hashes in `/var/tmp/rom-app-session-evidence/controller-metadata-evidence.json`.
Independent execution passed 57 targeted cases with no failures, cancellations, or skipped tests.
Evidence: `/var/tmp/rom-010-root-controller-metadata-targeted-20261007.log`.
The worker reports 257 unit cases and a clean type check for this snapshot.
The coordinator did not repeat those two commands during this review.

This review permits the planned App integration increment. It does not accept the complete application workflow.
Installed App scenarios, both database adapters, both browser engines, and original-consumer acceptance remain open.
The earlier snapshots and failure evidence remain preserved.
