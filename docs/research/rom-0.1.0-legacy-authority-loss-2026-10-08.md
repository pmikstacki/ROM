# Explicit browser authority loss

Date: 2026-10-08. Status: executed library correction; original-consumer integration remains open.

Source investigation found that Astral Plane clears private views for anonymous sessions and `SessionExpiredError`.
The legacy adapter previously returned a generic error for both confirmed denial and temporary unavailability.
An application could therefore retain private views after a confirmed server denial.
This application-level consequence is source-derived; it has not been reproduced in the deployed consumer.

## Correction

`createBrowserAuth` now throws `SessionDeniedError` for validated HTTP 401/403 responses with `error: denied`.
The new error is distinct from `SessionExpiredError`. Neither response invents an anonymous generation.
HTTP 503 with `error: overloaded` remains temporary unavailability.
The existing transport clears CSRF state for confirmed denial and retains it for temporary unavailability.

The `/auth` public entry exports the compatibility adapter and both error classes.
The existing application auth facade retains its paths and adds the new class.
Consumers must clear private views on either confirmed denial or expiry.
They must not treat every temporary acquisition error as authority loss.

## Evidence

Two maintained tests failed before the correction: HTTP 401 and HTTP 403 produced an ordinary error.
The HTTP 503 control passed before the correction.
After correction, all 17 legacy adapter tests passed, including the public error-class identity check.
The complete Studio unit suite passed 417 tests. Svelte checking reported zero errors and zero warnings.

Logs:

- `/var/tmp/rom-010-legacy-authority-loss-red.log`
- `/var/tmp/rom-010-legacy-authority-loss-public-green.log`
- `/var/tmp/rom-010-legacy-authority-loss-studio-units.log`
- `/var/tmp/rom-010-legacy-authority-loss-studio-check.log`

## Remaining acceptance

AP-UX-006, AP-UX-013, and AP-UX-021 retain their original-application acceptance requirements.
Astral Plane must adopt the explicit denial handling or the shared session lifecycle through an installed, verified package.
No deployed application or vendor snapshot was changed by this correction.
Browser confirmation, independent review, artifact admission, and the updated full verifier remain required.
