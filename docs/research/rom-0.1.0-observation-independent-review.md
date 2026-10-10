# Independent observation review

Reviewed on 2026-10-07 against the [observation ownership contract](rom-0.1.0-ui-composition-admission.md#observation-ownership). Three reproduced defects require correction before observation API acceptance. The focused candidate suite passes 14 tests. Those tests do not cover these defects.

This review changed no product source or product tests. It ran Node probes while the parent owned the native verifier lease. It does not establish browser acceptance, original-consumer acceptance, or release readiness.

## Reviewed source and evidence

The review inspected `studio/src/observe.ts`, `studio/src/lib/observe/{controller,types,scope}.ts`, and `studio/tests/unit/observation-{controller,scope,public}.test.ts`. The controller SHA-256 was `995bde539f4b7015810dae212927859cff0a4593d39f691aeb5f82366850ec15`.

Complete reviewed file hashes are in `/var/tmp/rom-010-observation-independent-reviewed-source-hashes.txt`. Commands ran on host Node `v22.16.0` with `--experimental-strip-types`.

| Executed evidence | Result |
| --- | --- |
| `/var/tmp/rom-010-observation-independent-baseline.log` | 14 tests passed, zero failed. |
| `/var/tmp/rom-010-observation-independent-probes.log` | Three defect reproductions and two contract clarification probes completed. |

The baseline command was:

```sh
node --experimental-strip-types --test studio/tests/unit/observation-{controller,scope,public}.test.ts
```

## Required corrections

### P1: A subscriber exception suppresses authority loss

At `controller.ts:89`, denial publishes before calling `onAuthorityLost` at line 90. A subscriber exception interrupts that publication. `failure()` already cleared `active`, so the surrounding catch cannot recover through `owns()`.

The probe used a denied source and a subscriber that throws only for the denied notification. It observed `phase="denied"`, empty rows, and authority-loss callback count zero. This controller clears its own rows. The host's required authority invalidation and other private-view clearing do not run.

Preserve empty denied state before calling host code. Ensure subscriber failure cannot suppress the mandatory authority callback. Recheck the original ticket and scope before that callback. Do not invalidate a newer owner after notification reentrancy. Add regressions for a throwing first subscriber, a later subscriber, and a subscriber that rebinds before throwing.

### P2: A subscriber exception leaves a connection that never started

At `controller.ts:150–163`, `connect()` installs `active` before publishing `connecting`. Publication occurs outside the asynchronous source handler. If a subscriber throws, `start()` throws and source invocation never occurs. `active` remains set.

The probe observed `phase="connecting"` and zero source calls. Calling `retry()` could not recover because it requires `!active`. Repeated `start()` also cannot recover because `started` is already true.

Define subscriber exception behavior explicitly. Ensure failures leave no active controller or timer without owned work. Keep these programming errors separate from upstream denial or transient error classification. Add regressions for start, fresh notification, failure notification, initial subscription, and disposal cleanup. A throwing initial subscription currently installs its listener before it can return an unsubscribe function.

### P2: Notification reentrancy starts the newest source twice

At `controller.ts:245–252`, `rebind()` publishes idle state, then calls `begin()` without checking its original ticket. A subscriber can rebind again during that publication. The nested rebind starts the new source; the outer continuation then aborts and restarts it.

The probe rebound from `outer` to `inner` during the idle notification. It observed two `inner` source calls and one abort before disposal. The final scope remained `inner`; this probe did not establish wrong-owner row disclosure. It establishes an unnecessary source invocation and cancellation caused by an obsolete continuation.

Check the original epoch after publication and before restart. Add regressions for notification-driven rebind, hide, and disposal. Require one invocation of the surviving source and no continuation from the superseded operation.

## Contract clarifications

### Initial clock failure

`begin()` calls `clock()` at `controller.ts:211` without the failure handling used by retry timers. An initial `NaN` clock throws after `start()` sets `started=true`. The probe observed idle state and zero source calls after a second `start()`.

The public contract should state whether this is a synchronous programming error or a sanitized exhausted state. If synchronous failure is intentional, document recovery through explicit `retry()` or reconstruction. Keep initial-clock and timer-clock handling consistent. No claim is made that a defective clock can supply a valid elapsed-time guarantee.

### Retry elapsed time and stalled sources

`maxElapsedMs` is checked after failures and before reconnects. It does not abort an active source. A source that never yielded remained connecting after 30 ms with a configured 10 ms ceiling. This is a measured behavior, not proof of a violated active-stream deadline: observations normally remain open.

State whether the limit bounds reconnect admission or also initial connection acquisition. Do not imply it bounds the lifetime of a fresh observation. Require the source adapter to respect cancellation and provide its own connection bound if that remains outside this helper.

`AbortSignal` communicates cancellation to an operation. It cannot force an arbitrary async iterator to settle. Existing transport ownership and frame bounds remain necessary. [AbortSignal documentation](https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal).

### Trusted cloning, measurement, and safe codes

`types.ts` assigns shape validation and detachment to the host's `clone`, and exact byte counting to `measure`. Source rows are cloned and measured before publication. State reads and subscriber snapshots clone again. Those callbacks must produce detached, bounded values on every invocation.

Document whether clone and measure callbacks must be side-effect free. Current epoch checks prevent publication after callback-driven rebind. They do not make an arbitrary host clone trustworthy. Do not substitute ordinary JSON serialization for exact ROM wire values.

The classifier accepts any string code. The host must map errors to sanitized, bounded codes; the API must not suggest automatic redaction of classifier output. Consider a finite validated code contract or explicit byte limit if bounded public state includes error codes.

## Verified safeguards and limits

The baseline tests establish detached row snapshots, finite reconnect attempt count, denial row clearing with a nonthrowing subscriber, row and byte rejection, hidden abort and visible reconnect, and fencing of late canceled-source rows. They also cover abort-listener rebind, a throwing classifier, and a retry-timer clock defect.

Source review confirms exact principal comparison without delimiter identities. Scope capture freezes selected principal fields and generation. Explicit public scope cannot be inferred from missing private authority. Generation changes distinguish renewed authority. Source, clone, measurement, classifier, and timer continuations check current ownership before publishing rows.

No process-global cache or implicit document visibility listener was found. The host owns visibility integration and current-authority rebinding. A hidden view retains a labeled stale snapshot only within its existing scope. A same-scope rebind also retains rows; the design should explicitly distinguish this from authority-generation changes that clear private rows.

Use a monotonic clock for elapsed retry accounting. `performance.now()` is based on a monotonic clock; it differs from wall-clock time. Timer callbacks can run later than requested, so elapsed checks must remain authoritative. [Performance.now documentation](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now), [setTimeout documentation](https://developer.mozilla.org/en-US/docs/Web/API/Window/setTimeout).

The review did not execute browser visibility throttling, real transport cancellation, a Svelte lifecycle composition, or a second external consumer. It did not prove physical termination of sources that ignore cancellation. The three reproduced corrections need maintained behavioral RED→GREEN evidence before acceptance.

## Consumer traceability

This helper contributes to AP-UX-013 bounded private observation and AP-UX-007/008 usable progress and recovery. Generation-bound clearing supports AP-UX-023/024 context and fresh-versus-retained disclosure. It does not establish application source freshness, domain interpretation, or authorization policy by itself.

The maintained feedback audit has 47 classified user references and 33 AP-UX groups. Original-consumer acceptance remains open. Passing the 14 internal tests does not close any original-consumer observation.
