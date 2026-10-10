# Public result invalidation regressions

Date: 2026-10-08. Status: executed candidate checks; release admission remains open.

Independent review found two unexecuted browser branches in the preceding 72-case candidate.
Source inspection suggested correct behavior. It did not establish browser acceptance of those branches.

Two maintained cases now use the actual native Resource pipeline:

- A publisher changes an observed Guide after the guest computes and exports revision one.
  The live revision-two projection clears the arithmetic result and removes the download control.
  Explicit recomputation yields 240; explicit export captures revision two and keeps private notes hidden.
  The authorized journal contains exactly the seed event and the publisher's new event.
- A guest changes selection while the proxy holds an actual export read response.
  Releasing the original response cannot publish stale bytes or restore the download control.
  The principal remains unchanged and no page error occurs.

The focused tests passed eight executions across SQLite/redb and Chromium/WebKit.
Existing behavior passed directly. No product failure or product correction is claimed.

The final matrices passed 40 executions per adapter, 80 total, on the same source candidate.
Type checks, builds, and owned proxy bind-failure cleanup also passed.
The only fixture change from the preceding candidate is consumer/tests/portal.spec.ts.
The native binary SHA-256 remains d7290ea6ec2c989934e9fdb6138c0c36eca575e880666c28b509a5f126f97e8a.

Evidence: /var/tmp/rom-010-portal-public-invalidation-full-witness.json.
Reports: /var/tmp/rom-010-portal-public-invalidation-full-sqlite/result.json and /var/tmp/rom-010-portal-public-invalidation-full-redb/result.json.
The earlier 72-case candidate, initial missing-interface failure, and corrected actor-body oracle remain preserved.

These cases support AP-UX-013/017/021/025/033 through an installed generic fixture.
They do not establish conditional-cache policy, original Astral Plane adoption, human usability, or release acceptance.
Independent review of this test-only increment remains separate from the preceding source review.
