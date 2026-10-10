# OpenRouter catalog deadline measurement

## Observed failure

The combined Host and lease-reader verifier failed in `catalog_delay_consumes_the_original_deadline`.
The adapter returned `DeadlineExceeded`, but the test's elapsed-time assertion exceeded 120 milliseconds.
The [failed execution](/root/ROM/.superpowers/rom-010-auth-lease-combined-verifier-20261009/result.json) retains the exact source and physical closure.
Its Node test stage passed 560 tests. The complete verifier did not pass.

## Investigation

The test started its elapsed-time measurement before constructing the HTTP client.
It constructed the 25-millisecond operation deadline after that client construction.
Therefore the measurement included preparation outside the deadline it intended to test.
Rust evaluates method-call operands in source order. See the [Rust Reference](https://doc.rust-lang.org/reference/expressions.html#evaluation-order-of-operands).

Temporary instrumentation preserved the original 25-millisecond deadline and 120-millisecond assertion.
The isolated test measured 15.748 milliseconds of preparation and 25.716 milliseconds of operation.
The parallel 19-test target measured 26.847 milliseconds of preparation and 26.417 milliseconds of operation.
Both commands passed and physically closed. Their [evidence](/root/ROM/.superpowers/rom-010-openrouter-deadline-instrumentation-execution-20261009/result.json) retains the measured output.
These observations prove the measurement mismatch. They do not identify the exact scheduling delay in the earlier failure.

The failed verifier's cgroup recorded CPU throttling. That aggregate measurement cannot attribute a delay to this individual test.
Tokio also documents that a timeout cannot preempt a future that does not yield. See [timeout](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html).
Paused-time tests require care with external IO and wall-clock checks. They do not replace this real TCP scenario.

## Correction and limits

Construct the client before starting the operation measurement. Keep the original deadline, delayed response, and elapsed-time threshold unchanged.
Production adapter code, retry rules, credentials, and request budgets remain unchanged.
Remove the temporary diagnostic output after retaining its evidence.

The corrected affected target passed 19 tests. The complete local verifier then passed, including 560 Node tests.
The [verified execution](/root/ROM/.superpowers/rom-010-auth-lease-combined-verifier-v2-20261009/root-terminal-review.json) records the source, outputs, and physical closure.
These results do not replace installed-browser, mixed-load, native SQLite qualification, or release acceptance.
A passing loopback test does not establish a production latency guarantee under arbitrary host scheduling.
