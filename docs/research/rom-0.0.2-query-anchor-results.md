# Native query anchors for Studio

The native query anchor route passed an actual TCP test with SQLite.
The route delegates to `Runtime::query_anchor`. It does not duplicate codec normalization in JavaScript.

The test creates a Resource with `FiniteF64`, queries it, and requests a moving boundary.
The boundary preserves `1.0` as a floating JSON number in both its filter and order value.
The next page accepts the boundary and returns an empty result. An invalid authentication token receives HTTP 403.

`ProjectedView` remains serialization-only. The HTTP route accepts a separate strict request DTO.
An anchor grants no authorization. The runtime validates the submitted query and normalizes the boundary through the registered codecs.

The browser retains the entire native anchor as canonical JSON. This representation survives `structuredClone` without changing floating-number categories.
It is separate from a journal cursor. Browser-side structural validation remains subject to independent review.

The first behavioral run failed with HTTP 404. The implemented route passed the same test.
Earlier compiler errors came from an incorrect test declaration, not a runtime defect.
The corrected fixture uses the public `FiniteF64` and policy APIs.

Evidence:

- [Initial fixture compile](evidence/rom-0.0.2/client/query-anchor-initial-compile.log)
- [Missing route](evidence/rom-0.0.2/client/query-anchor-red.log)
- [Successful actual TCP test](evidence/rom-0.0.2/client/query-anchor-green.log)
- [Scoped Clippy](evidence/rom-0.0.2/client/query-anchor-clippy.log)

Commands ran in the existing `rom-dev` container, with the shared release target and one Cargo build job.
This test does not establish actual Studio browser integration or adapter parity. Those remain separate release gates.
