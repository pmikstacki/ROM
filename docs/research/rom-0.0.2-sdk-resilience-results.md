# SDK resilience corrections

Independent review found six request and response defects. The coordinator reproduced their failure modes before changing the client.

| Defect | Correction |
| --- | --- |
| POST ignored cancellation when an injected transport kept running | A finite deadline arbitrates completion. An already-aborted caller does not reach the transport. |
| Body cleanup waited indefinitely for cancellation | Cleanup requests cancellation without awaiting the transport's cancellation promise. |
| An opened idle response survived the request timeout | The response reader listens to the request signal and cancels its body. |
| A mutation accepted an unrelated revision or live delete result | Validation binds the result to the frozen expected revision and operation. |
| Floating wire metadata became integer metadata | Integer validation checks the retained lexical category on the original parsed container. |
| Queries ignored the requested page size | Queries and live snapshots apply the smaller requested and client limits. |
| Parser admission ignored the configured byte policy | Body reading and parsing use the same admitted bound. |
| An excessive timer value became a short timeout | Startup rejects durations above the supported signed 32-bit timer range. |
| Concurrent observations had no admission bound | The client defaults to eight observations. Cancellation releases the observation slot. |

The mutation revision check permits the original revision for a no-op and the next revision for a change.
A malformed success response remains an unknown outcome. The original request and idempotency key remain available for explicit recovery.
Receipt replay still makes a server request and rechecks current authority. The browser does not manufacture a cached success.

The query page limit is captured before waiting for a response. Observation requests and anchor requests are copied before transport submission.
Callers cannot change the admitted query during the asynchronous wait.

The latest coordinator run passed 48 SDK tests. Svelte type checking reported zero errors and zero warnings.
The independent query-anchor tests were being added in parallel. This count identifies that run, not a fixed final release total.

Evidence:

- [Initial request failures](evidence/rom-0.0.2/client/request-resilience-red.log)
- [Integer metadata failure](evidence/rom-0.0.2/client/integer-metadata-red.log)
- [Observation admission failure](evidence/rom-0.0.2/client/observation-admission-red.log)
- [Idle reader cleanup failure](evidence/rom-0.0.2/client/reader-cleanup-red.log)
- [Successful coordinator unit and type checks](evidence/rom-0.0.2/client/request-resilience-final.log)
- [Independent SDK review](rom-0.0.2-sdk-review.md)

The fixtures exercise injected browser transport seams. They do not establish actual host or provider behavior.
Dual-browser application tests and real-host acceptance remain separate release gates.
The canonical query-anchor string preserves native floating values across cloning. Ordinary cloned projections do not provide that guarantee.

## Initial revisions and actual provider redirects

A later source review found an incomplete revision check when the frozen request had no prior row.
The native execution path requires the stored prior revision to equal the submitted expected revision.
Without a prior row, a non-deleting mutation starts at revision one. A no-op delete starts at zero.
The client now checks those initial revisions. Its test fixtures retain large revision values for read tests, not impossible initial creates.

The new regression failed before the correction. The subsequent full frontend gate passed 52 SDK tests, 16 model/auth cases, and 28 browser cases.
Type checking reported zero errors and zero warnings.
The browser cases still use API fixtures; actual native host acceptance remains separate.

Actual-host Chromium testing also found a provider-fixture redirect problem.
The fixture's `form-action 'self'` policy blocked the cross-origin callback after consent.
The fixture now permits only its explicitly configured callback origin in addition to itself.
The origin comes from host configuration, not incoming browser parameters. No wildcard was added.
Three real provider tests passed, including the new CSP header assertion.
Actual browser confirmation of the correction belongs to the host acceptance worker.

Evidence:

- [Initial revision regression](evidence/rom-0.0.2/client/initial-revision-red.log)
- [Full frontend acceptance after revision correction](evidence/rom-0.0.2/client/initial-revision-frontend-green.log)
- [Provider CSP regression](evidence/rom-0.0.2/client/provider-csp-red.log)
- [Provider CSP correction](evidence/rom-0.0.2/client/provider-csp-green.log)

## Authorized action metadata subsets

The integration reviewer found that the client required every visible action name to have a visible input descriptor.
Native discovery intentionally omits an input descriptor when it would disclose a hidden referenced Resource kind.
The action name can remain permitted. The browser must accept this authorized subset.

The client now rejects unknown or duplicate input descriptor names without requiring a complete name-to-input mapping.
Studio creates action forms from the supplied input descriptors only.
A missing descriptor does not grant invocation authority or disclose the hidden target.

The new regression failed before the correction. The complete SDK suite then passed 64 tests.
The earlier counts in this report identify their respective source slices.

- [Hidden input descriptor regression](evidence/rom-0.0.2/client/hidden-input-descriptor-red.log)
- [Successful SDK acceptance](evidence/rom-0.0.2/client/hidden-input-descriptor-green.log)

## Native metadata admission limits

The client now rejects six descriptor forms that the native registration contract cannot produce.
They are empty or oversized enums, nested optional presence, repeated nullable wrappers, scalar optional presence, and oversized UTF-8 codec names.
These checks preserve the native shape contract before Studio selects a renderer.

All six regression cases failed before the correction. The subsequent complete SDK run passed 70 tests.
The successful type check reported zero errors and zero warnings.
This evidence covers injected descriptor responses, not deployed HTTPS authentication.

- [Metadata admission regressions](evidence/rom-0.0.2/client/metadata-shape-bounds-red.log)
- [SDK acceptance](evidence/rom-0.0.2/client/metadata-final-unit.log)
- [Type check](evidence/rom-0.0.2/client/metadata-final-typecheck.log)
