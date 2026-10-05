# Independent Studio startup preservation review

Review date: 2026-10-05.
Base revision: `c66c470eb894b8449f9ab707d94179c88af09113` with the reviewed correction uncommitted.
Result: no material finding in the frozen correction.
The demonstrated startup defect is corrected within the reviewed host bootstrap scope.
Final native host restart, full local verification, complete production, artifact verification, and live preview acceptance remain separate gates.

The reviewer inspected source, public contracts, tracked diffs, and preserved synthetic test logs.
No repository file, private database, service, lock, browser, or build was changed by this review.
No private diagnosis IDs or values are reproduced here.

## Public contracts and correction

`Storage::load` returns `None` for an absent key and `Some(Row)` for stored state, including a tombstone with no live value.
The old host seed path replayed stable create receipts over current state.
Core authorization correctly refuses to disclose a historical live create outcome when the current row is tombstoned.
The correction does not change this denial, runtime idempotency, the storage contract, or any library implementation.

The generic host seed helper now performs an existence check through the same backing Storage used to construct Runtime.
It skips every existing row, including edited live rows and tombstones.
It does not decode a historical receipt, reconstruct prior state, invent a cached identity, or write a replacement.
Only an absent key follows ordinary Runtime.execute with the existing create command and stable seed identity.
Revision arbitration, current mutation authority, codec validation, reference validation, and atomic persistence remain runtime responsibilities.
Storage lookup errors propagate through `?`; they are not treated as absence or successful startup.

The host calls Runtime.establish_actor before inspecting seeded existence.
That public host integration applies current authority and runtime admission/lifecycle checks.
An already-closed runtime therefore cannot report success merely because all rows already exist.
The trusted host actor and definition policies are unchanged.
This is startup provisioning, not a public read endpoint or a bypass for session authentication.
No stored values or protected tombstone metadata are returned to the browser by the new existence path.

The Studio host retains a clone of its selected Storage and passes that exact instance into bootstrap.
The public Studio run/profile interfaces remain unchanged.
The `lib.rs` addition is a test-module declaration only.
The Studio startup module remains private to the demo crate.
The separate general demo startup path is not changed or certified by this correction.

## New dependent fixture on older stores

An older store can lack the new semantic showcase while its default required Task target is tombstoned.
That case is now handled before creating the optional synthetic fixture.
An existing showcase, live or tombstoned, is skipped unchanged.
An absent showcase selects only an actually stored live row among the three explicitly declared demo Task candidates.
A single shared Task seed constant supplies both template provisioning and candidate selection.
The selected exact target becomes a typed ResourceRef, and the ordinary runtime create still validates it.

When every candidate is deleted, the optional showcase is omitted.
No Task is resurrected to satisfy the fixture, and no existing showcase relation is rewritten.
The candidate loop is finite; lookup errors still propagate.
No network search, name inference, unrestricted public query, or arbitrary identity generation is introduced.
This is an explicit host fixture prerequisite decision rather than a type-specific library mutation path.

## Executed evidence inspected

The preserved initial `startup-red.log` failed during fixture setup with a relation Conflict.
It does not establish the intended tombstone replay failure.
After the fixture moved its required relation through ordinary runtime mutation, `startup-red-deleted-seed.log` failed on both SQLite and redb with Denied at restart.
This is the intended original defect reproduction.

The intermediate four-case GREEN record covers deletion preservation, closed runtime, and lookup failure.
`startup-upgrade-red.log` then records four additional prerequisite Conflicts for missing showcases with deleted targets.
`startup-upgrade-green.log` records all eight final focused tests passing.

The two real adapter reopen tests preserve the exact stored tombstone, revision, protected metadata, all-kind Row snapshots, journal heads, and state/event/receipt/effect counts.
They also preserve an edited live relation and repeat bootstrap without new writes.
The closed-runtime test checks exact Closed and unchanged counts on a fully seeded store.
The failed-lookup test checks exact Storage error and unchanged counts rather than silently creating or replaying rows.
Four upgrade cases cover a deleted default target and all targets deleted on both adapters.
They preserve Task snapshots, choose a live prerequisite only for a new fixture, and verify a repeated pass does not add writes.

The implementing agent's full demo test and Clippy log records 22 unit passes, passing integrations, and successful all-target/all-feature Clippy with warnings denied.
Four existing subprocess helper tests remain ignored across their integration binaries; they were not newly skipped by this correction.
These are inspected execution records, not independent reviewer executions.
The review does not establish a full local verifier or a new actual-host/browser acceptance result.

## Frozen source and remaining acceptance

| File | SHA-256 |
| --- | --- |
| `demo/src/studio_startup.rs` | `9772fcaed8212e85e15f717f3caf306f0c02fa179af1381dd1634e102b6c9878` |
| `demo/src/studio.rs` | `e722aafb574668b2752585db07607fa41dcaff25e672532d05ed3f827c336aaa` |
| `demo/src/studio_startup_tests.rs` | `c2f567ffbd358f1a2c5bfc47df62dee3f12c905416ac997e66442032236b4860` |
| `demo/src/lib.rs` | `c5ad89be63c3728fa1c6f4424070ae0c1b7885fe7460b16cf6065292e6ed6ae9` |

The implementing owner confirmed these sources are frozen.
Synthetic evidence remains under `/var/tmp/rom-003-isolated-resume-diagnosis/`.
An optimized isolated old-store startup probe was still reported in progress at freeze.
Its result must be recorded separately without publishing private identifiers or values.

Previously accepted artifacts and healthy primary preview evidence remain historical evidence.
This production bootstrap change cannot be covered by the earlier byte-identical runtime bridge.
The corrected binary requires current host startup acceptance and fresh release gates before the goal can be complete.
No completed-release declaration follows from this source review or the eight focused tests.
