# Installed browser intent storage experiment

Date: 2026-10-07. This is candidate evidence, not release admission.

The [primary-source investigation](rom-0.1.0-browser-intent-storage.md) defines the storage contract and its limits.
This experiment uses the public `rom-studio/recovery` export in an installed consumer.
It does not import private implementation modules.

## Reproduced failures

The initial declaration failed type checking because the public store export was absent.
Evidence: `/var/tmp/rom-indexeddb-contract-red-20261007/type-check.log`.

A later browser regression reproduced an invalid write deleting the accepted record.
Both engines failed the existing public client case when `compareExchange` received `undefined` instead of a record or explicit `null`.
Evidence: `/var/tmp/rom-indexeddb-undefined-red-20261007/browser.log`.
The implementation now rejects that argument before it opens a write transaction.
A stored `null` is malformed data; it is not a missing record.

The first retry encountered four type errors in the concurrent controller candidate.
Evidence: `/var/tmp/rom-indexeddb-undefined-green-20261007/type-check.log`.
That run did not reach browser execution and is not successful storage evidence.

## Executed candidate

Command:

```sh
ROM_PUBLIC_CONTROLS_BROWSER=1 ROM_WEBKIT_EXECUTABLE=/var/tmp/rom-studio-webkit-2359/pw_run.sh node studio/tests/public-controls/verify.mjs /var/tmp/rom-indexeddb-lifecycle-green-20261007
```

Locked installation, source comparison, type checking, build, and browser execution completed with exit code 0.
The report contains six passed cases per engine: Chromium and WebKit.
There are twelve total consumer cases, not twelve separate storage tests.
The actual IndexedDB experiment runs inside each engine's existing public client case.

The experiment verifies competing writers produce one CAS winner.
It rejects stale versions, malformed arguments, and writes above the byte or slot limits without replacing accepted records.
Close and reopen preserve exact payload bytes, including a large integer literal, `false`, and `null`.
Deleting a record makes its capacity available again.
Closing the store aborts an outstanding transaction; the attempted replacement remains absent after reopen.
A version upgrade closes the previous connection and prevents further work through its store handle.
Reading a malformed stored `null` rejects the record.

Two additional unit cases pass for bounded configuration and missing IndexedDB capability.
They do not simulate database behavior.

Results and source/log hashes:

- `/var/tmp/rom-indexeddb-lifecycle-green-20261007/result.json`
- `/var/tmp/rom-indexeddb-lifecycle-green-20261007/storage-review.json`
- `/var/tmp/rom-indexeddb-lifecycle-green-20261007/browser-results.json`

## Remaining acceptance

The verifier reports `authoring_checkout`; this is not clean-source release admission.
The installed controller source matches the candidate at capture time, but this fixture does not test the full App session workflow.
The experiment does not establish crash durability, persistence permission, quota failure, eviction resistance, or physical-device behavior.
Blocked-open notification and quota errors still need direct failure injection.
Current authorization remains independent of stored bytes.
Original Astral Plane consumer acceptance, independent review, and the full local verifier remain open.

## Queued-open timeout and test sensitivity

A real initial upgrade is held by bounded requests while the helper's queued open reaches its deadline.
The fixture releases the upgrade and attempts version 2.
The subsequent upgrade succeeds only if the helper closes its late connection.
This exercises open timeout and late-success cleanup without a fake IndexedDB implementation.

The working-source retry passed twelve installed consumer cases across both engines.
Evidence: `/var/tmp/rom-indexeddb-open-deadline-final-green-20261007/result.json`.

An isolated candidate copy removes only the late-success `close()` call.
It changes one implementation file; no working source was changed for that mutation.
The same consumer fixture passes against the unmodified candidate and fails against the mutated copy.
Both engine failures contain `Timed-out late open leaked a blocking connection`.
The mutated run contains ten passed cases and two intended failures, with no skipped or flaky case.

Control and mutation evidence:

- `/var/tmp/rom-indexeddb-late-open-control-green-20261007/result.json`
- `/var/tmp/rom-indexeddb-late-open-control-green-20261007/mutation-review.json`
- `/var/tmp/rom-indexeddb-late-open-diagnostic-red-20261007/browser.log`

The supplied source directories derive from an authoring candidate. Their verifier mode does not establish complete release provenance.
This mutation verifies test sensitivity within the browser fixture; it does not establish production acceptance.
