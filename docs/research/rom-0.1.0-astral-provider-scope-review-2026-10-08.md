# Astral provider redirect scope: independent review

Date: 2026-10-08.

## Outcome

No blocking defect was found for an exclusively owned synthetic provider fixture. This result does not authorize production provider changes.

The reviewed helper is `tests/astral-adoption/provider-redirect.mjs`. Its maintained tests are in `tests/astral-adoption/provider-redirect.test.mjs`.
The reviewer did not change these files, run native builds, or contact a provider.

## Source findings

The helper admits only `http://127.0.0.1:43902/auth/callback/astral` before provider access (line 18).
It retains existing redirect entries and appends one strict entry (lines 23–29).
An existing exact strict callback requires no mutation. A regex entry does not count as exact admission.

Activation requires an exact PATCH response and a separate matching GET before the body starts (lines 34–40).
The restoration flag is set before PATCH. Therefore, a lost activation acknowledgement still enters restoration (lines 33, 44–50).
Restoration requires both a matching PATCH response and matching GET.
An unknown restoration outcome refuses success even if the server actually restored the value.

The body error remains the original error when restoration succeeds.
When both operations fail, `AggregateError` retains the body error and restoration error (lines 55–58).
Generated evidence contains stages, counts, and restoration status. It does not copy provider response secrets.
Arbitrary errors from the API or body are retained; callers must not serialize these errors as secret-free evidence without review.

## Executed checks

The independent command `node --test tests/astral-adoption/provider-redirect.test.mjs` passed six tests, with no failures or skipped tests.

The recorded GREEN log also contains six passing tests:
`/var/tmp/rom-010-astral-redirect-green.log`.
The recorded RED log shows an absent-module setup failure:
`/var/tmp/rom-010-astral-redirect-red.log`.
It does not demonstrate six behavioral regression failures.

Four additional synthetic probes ran through Node without source edits:

| Fault | Observed result |
| --- | --- |
| Activation GET differs from expected redirects | Body does not start; original redirects are restored. |
| Activation response contains malformed redirects | Body does not start; original redirects are restored. |
| Restoration GET differs after a successful body | Helper refuses success and records `restored: false`. |
| Restoration commits but acknowledgement is lost | Helper refuses success and records `restored: false`. |

An earlier disposable probe contained `return42`, which caused a body error in restoration cases.
The corrected probe used `return 42` and checked the exact restoration error. All four corrected cases passed.
These probes are review checks, not maintained regression tests.

## Tests to add

Maintain the four fault cases above before the provider gate.
Also test malformed initial responses, exhausted redirect capacity, preservation of regex entries, and existing exact callbacks with other entries.
Test restoration failure after a successful body explicitly. The maintained failure test currently combines body and restoration failures.

## Scope limits

Restoration replaces the whole initial redirect array. Concurrent writers could lose their changes (line 48).
The actual gate must use an exclusively owned fixture. This helper does not provide conditional provider updates or writer coordination.

The helper does not enforce API deadlines or handle process termination.
Its `finally` block does not prove recovery after SIGKILL, process exit, or an API promise that never settles.
The gate caller must supply bounded requests and cleanup ownership. Record any remaining uncertain restoration state as a failure.

This review proves synthetic callback admission and restoration behavior only.
It does not prove real login, browser behavior, provider integration, or ROM 0.1.0 release admission.

## Corrective maintained coverage

The maintainer added the missing fault categories without changing the helper.
The reviewer reran the maintained suite: 12 tests passed, with no failures or skipped tests.
The recorded expanded run is `/var/tmp/rom-010-astral-redirect-expanded-green.log`.

Maintained cases now cover activation mismatch, malformed activation, restoration mismatch, lost restoration acknowledgement, initial malformed data, exhausted capacity, and regex preservation.
Restoration failures are also tested after a successful body.
The exclusive-writer, bounded-request, and process-termination limits above remain unchanged.

## Actual provider schema correction

The actual trial's initial provider GET returned HTTP 200, but the original two-field redirect validator rejected its response.
The preserved `rom-actual-recovery-hmzXFq/execution.json` records one GET, no redirect mutation, no host target, and no browser success.
Its controls and host shutdown lists are empty. Endpoint vacancy and unchanged inputs are true.
This was an actual schema incompatibility before application launch, not a failed callback or mutation.

The corrected validator accepts the optional known `redirect_uri_type` values `authorization` and `logout`.
It preserves each original entry's exact metadata.
An exact logout URI does not satisfy authorization callback admission.
When existing entries use the typed schema, the appended callback explicitly uses `authorization`.
Unknown values still fail before PATCH.

The maintainer preserved a behavioral RED with one failed typed-schema case and 13 passing cases.
The reviewer independently reran all Astral Node tests: 22 passed, with no failures or skipped tests.
The fresh `rom-actual-recovery-Iy8pqs` witness contains 20 source hashes. All matched during independent review.

The [official API reference](https://api.goauthentik.io/reference/providers-oauth-2-retrieve/) confirms the provider retrieval endpoint.
Its rendered text did not expose the field enum during this review.
Independent enum verification from that rendered page is therefore not claimed.

No new source blocker was found in this narrow correction.
The provider owner must reconcile its reported uncertain stop acknowledgements before another owned window is admitted.
The reviewer did not start or stop a provider, application, or browser.
