# Studio filter workspace

The owner selected quick-filter popovers and a shared Filters/Details sidebar on 2026-10-04. Existing autonomous release authorization covers implementation.

Keep one descriptor-driven filter draft. The popover and full sidebar edit that same draft. Applied chips describe the active query, not pending edits.
Use a pinned MIT SVAR builder fork. Replace its controls, numeric parsing and theme with ROM codecs and shadcn primitives/tokens.
Preserve the upstream license, original source hashes and explicit adaptations. Do not claim the entire upstream query language works in ROM.

The current contract supports conjunctions and eq/ne/lt/le/gt/ge. Reject OR, substring operators, unknown fields and predicate overflow before sending a query.
Preserve false, zero, empty strings, null, absence and exact integers. Field choices come only from current authorized descriptors.

Use compact applied chips, Filters, secondary refresh/live controls and one primary Create action above the table. Keep moving pagination below the table.
The full sidebar has Filters and Details modes. Closing or switching modes must not remount either draft. One page-size state serves both presentations.
Filter edits do not query automatically. Apply validates then updates the active query. Discard edits restores the applied query; Clear filters is distinct.
The sidebar becomes a keyboard-accessible Sheet on narrow screens. Popover and Sheet must close with Escape and return focus.

Applying a query must retain an open Resource draft when the same authorized Resource remains readable. Re-read it under current authorization.
A refreshed revision makes its draft stale. A denial clears the projected Resource; retained old projections must not bypass current authorization.
Descriptor or session changes invalidate obsolete filter edits. Unknown mutation retries retain their original frozen identity.

This change adds no transport, controller or repository per Resource kind. Repeat both persistence adapters, external custom renderer and full release gates.
