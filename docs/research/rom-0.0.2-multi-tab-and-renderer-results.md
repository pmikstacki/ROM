# Shared-session logout and generic table measurements

Date: 2026-10-04.

## Test boundary

The host tests use two pages in one Playwright BrowserContext. They share the real HttpOnly session cookie. They do not copy an Actor or inject successful API responses. The human provider completes authorization and consent. The first page opens a live task query. The second page reads the same Resource and then signs out.

The test requires the first page's live network request to terminate within five seconds after logout starts. Both pages must return to sign-in and remove all table cells within twenty seconds. The session endpoint must return the anonymous session. This checks browser delivery termination and visible data removal. It does not prove offline revocation or heap erasure.

These checks use the retained Studio-enabled binary at `/var/tmp/rom-studio-secure-files-acceptance/rom-demo`. Its SHA-256 is `60576302fec8f9d2439b3da34cd23ea8411c9277ce575c2bad9c2386986b9d7c`. No new native build or target was needed. The production release gate still builds the accepted checkout before acceptance.

## Harness diagnosis

The first attempt waited on Playwright `Response.finished()` for a streaming response. That wait reached the test timeout after both views had returned to sign-in. The timeout is retained in [the initial output](evidence/rom-0.0.2/multi-tab/response-finished-initial.log).

The corrected observer uses bounded `requestfinished` and `requestfailed` events for the exact live request. An initial four-case run passed on both browsers and both stores. The strengthened test observes termination before waiting for view clearing. A browser cancellation is a terminal result; it does not mean a mutation failed. No product correction was made from this harness timeout.

See [the initial bounded run](evidence/rom-0.0.2/multi-tab/browser-results.log) and [the stronger delivery run](evidence/rom-0.0.2/multi-tab/bounded-delivery-results.log). The latter records request termination and view-clearing times separately. The second tab clears through its logout flow. The first tab also has a periodic session check. Consequently, view clearing is bounded rather than simultaneous across tabs.

The stronger run passed all four browser/store combinations. Chromium reported request cancellation after 34 milliseconds on each store. WebKit reported request completion after 38 milliseconds on each store. Both pages cleared after 11.49 to 14.93 seconds. These measurements separate prompt delivery termination from the periodic session check in the other tab.

## Table measurements

A local descriptor fixture renders the actual generic ResourceTable with sixteen columns and custom codec renderers. The fixture contains either one hundred or five hundred rows. It performs no API requests. After rendering, the test removes all rows and checks that the table contains zero data cells.

| Browser | Rows | Data cells | Navigation to visible frame, ms | Clear to frame, ms |
| --- | ---: | ---: | ---: | ---: |
| Chromium | 100 | 1600 | 202 | 67.7 |
| WebKit | 100 | 1600 | 200 | 76 |
| Chromium | 500 | 8000 | 842.1 | 113 |
| WebKit | 500 | 8000 | 998 | 371 |

These are individual local samples with warm asset caches. Navigation includes module execution and frame scheduling. The clear measurement includes the click and subsequent browser frame. These figures are not isolated renderer CPU costs. The zero-cell result checks DOM retention, not garbage collection or heap reclamation. No database, parser or network performance conclusion follows from these samples.

All four samples passed. The full affected component suite passed thirty-six cases. See [the measured samples](evidence/rom-0.0.2/multi-tab/table-frame-results.log) and [the component regression](evidence/rom-0.0.2/multi-tab/components-regression.log).

The five-hundred-row case creates eight thousand cells. The result supports bounded page sizes for ordinary Studio views. It does not establish a universal latency budget or justify an additional virtualization dependency. A future large-table change should compare repeated samples under the same workload before selecting that dependency.
