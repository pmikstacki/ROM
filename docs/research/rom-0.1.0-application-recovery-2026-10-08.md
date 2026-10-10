# Whole-application recovery admission

Date: 2026-10-08. Status: Complete authenticated recovery passed on SQLite and redb for the captured 332-source snapshot.
The subsequent shared-fixture extraction compiled and passed focused checks; it has not repeated the complete application matrix.
This report covers R8 in the consumer research plan. It does not establish release readiness.

## Investigated public boundaries

Both database adapters expose `backup_to` and fresh-destination `restore_from`.
The database archive excludes actual blob bytes and external delivery effects.
`BlobService` and the folder adapter provide the existing attachment protocol.
`StudioHost` and `IdentityGate` provide identity and current Resource admission.
The fixture uses these public APIs without a product change.

The existing demo recovery reuses its original object directory.
That test does not establish recovery when the source directory is unavailable.
The new fixture restores objects into a separate destination.
A private mount namespace hides the original source behind an empty read-only directory.
The recovered process must observe a different mount namespace and cannot read the original canary.
The parent retains the original source and all evidence.

## Approved experiment envelope

Each adapter case uses 1,000 synthetic Resources, 100 tombstones, and 20 attachments.
Each attachment has at most 64 KiB. Twenty acknowledged mutations follow the snapshot.
The combined database archive and object inventory has a 128 MiB limit.
The fresh snapshot has a 256 MiB limit inside the retained 32 GiB identity fixture envelope.
The new native target has a planned 8 GiB observation boundary, not a hard filesystem quota.
Processes use the existing owned launcher and finite time, memory, CPU, PID, and output limits.
No production data, credentials, deployment, or shared trust is changed.

## Chosen application composition

The fixture reuses `rom_demo::declarations` and the public reference preparation and recovery journey.
That journey leaves a known compensation pending, then verifies stable recovery and receipt replay.
Additional typed Resources supply the declared population and tombstone profile.
The snapshot retains a leased work identity and journal cursor to test old-owner fencing.
Recovery verifies 100 original receipt replays without new journal events.
It verifies each attachment through `BlobService`, not only by comparing copied files.
The restored Host must also accept a fresh login from the isolated actual provider.
Configuration contains matching reference names. Provider credentials are provisioned separately and excluded from the manifest.

## Initial executed evidence

The manifest's missing-module RED is retained in `/var/tmp/rom-010-application-manifest-red-20261008.log`.
The bounded manifest and copy suite passed four tests before additional restore cases were added.
Tests cover byte limits, digests, exact references, duplicate object names, and existing destinations.

The fresh debug-zero build completed in 63 seconds with exit status zero.
It captured 322 source paths and preserved a detached 57,027,048-byte binary.
Its SHA-256 is `35f3d62a11f0a365c797296b9da392a51eb954b60d171c0512f5fe77b264044f`.
Build identity: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-build-37a9ec78e12de62466a4a028/build-identity.json`.
The observed target size was 583,175,629 apparent bytes after the build.
These values are measured allocations, not the enforced private snapshot limit.

The first source preparation safely failed before opening a database.
Its path validator confused a component prefix with a complete `Path` prefix.
Log: `/var/tmp/rom-010-application-prepare-first-20261008.log`.
Process evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-beaa9186c77ff124c0c020d4/preparation-result.json`.
The owned process and cgroup drained. No provider started.
The corrected fixture accepts only the exact private parent, a valid nonce, and closed recovery endpoints.
A fresh rebuild and actual preparation repetition remain required.

## Remaining proof at initial preparation

The final current-source matrix below completes these initial fixture requirements.
Original-consumer and release-artifact acceptance remain separate.

Current fixture execution must restore both adapters into fresh destinations.
It must prove source isolation, attachment bytes, fences, receipts, work recovery, and actual fresh-provider authority together.
Measure RPO as acknowledged post-snapshot changes absent from recovery, with the recorded snapshot and acknowledgement times.
Measure RTO from the recovery start until database, attachment, work, and authenticated Host checks pass.
Do not present the database-check duration alone as whole-application RTO.

AP-UX-006, AP-UX-013, and AP-UX-021 still require original-consumer identity lifecycle acceptance.
A restored backend's correct denial cannot establish private-view clearing in Astral Plane.
Packaged artifact admission and the combined release verifier remain separate requirements.

## Corrected preparation and observed retention boundary

