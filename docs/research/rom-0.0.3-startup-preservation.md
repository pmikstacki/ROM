# Preserve seeded Resource changes across demo restart

A later preview check found a demo bootstrap failure after deletion of a seeded Resource.
The stable create receipt still existed. Its earlier result contained a value.
The current Resource was a tombstone. The core correctly denied disclosure of that earlier positive result.
The demo treated that refusal as a startup failure.

Private copied databases reproduced the failure with both earlier and current binaries.
Fresh databases started with the same admitted profiles and assets.
A direct diagnostic copy mutation failed storage integrity admission. It is not evidence of successful recovery.
Original Resource values, tombstones, profiles, and attachments were not restored or rewritten.

The correction belongs to the trusted application bootstrap, not the core disclosure contract.
An existing Row means that provisioning already occurred. This includes a tombstone.
Bootstrap must leave that Row unchanged. Only an absent key receives the ordinary create action.
Current host admission and storage failures must remain visible.
No missing editor, authorization failure, or unknown commit result is converted into successful mutation.

The [RED regression](evidence/rom-0.0.3/startup-preservation/red.log) fails on both actual database adapters.
It covers initial seed, ordinary edit, ordinary delete, shutdown, reopen, and repeated bootstrap.
The corrected run passed. Final release acceptance remains pending.

The first fixture attempt hit reference-restrict before deletion. Its separate log remains preserved.
The corrected fixture moves the dependent reference before deletion and reproduces the intended Denied replay.
An older store also lacks the new showcase fixture. Its initial reference must select a live existing Task.
If every seeded Task is deleted, the optional new fixture must remain absent.
Existing Resources, including an existing showcase, remain unchanged.

The corrected [GREEN matrix](evidence/rom-0.0.3/startup-preservation/green.log) passes eight tests.
Both adapters preserve edited and deleted seeded Resources, journal heads, and state/event/receipt counts after reopen.
Both adapters cover a missing dependent fixture with a live fallback and with every seeded Task deleted.
Closed-runtime and lookup-failure checks retain visible errors without creating rows.
The [dependent-fixture RED](evidence/rom-0.0.3/startup-preservation/red-new-dependent-fixture.log) retains the earlier reference conflict.
These focused results do not replace the corrected complete producer or preview acceptance.

The [full demo and Clippy checks](evidence/rom-0.0.3/startup-preservation/demo-full-test-clippy.log) passed.
The [full local verifier](evidence/rom-0.0.3/startup-preservation/full-check.log) passed after the production correction.
The [independent review](evidence/rom-0.0.3/startup-preservation/review.md) found no material issue in the correction.
The earlier application identity bridge cannot accept this changed native binary. Fresh application acceptance remains required.

The [optimized copied-data probe](evidence/rom-0.0.3/startup-preservation/copied-upgrade.json) also passed.
All 13 existing Rows remained unchanged, including the deleted Task.
Provisioning added only the absent showcase, one event, and one receipt.
A second process restart returned HTTP 200 with unchanged counts.
This private development binary is not the final clean-source release binary.
