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