The corrected fixture test and Clippy passed. The new build captured 323 source paths.
Its detached binary has 57,043,784 bytes and SHA-256 `126af3a2c81eca6f1c7d188296f1e4a86e9d5e435f4835bd7a74516a6e43fc03`.
Build: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-build-68fa503ace828ba0c17fe466/build-identity.json`.
Fresh SQLite preparation populated the declared Resources, tombstones, and attachments successfully.

The subsequent restore failed with `HistoryGap` before the complete application checks finished.
The declared profile exceeds the default 1,024-row retained journal.
The reused reference recovery asks for history from the origin; its small example assumes that history remains available.
The existing public `open_with_limits` seam can configure 2,048 retained rows for this bounded drill.
The change retains the default 4 MiB journal byte limit and the approved archive and process limits.
It does not remove stale-cursor rejection or claim unlimited history.
A fresh configured-profile repetition remains required.

Failure: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-adf7e3711823374bc3dc7116/restore-result.json`.
The source and failed destination remain preserved.
The actual concealment mounts succeeded in a different private namespace.
The mounted source inode matched the empty directory, not the original source inode.
The mount was read-only. Both owned mount bridges drained.
Namespace: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-adf7e3711823374bc3dc7116/restore-namespace.json`.
This establishes the isolation prerequisite, not a successful whole-application restore.

## Configured-profile execution

The fixture now explicitly retains 2,048 journal rows and the default 4 MiB journal byte limit.
The fixture test, Clippy, and fresh native build passed after this configuration change.
The detached binary has 57,043,536 bytes and SHA-256 `cbfcf0d42ab38fd887140640e57c5565868105c50b6307158dbbc0215a617dd5`.
Build: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-build-840d877a72465bdf82711635/build-identity.json`.
Its source inventory contains 323 paths. The dedicated target had 849,494,341 apparent bytes after compilation.
This target measurement is not a hard quota or an allocated-block measurement.

Both adapters passed the real database, attachment, receipt, journal, tombstone, and work recovery checks.
Each restored database contains 1,000 original Resources, including 100 tombstones.
Each case verifies 100 original receipt replays without another journal event.
The old journal cursor and old leased-work claim are rejected after restore.
The original pending compensation completes with the same work identity and a higher generation.
All 20 attachment payloads are read through `BlobService` from the fresh destination folder.
The recovered processes cannot read the original source canary. The parent still has the source directory.

| Adapter | Archive bytes | Attachment bytes | Fresh restore checks | Evidence directory                              |
| ------- | ------------: | ---------------: | -------------------: | ----------------------------------------------- |
| SQLite  |     1,131,208 |           58,880 |             1,817 ms | `application-recovery-ea1562ed72b4774c17ac6bda` |
| redb    |     1,131,206 |           58,880 |             1,535 ms | `application-recovery-df8f3ec2d33465f09bceb299` |

These durations measure database, attachment, and work checks. They do not include authenticated Host recovery.
Evidence directories are under `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/`.
Each directory retains `restore-result.json` and its private namespace admission evidence.

Seven Node tests passed for manifest validation and bounded fresh-destination object restoration.
Log: `/var/tmp/rom-010-application-final-node-tests-20261008.log`.
Reference matching compares exact values independently of JSON property order.
Invalid digests and unresolved required references prevent object publication.

## Fresh authenticated SQLite recovery

The first browser attempt failed because the Node HTTP client lacked the private CA configuration.
Chrome's private NSS trust did not configure Node's independent TLS client.
The failure is retained in `application-recovery-ea1562ed72b4774c17ac6bda/http-result.json`.
The corrected fixture supplies the private CA to Node, without disabling certificate validation or changing shared trust.

A fresh SQLite destination then passed the complete authenticated recovery journey.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-43e08bb416cc506634560c43/http-result.json`.
The measured warm-provider RTO was 9,503 ms.
The measurement starts before object and database restoration. It ends after fresh authenticated access passes.
The actual isolated identity provider was already healthy before this measurement.
The result does not establish cold-provider provisioning time or a production recovery SLA.

An anonymous request was denied with HTTP 401.
A fresh actual-provider login returned a successful callback and the expected linked user.
The restored protected Resource was readable with HTTP 200.
An invalid CSRF value was denied with HTTP 403, without protected-value disclosure.
Attachment ownership remained the original embedded owner; this login does not transfer attachment ownership.
The fixture verifies attachment bytes through that owner's existing BlobService authority.

The measured RPO loss was 20 acknowledged post-snapshot mutations.
SQLite's last acknowledgement was 34,055 ms after the recorded snapshot finished.
That interval describes this synthetic sequence, not an application retention policy.
All owned Host, proxy, browser, and namespace processes drained.
Native sources, helper sources, and the host CA inventory remained unchanged.

## redb provider-window admission failure

A second timed redb destination passed all database, attachment, and work checks in 1,495 ms.
Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-80666e9226c4c4439b69794f/restore-result.json`.
Its subsequent HTTP admission rejected the recorded provider relay process identity.
The finite provider window had ended. The stale ready file was not accepted as a live provider.
Log: `/var/tmp/rom-010-application-timed-redb-20261008.log`.
This is a provider setup failure. It is not a redb semantic failure or an authenticated recovery success.
The redb authenticated recovery and whole-application RTO remain unproved.

