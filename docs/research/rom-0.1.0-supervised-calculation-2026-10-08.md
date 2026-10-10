# Supervised calculation boundary

Date: 2026-10-08. Status: implemented source candidate; external behavior tests and affected Clippy checks passed.

## Investigation and primary sources

The existing Runtime owns the Rayon pool supplied to Builder::build.
Runtime::io acquires finite admission and tracks blocking work until the actual closure ends.
Runtime::shutdown observes that work count. Cancelling a waiter does not release the work's permit.
The AI ReadContext has bounded reads and queries, but no calculation interface.

[Tokio 1.53.1](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html) states that started blocking tasks cannot be aborted.
Its documentation recommends bounded CPU admission and identifies Rayon as a suitable CPU executor.
[Rayon 1.12.0](https://docs.rs/rayon/1.12.0/rayon/struct.ThreadPool.html#method.install) specifies execution inside the selected pool.
Both versions match the root lockfile. This documentation review does not replace ROM behavior tests.

## Selected contract

Runtime::calculate is a generic, trusted synchronous calculation boundary.
It uses the existing Runtime::io lifecycle and its finite io_jobs admission.
The closure runs inside the Runtime's exact Rayon pool. No second pool or admission controller is created.
Full admission returns Error::Overloaded. No unbounded waiting queue is added.

The closure receives no Runtime or actor handle. This interface does not grant Resource access or commit permission.
Trusted application code can capture other handles; this interface is not a sandbox.
Calculations must terminate and must not wait for nested Runtime work.
One available I/O permit rejects nested admission as overloaded.
With additional permits and one CPU thread, blocking on nested Runtime calculation can deadlock that thread.
Joined or scoped child work must finish before the closure returns.
Detached child threads or Rayon tasks are not tracked by this Runtime's lifecycle.

Catch a calculation panic inside the CPU closure. Return Error::Panicked without closing the complete Runtime.
This preserves the existing action calculation's panic isolation.
The error carries no panic payload. The ordinary process panic hook can still write diagnostics.

An aborted or timed-out caller stops waiting only.
Its running calculation retains admission and shutdown ownership until completion.
A permanently blocked trusted closure can therefore prevent shutdown from completing.
No thread preemption, deadline guarantee for arbitrary code, or forced cancellation is claimed.

ReadContext can later delegate to this boundary under its existing ExecutionDeadline.
The AI host must still verify current grants before publishing results.
This adapter step remains separate from core implementation and verification.

## Alternatives

| Approach | Decision and reason |
| --- | --- |
| Detached host pool and separate semaphore | Reject: it introduces another lifecycle and does not participate in Runtime shutdown. |
| New Runtime CPU pool | Reject: it duplicates the application-owned shared execution pool. |
| Direct Tokio CPU execution | Reject: it bypasses the configured Rayon pool. |
| Existing tracked I/O plus selected Rayon pool | Select: it retains admission, cancellation ownership and shutdown through the established lifecycle. |

## External test evidence

The independent maintenance application tests both actual database adapters.
The tests require the exact selected CPU thread, retained capacity after abort and timeout, and shutdown drain.
They also require rejection after shutdown and successful work after a caught panic.
The pending source is examples/maintenance-portal/tests/calculation.rs.
The corrected external fixture failed with E0599 for the missing Runtime::calculate method only.
Evidence: `/var/tmp/rom-010-supervised-calculation-corrected-api-red.log`.
The first run also called a nonexistent SQLite constructor. That fixture error was corrected; its separate failure log remains preserved.
Product edits started after the identity worker confirmed its source fence and released the freeze.

All three external tests passed. Each test executes on actual SQLite and redb stores.
The one-thread calculation ran on the exact pool thread captured before Runtime construction.
Abort and timeout retained the single I/O permit, refused additional admission, and kept shutdown waiting until release.
The panic case returned Error::Panicked, kept Runtime ready, and admitted the next successful calculation.
Core all-target Clippy and all-feature consumer all-target Clippy passed with warnings denied.

- Tests: `/var/tmp/rom-010-supervised-calculation-green.log`
- Core Clippy: `/var/tmp/rom-010-supervised-calculation-core-clippy.log`
- Consumer Clippy: `/var/tmp/rom-010-supervised-calculation-consumer-clippy.log`

The native command used Rust/Cargo 1.99, two jobs, incremental compilation disabled, and locked offline dependencies.
Independent source review confirmed the inherited admission and shutdown ownership.
It identified the nested-work and detached-child limitations now documented above.
That review did not execute an independent native test or prove AI integration.
A complete verifier on the combined source remains required before integration.

These checks do not establish AI flow integration, production load bounds, or release acceptance.
