# Maintenance portal HTTP conformance

Date: 2026-10-08. Status: executed source candidate; release admission remains incomplete.

The external application uses public ROM APIs and the generic HTTP adapter.
No Resource-specific route or repository was added.
Two tests run the same journey against SQLite and redb.

## Executed behavior

Each journey creates Equipment, Inspection, WorkOrder, and PortalSettings through actual loopback HTTP.
Responses retain the expected Resource kind, ID, and revision.
Another authenticated owner cannot read the created rows or complete the WorkOrder.
An unverified credential cannot read them. A forged subject header does not change these decisions.

The completion produces revision two. Repeating the original invocation returns identical projected JSON.
Another authenticated owner remains denied after the receipt exists.
Disabling the owner's fixture credential denies replay without disclosing the original result.
Restoring that credential permits the same receipt replay.
These are separate credential and ownership checks, not a same-principal domain-policy-change experiment.

An invalid inspection date returns HTTP 400 and adds no inspection event.
The test does not separately assert absence of an invalid row or receipt.
A Settings replacement persists columns three and hidden history at revision two.
An attempted transfer to another owner is denied.

The server drains before the test drops its storage reference.
A bounded weak-reference check confirms that no storage handle remains before the database reopens.
After reopening, Settings and typed reference/date projections retain their values.
The original completion still replays at revision two. The WorkOrder journal still contains exactly two events.

## Review and evidence

Independent source review found a query variable that shadowed the database factory.
The coordinator corrected it before compilation.
The review also identified the missing storage-handle release oracle and post-receipt ownership check.
Both checks were added before execution.

All six application tests passed: two field tests, two embedded journeys, and two HTTP journeys.
Clippy passed with warnings denied. Formatting ran before both commands.
The commands ran with Rust/Cargo 1.99 through the `rom-dev` container, two build jobs, and incremental compilation disabled.

Logs:

- `/var/tmp/rom-010-maintenance-http-tests.log`
- `/var/tmp/rom-010-maintenance-http-clippy.log`

Source identity and command scope are recorded in `/var/tmp/rom-010-maintenance-http-evidence.json`.

## Limits

Fixed fixture credentials are not production credential verification.
The bounded response reader accepts this fixture's unchunked, close-delimited JSON; it is not a general HTTP client.
No installed Svelte portal, live-stream interruption, lost acknowledgement, or production identity journey ran in this increment.
The complete verifier, admitted artifacts, original-consumer acceptance, and human accessibility assessment remain open.
