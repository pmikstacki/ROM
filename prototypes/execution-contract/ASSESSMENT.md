# Implementer assessment: execution ownership under cancellation

Assessment date: 2026-10-02. Evidence: executable prototype at commit `dbcdb7d`, its [source](src/main.rs), retained [lockfile](Cargo.lock), and [recorded run](README.md#executed-observation). The implementing agent ran the locked release binary, formatting check, and Clippy successfully; the parent agent reports independently repeating those checks successfully. No additional experiment was run for this assessment.

## Recommendation

Use **admission before submission, with the CPU job owning its permit and a supervisor owning completion independently of the caller**. Reject the waiter-owned permit approach for work that continues after caller cancellation.

These are two ownership arrangements exercised within one execution prototype, not competing complete ROM implementations. The recommendation concerns execution accounting and lifecycle; it says nothing about whether ROM's resource, transaction, or reaction model is correct.

## Which exercised approach is better?

| Approach | Observed behavior | Assessment |
| --- | --- | --- |
| CPU work owns the permit; supervisor retains completion | With two jobs started and blocked, cancelling both reply waiters leaves zero permits. Further submission returns `full`. Shutdown remains pending until the jobs are released, then receives both outcomes despite abandoned replies. | Recommended: admission accounting follows actual work lifetime. |
| Async reply waiter owns the permit — intentional negative control | Cancelling the waiters returns capacity while both original jobs remain blocked. A third CPU job starts even though advertised capacity is two. | Incorrect for this lifetime: cancellation makes the limit cease to describe running work. |

The negative control matters because both versions appear bounded before cancellation. The distinguishing event is loss of caller interest while CPU execution continues. Three Rayon workers and admission capacity two made the defect observable as three started, unreleased closures, rather than only an extra queued job.

The recommended arrangement also handled an unwinding CPU failure as an error, restored its permit, and completed a subsequent job. This supports the tested cleanup path; it does not establish that arbitrary plugin state is safe after a panic.

## Patterns worth carrying into ROM

- **Separate interest from ownership.** Dropping a reply means the caller stopped waiting. It must not silently terminate supervision or free capacity consumed by ongoing work. Apply the same ownership question to an action entering its commit phase; transaction safety still needs separate proof.
- **Acquire before submitting.** An explicit `full` result avoids creating another queued closure. A worker-thread count alone is not a queue limit. Bound ingress and retained results separately.
- **Give the permit to the longest-lived operation it accounts for.** Move it into the CPU closure and release it after computation returns or unwinds. Do not tie it to an async wrapper that may disappear earlier.
- **Keep completion observable.** The supervisor can record an outcome even when forwarding its reply fails. Treat an abandoned reply as a normal outcome, not a panic.
- **Share host-owned execution services.** Cloned handles reuse the configured runtime, pool, and capacity budget. No pool is needed per action or domain resource.
- **Drain actual work.** Close admission before draining. An empty set of interested callers is not evidence that computation has stopped.
- **Test lifetime boundaries with signals.** Start/release/completion channels and a deliberately incorrect control revealed the failure without relying on sleeps. Keep this approach for cancellation and shutdown conformance tests.

## What the pool comparison does and does not establish

The selected Tokio–Rayon composition produced correct checksum results while async timers progressed. In the recorded run all 20 timer observations overlapped active CPU work. These are observations from one workload, not a latency guarantee or speedup measurement.

The executable did **not** compare the same CPU workload executed directly inside Tokio async tasks, nor compare alternate executors. Keeping substantial CPU work off async workers is an architectural recommendation here, supported by the runtime separation; superiority over an inline implementation was not measured. Tiny, bounded field checks may remain synchronous inline work.

## Production gaps before reusing the pattern

This code should remain a probe. A production bridge needs continuous completion reaping; bounded request, payload, and retained-result memory; typed outcomes and stable job identity; explicit handling of unexpected supervisor loss; and an agreed shutdown deadline and unfinished-work policy. It also needs cooperative CPU cancellation where appropriate, a deliberate panic policy, and workload-based thread/admission budgets.

The demonstrated drain path does not make an unexpectedly dropped supervisor safe. The probe's blocked CPU closures are test gates, not a recommendation to perform blocking I/O on Rayon. It does not cover aborting panics, distributed capacity, fairness, atomic persistence, uncertain commit outcomes, idempotency, or recoverable reactions. Those remain distinct ROM requirements and validation tasks.
