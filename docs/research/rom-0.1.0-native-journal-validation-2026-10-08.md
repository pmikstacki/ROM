# Native journal validation before public activation

Date: 2026-10-08. Status: candidate validation passed; public activation and release acceptance remain open.

## Scope

SQLite and redb have candidate native layouts with keyed journal positions and scalar metadata.
The shared current-format marker remained 10 during these trials. The candidate physical format was 11.
These results do not establish production throughput or complete 0.1.0 acceptance.

The large fixtures contain valid JSON metadata with whitespace padding to 129 MiB.
They exceed the default 128 MiB byte budget. An explicit 192 MiB profile permits their validation.
Padding tests physical input admission. It does not represent a large application workload.

Both tests cover default-budget refusal, custom-profile reopen, migration and retention.
They compare canonical state and restore fencing. Rejected maintenance must preserve source bytes and invoke no converter or publication callback.

## Findings and corrections

The first SQLite trial passed. The first redb trial failed a test assumption about logical file size.
The retained redb file had 537,927,680 logical bytes. Its allocated blocks represented 268,636,160 bytes.
The redb allocator can reserve a larger logical range than its current allocated data.

The corrected test uses a streaming length and SHA-256 witness with a 64 KiB read buffer.
It retains the finite 1 GiB filesystem bound. The logical input budgets remain unchanged.

The next trial exposed a product defect: rejected startup changed the redb source file.
The writable engine can trim unused allocation when its handle closes.
A small regression reproduced the change before the correction.

Existing-state admission now uses a read-only transaction before the writable engine opens.
The same validator checks the opened transaction. This avoids separate admission semantics.
An unclean database can need recovery on a private copy; the original remains unchanged during inspection.

Successful ordinary writer reopen can change engine allocation metadata.
The large test checks that writer lifecycle on an explicit copy.
It separately requires unchanged original bytes after refusals, migration and retention.

## Executed bounded trial

The final runner completed both cases in sequence.

| Adapter | Exact selected test | Result | Test duration |
| --- | --- | --- | --- |
| SQLite | `native_journal_maintenance_tests::candidate_valid_padded_header_requires_larger_than_default_profile` | One passed; no failures or ignored selected tests | 10.65 s |
| redb | `native_journal_tests::maintenance_tests::candidate_valid_inventory_above_default_cap_preserves_custom_profile` | One passed; no failures or ignored selected tests | 112.68 s |

Each target had a 1 GiB memory limit, zero swap, two-CPU quota and 128-process limit.
Each case used a fully allocated 1 GiB ext4 image in a private mount namespace.
The target deadline was 165 seconds. The namespace wrapper deadline was 180 seconds.

The redb memory limit recorded 2,459 pressure events. Neither case recorded OOM or an OOM kill.
This does not establish spare memory capacity or an independently measured peak.
The control Node process was outside the target cgroup and used bounded inputs.

Both cases completed explicit unmount, owned-process drain and verified loop detach.
The runner retained both images. All 256 product-source witnesses and 133 harness witnesses matched after execution.

The final evidence is under `.superpowers/rom-010-native-large-run/evidence/run-3dd18dde8b32bbadb99b7b2a/`.
The root summary is `/var/tmp/rom-010-native-large-phase6-root-result-20261008.json`.
The earlier failed images and raw logs remain available. These private paths are local evidence, not packaged documentation assets.

## Remaining release gates

Public constructors must initialize the physical layout, not only change its marker.
Archive readers and writers must support the coordinated 7/11 pair while retaining supported historical pairs.
Fresh copies of genuine native 8 and 10 fixtures must pass upgrade and downgrade-refusal checks.

The unchanged application seed and mixed-load tests remain separate requirements.
The full verifier, packaged-consumer checks and clean-source release artifacts must pass after integration.
