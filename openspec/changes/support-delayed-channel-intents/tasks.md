## 1. Design and compatibility

- [x] Inspect current scheduling, native markers, archive admission, and explicit upgrade seams.
- [x] Select a public timing seam and old-reader format fence.
- [x] Validate this OpenSpec change before implementation acceptance.

## 2. Core scheduling

- [x] Add failing deterministic tests for no early claim, exact eligibility, timing identity mismatch, and age-before-floor terminal handling.
- [x] Add frozen optional fields and `Channel::intent_at`.
- [x] Preserve the floor through retry, operator scheduling, stale claims, and restore.
- [x] Retain reconciliation, authority, version, and budget behavior.

## 3. Format upgrade and shared adapters

- [x] Advance native/archive markers with explicit format-8/archive-6 upgrade admission.
- [x] Reject scheduling fields declared under older markers.
- [x] Verify populated source-preserving upgrades on both actual adapters.
- [x] Verify archive restore preserves the floor and fences active claims.
- [x] Add actual-adapter atomic commit, replay, restart, denial, and changed-version negatives.
- [x] Add a bounded worker-driven continuation test using the existing worker.

## 4. Verification

- [x] Run affected core, backup, upgrade, retention, and shared-adapter checks through the allocated native toolchain.
- [ ] Run downstream compile fixtures and actual previous-reader rejection through the separately allocated toolchain.
- [ ] Record source, lock, compiler, commands, failures, bounds, and results.
- [ ] Obtain independent review and run the full local verifier after ownership freeze.
- [ ] Document struct literal compatibility and data-restore versus binary-rollback limits.
