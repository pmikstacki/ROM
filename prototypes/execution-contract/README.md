# Disposable Tokio–Rayon execution-contract probe

**Prototype only.** This executable answers whether CPU capacity and lifecycle can remain correct when async callers disappear. It does not implement ROM or its resource model. No database, network service, persistence, container configuration, or runtime installation is involved.

The host explicitly creates a two-worker Tokio runtime and one shared three-thread Rayon pool. Cloned host handles share the same pool and semaphore. CPU admission is **two**, deliberately lower than the Rayon thread count so the negative control can demonstrate actual concurrent over-admission rather than merely queued work.

## Run

From the checkout containing this prototype, using a compatible Rust toolchain:

```sh
cargo run --manifest-path prototypes/execution-contract/Cargo.toml --release --locked
```

On the configured development host, while the dedicated worktree exists:

```sh
nixos-container run rom-dev -- sh -lc 'cd /workspace/ROM/.worktrees/execution-contract/prototypes/execution-contract && cargo run --release --locked --quiet'
```

The [manifest](Cargo.toml) pins Tokio 1.41.1 and Rayon 1.12.0. The retained [lockfile](Cargo.lock) resolves rayon-core 1.13.0 and the complete dependency graph. These are probe versions, not ROM's production release selection. This exact lockfile built with Rust 1.82.0 / Cargo 1.82.0. Build outputs are ignored locally.

Checks used:

```sh
cargo fmt --manifest-path prototypes/execution-contract/Cargo.toml -- --check
cargo clippy --manifest-path prototypes/execution-contract/Cargo.toml --locked --all-targets -- -D warnings
```

Correctness assertions execute in the binary; there is no separate test suite duplicating them. An assertion failure exits unsuccessfully. A broken lifecycle could hang; an external watchdog such as `timeout 30s` may guard a previously built executable. A watchdog timeout is not used as a correctness oracle.

## What the executable proves in this run

| Scenario | Control and observable result |
| --- | --- |
| Shared ownership and overflow | Two CPU jobs send start signals, then wait on independent release channels. A third submission returns `full` before spawning; submission count stays two. Cloned hosts share pool and semaphore pointers. |
| Caller cancellation | Abort both Tokio tasks awaiting CPU replies, and join the cancellations. Both CPU jobs remain unreleased; available permits remain zero and another submission is rejected. |
| Shutdown and abandoned replies | Close admission and explicitly poll the drain future once. It is pending while CPU work remains blocked. Release both jobs; drain then receives both successful outcomes, records undeliverable replies without panic, and restores capacity. |
| CPU unwind | Inject `resume_unwind` inside a CPU job. The bridge catches it, reports `cpu panic`, releases capacity, and successfully runs a subsequent job. `resume_unwind` exercises unwinding without changing the global panic hook or printing an expected panic. |
| Negative control | Deliberately put permits inside cancellable async waiters instead of the CPU closures. After aborting the waiters, both original CPU jobs are still blocked but a third CPU job starts. This detects the advertised bound-two violation. All three jobs are then released and drained. |
| Async timer observation | Run two deterministic CPU checksum jobs while awaiting 20 Tokio timer deadlines, spaced 2 ms apart. Compare both outputs with sequential reference values. Print timer lateness, total observed job time, and the number of timer samples taken while CPU work remains active. No latency threshold is asserted. |

Start, release, and completion channels establish the ordering of every correctness assertion. No sleep duration or yield count is used to assume a CPU job has started, completed, or been cancelled. Timer waits occur only in the observational timing scenario.

The correct bridge owns a CPU permit inside the Rayon closure, releases it after the job returns or unwinds, and sends a completion to an independently retained Tokio supervisor task. The supervisor forwards a reply if its receiver remains interested. Cancelling a reply waiter does not cancel the supervisor. The intentionally incorrect bridge keeps that same permit in the reply waiter; its cancellation makes the accounting diverge from actual CPU work.

## Executed observation

Executed on 2026-10-02 in native `rom-dev`: Linux x86-64, AMD Ryzen 7 7700, 16 logical CPUs visible, Rust 1.82.0, optimized release profile. Formatting and Clippy with warnings denied passed. The final locked run exited 0:

```text
PASS shared host pool; capacity=2; overflow rejected before spawn
PASS cancelling both reply waiters retains both running CPU permits
PASS shutdown remains pending until CPU release; abandoned replies do not panic
PASS CPU unwind reported as error; permit restored; subsequent job succeeds
PASS negative control exposes violation: 3 CPU closures started/unreleased with advertised capacity=2
PASS CPU outputs match sequential reference; 20 async timer deadlines observed
OBS timing only: two CPU jobs elapsed=68ms; timer lateness p95=1185us max=1206us; samples with CPU running=20/20 (no performance threshold)
PASS all supervised probe work completed before host shutdown
```

The timing sample uses 120 million checksum iterations per job and nearest-rank p95 across 20 samples. Sequential reference computation occurs before timing starts. The job elapsed value includes completion delivery to the async waiter. These are one-run observations on a shared host, not throughput measurements, an SLO, a speedup claim, or proof of behavior on other machines. The overlap counter reports whether CPU work remained active at each timer observation; it does not force that overlap through timing assertions.

## Limits and conclusion

The observed result supports **pre-submit admission owned by actual CPU work, with supervision independent of caller replies**. The negative control shows why releasing capacity when an async waiter disappears is incorrect.

This small bridge is deliberately incomplete. It only handles `u64` results and static error labels, retains completed supervisor results until draining, and has no general ingress/payload/result-retention budget. A long-lived service would need those limits and ongoing result reaping. Its explicit drain path is tested; dropping the supervisor unexpectedly is not a safe lifecycle contract. Channel-blocked Rayon jobs are deterministic probe gates, not recommended production I/O placement.

The probe does not establish transaction atomicity, revision conflicts, idempotent actions, durable reactions, cross-process coordination, scheduling fairness, hard cancellation, or production shutdown deadlines. CPU computation is not forcibly interrupted. Unwinding handling does not recover `panic=abort` or prove arbitrary plugin state remains valid after a panic. Resource semantics still require separate ROM conformance evidence.
