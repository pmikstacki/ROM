# Authentication evidence for the load fixture

Date: 2026-10-09. This change prepares attribution for the next mixed workload.
It does not establish a successful load result.

## Gap and correction

The Host supports optional authentication diagnostics. The load fixture did not enable them or retain a final snapshot.
Therefore a failed workload could lack the first observed authentication failure category.

The fixture now enables diagnostics. The existing limits remain 32 sessions and eight authentication jobs.
It retains a Host clone for observation, then awaits Host shutdown and reaction-worker completion.
It writes `load-final-authentication-proof.json` after that lifecycle, including a failed lifecycle.
Listener binding occurs before Host construction and worker startup. A bind failure is a pre-lifecycle admission failure.
It does not produce a final lifecycle snapshot.

The proof contains fixed operation and stage counters, bounded failure categories, and the combined lifecycle result.
It does not contain credentials, identity values, request payloads, or arbitrary error text.
Its fixed counters are diagnostic evidence, not an authorization decision or a load acceptance verdict.

## Evidence contract

The writer limits the proof to 256 KiB, creates it with mode `0600`, and refuses an existing destination.
An evidence failure cannot replace the original lifecycle failure.
If the lifecycle succeeds but evidence fails, the fixture fails.
The proof records `lifecycle_result`, because the combined result can contain a Host or worker failure.

## Executed checks

The initial regression run failed all three tests before evidence was written.
The correction passed the same three tests against a real Host after shutdown.
The complete fixture suite first passed 23 tests.
The [execution directory](/root/ROM/.superpowers/rom-010-load-auth-evidence-20261009/) preserves the failing and successful logs.

These tests establish private bounded serialization, preservation of failure, and refusal to overwrite existing evidence.
They do not establish real-provider attribution under load or physical process closure for a future workload.
A fresh binary, source fence, provider window, and mixed workload remain necessary.

Independent review identified a startup gap: listener failure could follow worker startup.
The fixture now binds before Host construction and worker startup.
An actual occupied-listener control verifies `AddrInUse`, no final lifecycle proof, and an unchanged pending reaction ledger.
Its first attempt failed during database ownership admission because the test retained its seed writer.
The corrected test closes that writer before the runner opens the database.
The final fixture suite passed 24 tests. The earlier failed control remains in the execution directory.