The provider launcher exited zero after stopping its three exact owned containers.
A fresh cgroup observation found no live descendants. Private ports 44389–44393 were vacant.
The 323 native source hashes still matched the preserved binary witness before the source freeze was released.
Drain evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-provider-drain-fe64ff4f64117e717e779fac81c6e743.json`.
All containers, snapshots, failed destinations, source directories, and logs remain preserved.
A fresh provider window and a current source-matched binary are required for the remaining redb HTTP leg.

## Consumer feedback boundary

AP-UX-006, AP-UX-013, and AP-UX-021 still require original Astral Plane lifecycle adoption.
A backend denial does not prove that the application's private views are cleared.
Temporary provider failure remains distinct from confirmed identity denial and token expiry.
AP-UX-033 guest access remains separate from the protected Resource's anonymous denial in this fixture.
A successful installed recovery fixture does not establish original-consumer deployment or acceptance.

## Fresh complete matrix before shared-fixture extraction

The final build captured 332 source paths after the corrected R9 diagnostic integration.
The offline standalone lock refresh resolved 270 packages, including the newly required HMAC dependency.
The locked debug-zero build completed in 8.35 seconds with exit status zero.
The detached binary has 58,336,096 bytes and SHA-256 `8abde23a7bd9392073c877927af9e809acf18ecf05cde26d0f5c17629078dd69`.
Build identity: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-build-725cbf2ef07cd7fc1851a9ae/build-identity.json`.
The target used 1,084,268,544 allocated bytes after this build. This measured value is not a hard quota.

Both cases used fresh preparation, snapshot, post-snapshot mutation, and isolated destinations.
The snapshot contained 1,000 Resources, 100 tombstones, and 20 attachments.
Each case replayed 100 stored receipts without adding journal events.
The recovered process could not read the original source canary. The parent retained the source.
Actual attachment reads matched all 20 restored objects, with 58,880 combined bytes.
Old journal cursors and work claims were fenced. The original incomplete work completed with its stable identity.

| Adapter | Full recovery evidence directory suffix         | Whole-application RTO | Acknowledged post-snapshot mutations lost | Last acknowledgement minus snapshot time |
| ------- | ----------------------------------------------- | --------------------- | ----------------------------------------- | ---------------------------------------- |
| SQLite  | `application-recovery-1cd302be426419737e8b7b50` | 8,725 ms              | 20                                        | 139,592 ms                               |
| redb    | `application-recovery-a990827542bdfb7aef4d31c8` | 7,043 ms              | 20                                        | 7,964 ms                                 |

Evidence directories are under `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/`.
Each directory contains `restore-result.json` and `http-result.json`.
The RTO starts before object and database restoration and ends after authenticated Host checks.
The provider was already healthy before the timer started. These are warm-provider recovery measurements.
The acknowledgement interval describes controlled snapshot loss; it is not a configured production RPO guarantee.

Both browser cases observed protected anonymous denial `401`, provider callback `303`, and the expected authenticated User.
The protected read returned `200` after fresh provider login. Invalid CSRF returned `403` without the protected value.
Provider credentials remained separately provisioned and absent from the backup manifest.
Restored attachment authority remained with the original embedded owner; login did not transfer object ownership.

The fresh default provider window used the computed remaining-lifetime admission proof.
Each leg passed the 210-second pre-restore guard and separate 150-second HTTP admission guard.
The provider process exited zero after the exact owned stop request.
All three owned containers were stopped and preserved. The provider cgroup was empty, and ports 44389–44393 were vacant.
The final 332 source hashes, detached binary identity, fixture sources, and host trust remained unchanged.
Cleanup evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-recovery-final-drain-d078dd6f513ce404be7a401b1e1343b5.json`.

The combined provider, recovery, and load-helper Node gate passed 125 tests in 11,441.762185 ms.
Log: `/var/tmp/rom-010-recovery-load-provider-node-20261008.log`.
Earlier setup and retention failures remain preserved. They are not rewritten as successful runs.

This matrix establishes bounded fixture recovery. It does not establish installed release artifact admission.
AP-UX-006, AP-UX-013, and AP-UX-021 still require denial and expiry clearing in the original Astral Plane consumer.
Temporary provider outage must remain distinct from confirmed identity loss.
AP-UX-033 guest access remains separate from these protected-resource `401` checks.
The combined release verifier and original-consumer acceptance remain separate gates.

## Subsequent shared-fixture extraction

After the complete matrix ended, shared Host configuration and synthetic identity setup were extracted for the load fixture.
The changes are limited to the recovery fixture's HostConfig composition, Resource registration, and identity seeding modules.
The 332-source binary remains preserved against its executed snapshot.
It is historical evidence after this source change, not a binary built from the extracted source.
The extracted fixture compiled and passed its configuration test and Clippy with warnings denied.
The combined verifier remains required before integration.
It does not change the product core or automatically admit the new load runner.
