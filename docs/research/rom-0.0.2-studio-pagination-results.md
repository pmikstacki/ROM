# Studio moving pagination and draft revision guard

Date: 2026-10-03. This trial closes the separate sort-field and moving-pagination gaps in Task 5.

## Query behavior

The filter field and sort field are independent. The sort picker includes scalar fields and excludes lists and maps. Optional and nullable scalar fields remain available. The query editor emits the shared QuerySpec and resets page history when the user changes the query.

First, Previous, and Next are moving-view operations. Each operation fetches current authorized rows. The controller retains at most 128 previous query boundaries, not previous row snapshots. First remains available when the older history is no longer retained.

Next calls `client.anchor(query, lastRow)` once. The generic native `/query/anchor` endpoint applies the Resource's codec and returns a native boundary. The SDK retains that boundary as the typed QueryAnchor envelope. Its canonical JSON preserves normalized float tokens and custom codec values. The UI neither edits nor reconstructs it. A query anchor is navigation data; it grants no authority and is not a journal cursor.

The boundary request and page request have navigation guards. A late anchor cannot move another Resource's view. Paging cancels the previous live subscription. If observation was active, it opens a fresh authorized subscription for the new page. Changed filters reset the history. Live updates retain no stable database snapshot.

## Draft revision behavior

An integration review found that retaining an editor draft while updating the selected live revision could submit that draft with a newer expected revision. This could bypass the intended conflict protection.

ResourceDetails now captures the draft's base revision. A later Resource revision preserves the draft but disables its mutations and displays a reload instruction. Explicit reload discards the draft and opens the current revision. Actions and deletion use the same captured revision. The server still enforces its revision condition; the browser guard improves author feedback.

The details component has a separate responsibility from the Resource list and query page. Its test fixture is excluded from the ordinary production build.

## Executed checks

The [retained gate log](evidence/rom-0.0.2/frontend-pagination/gate.log) records:

- 32 SDK tests passed.
- 16 application/auth tests passed.
- Diagnostics: zero errors and warnings.
- Production build passed.
- 28 browser tests passed: 14 in Chromium and 14 in WebKit.

New controller tests cover Next and Previous refetch, filter reset, late-anchor rejection, bounded history, First after history eviction, and live cancellation/reopening. The browser pagination test uses the real SDK and a native-format anchor transport fixture. It checks Next, Previous, and the number of query and anchor calls. Other browser tests check independent sort fields, exclusion of collection sort fields, retained drafts, stale submission blocking, and explicit reload.

The browser transport responses are explicit fixtures. Actual Rust host and human OIDC acceptance remain separate. These checks do not establish a stable snapshot, a benchmark result, or production readiness.
