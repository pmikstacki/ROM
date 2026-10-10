# ROM 0.1.0 current release evidence audit

Date: 2026-10-08. Status: release admission remains open.

## Later root verification, 2026-10-08

This section records later observations. The original audit below remains historical evidence.
These observations do not admit a clean 0.1.0 package or production deployment.
Current `rom_backup::STORAGE_FORMAT` is 11; the canonical archive version remains 7.
The format-10 descriptions in the original audit concern earlier source. They do not identify the current native layout.

The root observed the complete v8 verifier terminate with exit zero.
It ran 439 Node tests and the Rust, documentation, and other checks in `./scripts/check`.
The before and after inventories matched all 3,666 working-source entries, including 55 tracked deletions.
The evidence is `.superpowers/rom-010-r7-refresh-prep/root-v8-full-20261008/result.json` and its retained log and inventories.
The log SHA-256 is `2d8e167e8f2dba33194f26d595a36ad7cd6e8c0e1ac2fbfad66e09d3ddf421ba`.
This execution concerns the working candidate with 0.0.3 versions. Later edits need fresh verification.

The v8 native-TLS browser trial failed at `authorization-submit` after the identification and password stages.
The root independently verified physical close, three empty process groups, and four vacant case ports.
The evidence is `.superpowers/rom-010-r7-refresh-prep/root-v8-case-terminal-review.json`.
The proxy never armed. Its zero native counters exclude requests before arming and do not prove absent token acquisition.
Source investigation identified a redirect-interception problem in the test's Playwright callback handling.
The proposed correction holds the original callback at the owned proxy before forwarding it to the native Host.
This correction is not yet an executed TLS acceptance result.

The private ENOSPC controller passed root verification of its 21-file source freeze and 64 pure tests.
The independent reviewer passed 66 pure tests, including two additional regression tests.
The freeze SHA-256 is `346384f38df15e77eb982826b8f979fa9046c2434c2d5f2a6330889e30742769`.
The review is `.superpowers/rom-010-enospc-independent-review/final-review.md`.
Compilation and focused Rust tests are admitted next. No real ENOSPC experiment is accepted by this review.
Native compilation, filesystem injection, causal syscall evidence, and same-key recovery remain separate gates.

This audit maps R1–R14 to retained evidence and exact next checks.
It changes no source, version, tag, deployment, or accepted historical record.
No Cargo command, native probe, load trial, browser, or database operation ran during this audit.
The audit read repository reports, selected source, and current raw logs and evidence records.
Conversation references in those reports remain recorded claims. This audit did not read or quote conversation contents.

## Source and evidence boundary

Inspected HEAD is `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`.
The large dirty worktree is the candidate; HEAD alone does not identify it.
These selected hashes identify inspected inputs, not a complete compilation witness:

| Input | SHA-256 |
| --- | --- |
| `Cargo.lock` | `f7f2bfae15f9464f96f0ba4ec24cfa7832098494562556351b8580bd4afefe3b` |
| `studio/package-lock.json` | `11e7630aa0293c7360838d41bb07bb40a52940f80b4938063a8d0eb9465ce5dc` |
| `crates/rom-backup/src/model.rs` | `2bcf4e6de2bf0647b54c0b0b77fc92d2290de7e594d0746c5e02e4ea30aa7f20` |
| `scripts/check` | `3f39fb22e22ba88804ffbeaac60f39edf90120527b70986ae22c8a3973be0bed` |
| `scripts/check-rust` | `b4cbe57b3294fd40c59bd95a47b914c65ae590bc00fda2c9630957e79ea6f10b` |
| `scripts/release-artifacts/requirements.mjs` | `5c1b9caf394d538000c9538cb8420547b892d659816ed8f8f1018cc0a4005808` |
| `crates/rom-sqlite/src/persistence.rs` | `db25efbcbe0ecd9e4038740dc6ec3a2bee1bbac7ed8429b4a3a2e434e2303ac3` |
| `crates/rom-redb/src/commit.rs` | `d7a334a9fe878d8b59e9d1f3a91d9b54d4e0a39d77cd6520a02bfc675b27da5b` |

Current `rom_backup::STORAGE_FORMAT` is 10; canonical archive version remains 7.
The ordinary archive reader explicitly accepts archive-7/storage-9 and archive-7/storage-10.
The [format decision](rom-0.1.0-incremental-format-decision-review-2026-10-08.md) defines the required predecessor compatibility controls.
Changing the native layout does not erase earlier semantic evidence. It prevents automatic inheritance of final-source acceptance.

The root supplied these scoped current results: core 150 tests and Clippy; SQLite 49 test-support and 36 default tests; redb 14 tests.
Studio 417 unit tests and type checking concern an earlier broad candidate, without a current browser matrix.
Those supplied counts were not independently repeated in this audit.

The audit directly read `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-native10-full-conformance-second-repaired-20261008.log`.
Its SHA-256 is `bb201810029b6bb524984aaedb61e50a44093e56cd03540fff04691f26822dab`.
All 41 success summaries report zero failures. Nested child summaries prevent treating their sum as a unique test count.
The log includes native backup, delayed upgrade, recovery, and shared semantics.
Both genuine predecessor-writer tests remain explicitly ignored in this ordinary command.
This raw log alone lacks a complete before/after source witness and compiler identity.

The audit also read `/var/tmp/rom-010-stage-consumer-root-checks-20261008.json`.
It records 15 optimized diagnostic-consumer tests, scoped Clippy, and the load-helper Node command with exit zero.
Its scope explicitly excludes a before/after fence and release admission.
The root reports 33 load-helper cases and 17 strict OpenSpec items passed.
These helpers do not establish the workload gate.

The first current `./scripts/check` passed 17 strict OpenSpec items and 319 Node tests, then failed formatting.
The subsequent formatted run failed on a stale isolated compile-fixture HMAC lock.
A later run reached the corresponding stale maintenance-portal HMAC lock.
The root repaired three isolated locks and completed the entire verifier with terminal exit zero.
The audit independently read its raw log:
`/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-current-full-verifier-all-active-locks-20261008.log`.
The log has 249,307 bytes and SHA-256 `e0818f4bb86e2e973407a7572f7a82d25c8ca60b04f15f362c44ec7d9e0c1edb`.
Both size and hash match `/var/tmp/rom-010-full-verifier-post-source-20261008.json`.
That record states terminal exit zero and captures 2,189 selected files, 20 omitted entries and one deleted path after execution.
It is a post-command selected snapshot, not a complete before/after compilation fence or clean-source artifact admission.
Earlier failure logs remain historical evidence; their results are not rewritten.

The successful log records 17 strict OpenSpec passes, 319 Node passes and Rust 1.99.0.
It reaches workspace checks, feature-enabled OpenRouter tests, warning-denied documentation, the public consumer and both compile-check drivers.
It also reaches the AI and maintenance-portal examples, all auth feature configurations and final identity documentation generation.
`scripts/check` and its Rust/auth/identity subcommands use `set -euo pipefail`.
Their recorded terminal success establishes execution of the complete fixed verifier, including silent formatting and dependency-graph guards.
Node release-orchestration fixtures in this log do not represent a real clean-source producer or actual browser admission.

This closes the whole-verifier execution gap for that completed candidate only.
The SQLite owner's next diagnostic-counter source work is outside this result and requires new affected and complete checks.
Formatting changed the prior 387-input diagnostic source capture. Its terminal trial fences remain valid for their historical execution.
`/var/tmp/rom-010-invalidation-ai131-full-verifier.log` predates later source changes and remains historical verification.

## Requirement map

“Recorded” below means the linked repository report describes execution at its own source identity.
It does not mean this audit reran the experiment or verified every underlying log.
All next checks must preserve old inputs and use isolated destinations, outputs, and owned processes.

The table records the 2026-10-08 audit boundary. Later sections record subsequent executions and source changes.
The current candidate uses native format 11 and archive format seven.
Final-package checks must use this current contract, not the table's historical native-10 source set.
The current support contract is [release support](../release-support.md).

| Requirement | Authoritative evidence and measured scope | Current scope gap | Exact next verification |
| --- | --- | --- | --- |
| R1: runtime-owned validators | [Captured validator review](rom-0.1.0-captured-validator-review.md): two independent Runtimes per adapter, conflicting catalogs, rejection state checks, caller compatibility and authority-aware replay. Current conformance includes transition tests; the completed whole verifier includes public compile fixtures. | Historical review and current checks lack final-source packaged admission; validator overhead remains separate from commit cost. | Repeat the two-Runtime SQLite/redb scenario from extracted packages. Record bounded callback restrictions and a validator-only overhead comparison. Repeat affected/full checks after subsequent source changes. |
| R2: public Studio authoring | [Installed Astral adoption](rom-0.1.0-astral-installed-adoption-2026-10-08.md): installed public entry, 44 frontend browser cases, later actual packaged-host recovery. [UI gap review](rom-0.1.0-ui-feedback-gap-review.md) maps controls, layout and feedback. | Earlier packages and browser binaries do not cover the final format-10 source set. Human accessibility assessment remains separate. | Build fresh installed controls, `/ui`, `/ui/components`, client, recovery and auth entries. Run Chromium/WebKit focus, values, styled-node sizing, compact composer and application recovery against matching final native packages. |
| R3: supported installation layouts | [Shared-install notices](rom-0.1.0-shared-install-notices.md) and [independent review](rom-0.1.0-shared-install-review.md) record whole-directory symlink admission and negative ownership/license cases. | Source/build notice checks do not admit final archives; unsupported package layouts cannot inherit support. | Run package tests and clean artifact production. Verify ordinary install, whole-directory dependency symlink, extracted package licenses and portable IDs; retain unknown-owner, missing-license and invalid-identity failures. |
| R4: reusable mutation recovery | [Installed Astral adoption](rom-0.1.0-astral-installed-adoption-2026-10-08.md): actual application, real Authentik and four recovery scenarios in each browser. [Public-result invalidation](rom-0.1.0-public-result-invalidation-2026-10-08.md): 80 portal executions. | Dirty historical packages, loopback HTTP and prior native formats do not establish final TLS/artifact admission or every R4 adversarial case. | Repeat the full current recovery matrix on both adapters and engines, including lost acknowledgement/reload/navigation, changed input, stale revision, cancellation, logout and revocation. Assert original keys/revisions and exactly one committed event bundle. |
| R5: durable composition | [AI read progress review](rom-0.1.0-ai-read-progress-independent-review-2026-10-08.md) records 165 AI, 44 adapter and 17 external consumer passes. Current shared conformance checks durable work. | Current layout and preparation refactors require renewed public-consumer restart proof; browser progress is a separate authorized projection. | Repeat prepare/publish/follow-up restarts at every boundary on both adapters. Check prepared input, original identity, age, attempts, budgets and receiver deduplication. Preserve visible unknown outcomes and explicit compensation semantics. |
| R6: public projections | [Public-result invalidation](rom-0.1.0-public-result-invalidation-2026-10-08.md) records live invalidation and export rejection after changed selection; [guest review](rom-0.1.0-public-guest-independent-review-2026-10-08.md) records narrow guest disclosure. | Prior portal matrices do not establish final packaged browser behavior or every conditional-cache/principal scenario. | Repeat anonymous allowlist/raw-job denial, private edit and permission invalidation, stale export and conditional-cache separation across principals on matching final packages. |
| R7: real production identity | [Identity continuation](rom-0.1.0-identity-continuation-2026-10-08.md) records real Authentik outage/restart recovery, four expiry cases, three plus one key-cutover passes, and four linked-User-denial/unlinked-account cases. | These successful source-authoring witnesses predate current persistence changes. Linked ROM User denial does not establish provider-disabled-account behavior. Browser runtime advisory limits remain. | Rebuild a complete source-fenced Host and repeat the actual TLS/provider lifecycle and negative certificate controls from final packages. Preserve the 3+1 rotation runs separately. Review denied-grant coverage and dependency advisories before claiming production identity support. |
| R8: complete application recovery | [Application recovery](rom-0.1.0-application-recovery-2026-10-08.md): complete authenticated two-adapter restore at a 332-source snapshot; measured RTO 8,725/7,043 ms and 20 deliberately lost later writes. | The report explicitly says subsequent shared-fixture extraction did not repeat the whole matrix; format-10 adds another source boundary. Warm provider recovery does not prove fresh provider deployment. | Repeat coordinated database/blob/configuration-reference/unfinished-work restoration on clean destinations using final native-10 packages. Verify current receipts, tombstones, fences, attachments and login. State accepted RPO/RTO and the warm-provider boundary. |
| R9: operation diagnostics | [Public diagnostics consumer](rom-0.1.0-public-diagnostics-consumer-2026-10-08.md): three real native tests, scoped Clippy and 18 finite overhead measurements. Current conformance includes diagnostics. | Earlier exposed-consumer/overhead proof lacks final packaged repetition. SQLite stage timers are diagnostic attribution, not production observer acceptance. | Repeat public diagnostic tests from extracted packages; verify action/receipt/event/reaction/attempt correlation, secret-sentinel redaction, fixed labels, full/closed queue and exporter failure. Measure disabled/enabled overhead under the same declared workload. |
| R10: supported workload | [Current native batch seed validation](rom-0.1.0-public11-load-validation-2026-10-08.md): the same default release binary completed 10,000 Resources and 10,000 Work records on each adapter within the unchanged 120-second deadline. SQLite elapsed 85,748 ms; redb elapsed 74,729 ms. Historical failed trials remain preserved. | The current seed experiment passes. Mixed workload, fault acceptance and final packaged reproduction remain unexecuted. Single seed observations do not establish sustained throughput or latency distributions. | Execute the reviewed mixed coordinator on each adapter. Verify actual reads/writes/streams/uploads/work, final durable completion, latency distributions, RSS, disk growth and queue measurements. Run separate genuine disk-full, unavailable-provider and interruption cases with recovery oracles. Repeat from final packages. |
| R11: safe upgrades | [Accepted writer](rom-0.1.0-accepted-writer-upgrade-2026-10-08.md) and [actual Runtime codec replay](rom-0.1.0-runtime-predecessor-upgrade-2026-10-08.md): genuine accepted native-8/archive-6 population and historical commands, not marker rewriting. Current conformance covers synthetic conversion. | Both genuine writer tests are skipped by ordinary conformance. Earlier writer/reader proofs predate native-10. Whole-application cutover and rollback remain distinct. | Explicitly rerun both ignored tests with preserved witnessed inputs on fresh copies. Restore genuine archive-7/storage-9 and write/restore storage-10. Convert native-9 to 10. Prove accepted old and actual native-9 readers refuse native-10 before writable open, including dirty redb copy handling. Test application rollback separately from retained-backup restore. |
| R12: human usability and genericity | [Installed Astral adoption](rom-0.1.0-astral-installed-adoption-2026-10-08.md), maintenance portal matrices and the [UI gap review](rom-0.1.0-ui-feedback-gap-review.md) provide agent fixture evidence for two domains. | Agent-authored fixtures cannot establish owner productivity, independent human accessibility or the owner's feature-completion experience. | Refresh two installed public consumers, then retain a human-authored feature exercise and independent accessibility observations. Record internal imports, vendor patches, repeated orchestration, diagnostics and completion effort. Keep original application, fixture and human evidence separate. |
| R13: bounded provider routing | [AI read progress review](rom-0.1.0-ai-read-progress-independent-review-2026-10-08.md), [AI contract review](rom-0.1.0-ai-contract-review.md) and historical full-verifier feature-enabled OpenRouter checks record policy/accounting evidence. | Final source, extracted feature configurations and consumer adoption remain unverified; reservations are not measured actual provider spend. | Run final feature-enabled OpenRouter and provider-neutral tests, compile gates and extracted AI consumer. Verify deterministic discovery-through-attempt deadlines, capabilities, fallback continuation, bounded attempts/rate limits and distinct reservation/actual accounting. State spend-ceiling limits. |
| R14: ergonomic durable agent flows | [AI read progress review](rom-0.1.0-ai-read-progress-independent-review-2026-10-08.md): current-authority progress, physical ownership after timeout, original read Wake identity and lost-acknowledgement recovery. | Scoped source tests do not establish final installed-package proof, two unrelated flow consumers or human authoring acceptance. | Repeat typed-tool compile diagnostics and both-adapter restart/lost-acknowledgement cases from final packages. Run two unrelated consumers through denied tools, invalid output, exhausted budgets, unknown effects and authority revocation. Separate ephemeral tokens from durable results. |

## Publication gates

| Gate | Current evidence boundary | Required admission |
| --- | --- | --- |
| Whole current verifier | Complete `./scripts/check` terminated with exit zero after scoped formatting and three isolated-lock repairs. The raw log and post-command source record were independently inspected as described above. | Preserve this completed execution. Repeat affected checks and `./scripts/check` after the next diagnostic-counter/source changes and on final release source. Retain compiler, source/lock witness, command and exit status; this post-command snapshot does not establish a before/after fence. |
| Compatibility and support documents | The root corrected `docs/native-upgrade.md` and the current compatibility paragraphs in `docs/release-support.md` to native 10/archive 7 and conversions 3–9 into 10. They distinguish format-9 eligibility, format-10 keyed Work layout and earlier predecessor-reader evidence. Accepted 0.0.3 native-8/archive-6 claims remain correct historical statements. | Complete final-format predecessor compatibility and repeat verification after subsequent source changes. Reconcile remaining support claims and measured limits after their checks. Review the native contract, corrected backup README and migration notes together. Preserve historical assertions and source identities. |
| Dependency admission | Isolated private browser crypto controls exist; retained runtime advisory limits remain recorded. | Review exact locked dependencies, advisories, licenses, Rust floor, features and production TLS/browser profile. Do not equate a private diagnostic runtime with production approval. |
| Matching source/artifacts | Current candidate package experiments use dirty source and `0.0.3` versions. | After functional admission, prepare one matching 0.1.0 Rust/Studio/lock/source set and review its compatibility. Produce only from clean committed source. |
| Full producer | No current completed clean-source producer is claimed. | Run `./scripts/release` to an exclusive output. All eight fixed commands in [the producer procedure](../../scripts/release-artifacts/README.md) apply, including fresh extracted production assets in both actual browsers. |
| Independent artifact and requirement review | `requireReleaseRequirements` validates record shape, retained digests and fourteen entries; it executes no behavioral checks. | Independently verify source/tree/lock/asset/package identities and complete R1–R14 executed evidence with scoped reviews. Static requirement-record acceptance cannot replace human or deployment evidence. |
| Source publication and completion | No 0.1.0 version, tag or deployment change is performed by this audit. | Publish the matching source/tag only after the applicable source, artifact, consumer, operational and human gates are recorded. Keep Actions and registry policy unchanged unless the owner changes it. |

## Current action order, 2026-10-09

1. Diagnose the retained login and public-composition failures. Preserve their original evidence and test assertions.
2. Verify the conservative planner correction on actual SQLite 3.53.4 and bundled 3.53.2 engines.
3. Integrate verified corrections. Run affected checks and the full local verifier on the resulting source set.
4. Complete authenticated application upgrade, rollback, and fresh-destination restoration with the current native format 11.
5. Verify genuine predecessor inputs, archive pairs, and downgrade refusal on private copies. Preserve accepted 0.0.3 source and binaries.
6. Pass mixed workload and fault cases on each supported adapter. Preserve the completed seed and ENOSPC experiments separately.
7. Prepare matching 0.1.0 Rust, Studio, dependency locks, and source identities. Produce clean-source artifacts with all eight acceptance commands.
8. Repeat both unrelated public consumers, AI flows, diagnostics, and operational checks against those final artifacts.
9. Complete human authoring and accessibility observations. Agent fixtures cannot supply these observations.
10. Finalize support and migration records, dependency review, independent artifact review, preview deployment, and source/tag publication.

Existing successful experiments establish their recorded source and execution scopes only.
Final-package admission, full application upgrade/restore, mixed-load acceptance, and human assessment remain open.
Subsequent source changes require fresh verification.

## Current operational verification: native TLS and predecessor controls

This section supplements the historical requirement map above. It does not establish final 0.1.0 artifact admission.
The current persistence format is native 11. The backup archive format remains 7.

The root observed terminal exit zero for the latest complete local verifier.
Its retained result records 443 Node tests and the complete Rust and documentation checks.
All 3,668 selected source entries matched before and after execution.
The result is in `.superpowers/rom-010-r7-refresh-prep/root-v10-full-20261008/result.json`.
Its log SHA256 is `fcbda2e0c67cbc4ced2bde9a6c858208afdc12b1cf1d132b2a3efae98dbc59ed`.
Later source changes require new verification.

Native TLS has six successful cases out of fourteen planned cases.
The trusted SQLite case exchanged the original authorization code, acquired JWKS, and returned authorized Resource data.
SQLite rejected wrong CA, wrong name, and expired certificates on the token route.
SQLite also rejected wrong CA and wrong name on the JWKS route after successful token acquisition.
The matrix stopped when the remaining provider window was too short for another case.
Eight cases remain unexecuted: SQLite JWKS expiry and all seven redb cases.
The root verified the five negative-case result hashes in `.superpowers/rom-010-r7-refresh-prep/root-partial-matrix-review.json`.
These results do not establish final-package identity or every provider lifecycle scenario.

The current predecessor refresh passed twelve tests, including genuine old-writer conversion and Runtime replay on both adapters.
Eight separate old-reader controls also passed. Older readers refused the upgraded native and archive formats.
The root verified retained logs, terminal records, original inputs, and source identities.
See `.superpowers/rom-010-upgrade-refresh-prep/reader-controls-20261008-01/root-reader-review.json`.
Whole-application cutover and rollback remain unproven.

The ENOSPC native probe compiled, but the actual fault attempt stopped before native database execution.
The trace tool could not open the inherited console socket as a regular output file.
The replacement uses an exclusive regular trace file with separate console capture.
The root reproduced all 90 tests of the revised test mechanism.
The 28-file source review is in `.superpowers/rom-010-runtime-r10-prep/faults/evidence/root-v5-source-review.json`.
A separate tool-only transport probe must pass before another database fault attempt.
No native ENOSPC recovery result is claimed. Failed images and logs remain preserved.

The validator benchmark remains separate from production workload acceptance.
Its first isolated compile failed because the copy lacked an existing test fixture.
The fixture was copied without changing maintained source. The optimized compile retry and exact semantic test passed.
Both commands drained. All 148 input entries matched before and after execution.
The proof is in `.superpowers/rom-010-r1-validator-overhead-prep/copy-review/compile-semantic-20261008-02/execution-proof.json`.
No validator timing result is claimed.

### Failed continuation remains separate from certificate results

A fresh provider window admitted the eight remaining TLS cases.
The first resumed case stopped before proxy and browser launch.
All three recorded child processes drained. No native TLS observation was produced.
The root verified the failed case and retained the result hash in `.superpowers/rom-010-r7-refresh-prep/root-resume-failure-review.json`.
The orchestrator discarded Host output and used a generic failure result.
Therefore, the existing evidence does not identify the exact startup failure.
The listener guard differs from the 3,000 ms OIDC acquisition limit.
It permits forty attempts with 100 ms pauses and separately bounded listener queries.
A source correction will record closed stage categories without retaining secrets or changing the deadlines.
Six certificate cases remain proven. Eight remain unproven.

The tool-only ENOSPC transport probe also stopped on a process-ownership guard.
Its retained readiness record shows the child file limit was restored.
The child's exact process group was not saved before rejection, and the process has exited.
An intermediate group created by the timeout wrapper remains a hypothesis.
The next wrapper will retain process identity before the guard and preserve whole-group supervision.
This result does not prove database ENOSPC or recovery behavior.

### Later V11 verification and remaining operational gates

The V11 Node suite passed 446 tests after an explicit toolchain PATH correction.
The first attempt failed because fixture commands could not find `rustc`. Both logs remain preserved.
All 3,615 selected entries matched before and after the successful run.
This inventory differs from the earlier 3,668-entry full-verifier inventory.
The root review is in `.superpowers/rom-010-r7-refresh-prep/root-v11-node-result-review.json`.
This result is a Node-suite result, not a new full Rust verifier result.

SQLite JWKS expiry and trusted redb TLS subsequently passed with actual native and browser observations.
The root checked both result hashes, certificate-case assertions, source identities, and recorded process drains.
See `.superpowers/rom-010-r7-refresh-prep/root-sqlite-jwks-expired-v11-review.json` and `.superpowers/rom-010-r7-refresh-prep/root-redb-trusted-v11-review.json`.
Eight of fourteen certificate scenarios are now proven across their retained source identities.
The six redb negative cases remain unexecuted at this checkpoint.
The V4 provider ended with actual terminal code zero. Its owned processes drained and all five ports were vacant.
The root reviewed the next six-case batch and reproduced its three admission tests before admitting a separate fresh provider.
Batch execution still requires a fresh readiness reference and an exact finite admission record.

The replacement foreground transport probe passed with actual tracer and child process identities.
Its trace used a regular file. The root checked its result and process drain in `faults/evidence/root-tool-positive-review.json`.
That path is relative to `.superpowers/rom-010-runtime-r10-prep/`.
The next SQLite fault attempt stopped during filesystem filling, before the native mutation was armed.
Therefore, it does not prove native ENOSPC or recovery. Its failed image and logs remain preserved.
The previous filler did not retain its filesystem facts before rejecting the attempt.
Those missing facts cannot establish the earlier failure's cause.
The revised collector saves facts before rejection and passed nine tests reproduced by the root.
An ext4 availability-rule correction is under review against the host's Linux 6.6.94 sources.
No new fault attempt is admitted by that source review.

The whole-application upgrade and rollback fixture remains source-only.
Its controller tests do not establish actual application cutover, restored traffic, or external-effect recovery.
Validator timing and mixed-load execution also remain unexecuted.
Final-package verification, release publication, and deployment remain open.

### Completed remaining certificate matrix

The separately admitted six-case redb matrix subsequently ended with actual terminal code zero.
All token and JWKS routes rejected wrong CA, wrong name, and expired certificates as expected.
The root checked every retained result hash and certificate-case assertion.
The matrix's current outer cgroup was empty. The four case ports were vacant.
All seven private and thirty-three maintained source hashes matched the reviewed freeze.
The matrix retained 43,499,520 allocated bytes across the six cases.
See `.superpowers/rom-010-r7-refresh-prep/root-redb-negatives-v11-result-review.json`.
All fourteen planned certificate scenarios are now proven across their retained V10 and V11 source identities.
This does not establish final-package identity or the remaining provider lifecycle, mixed-load, and application recovery gates.

### Actual fault and mixed-load attempts remain incomplete

The next ENOSPC wrapper stopped before image creation because its private manifest had mode `0644` instead of `0600`.
The manifest contents remained unchanged. The failure record is `faults/evidence/sqlite-v6-preflight-failure.json` under `.superpowers/rom-010-runtime-r10-prep/`.
Its successor created the isolated filesystem and opened the native database before filling.
The filler then observed actual write ENOSPC, successful fsync, 3,934 free blocks, two available blocks, and 49,135 free inodes.
The filesystem used 1 KiB blocks. The failed requested write was 64 KiB.
The strict zero-availability guard stopped the attempt before arming the native mutation.
Thus neither attempt proves native publication failure or recovery.
Their source, image, syscall observations, process drains, and unmount results remain preserved.

The revised filler packs smaller requests after ENOSPC within explicit byte, call, and error-record bounds.
The root reproduced seven new regression cases and checked all thirty-eight frozen inputs.
See `faults/evidence/root-tail-packing-review.json` under `.superpowers/rom-010-runtime-r10-prep/`.
This source review does not establish a new actual fault result.

The first SQLite mixed attempt stopped during login before workload traffic.
The database copy and Host readiness succeeded. The login child returned exit code one.
The proxy recorded one TLS error and no HTTP requests. The original database remained unchanged, and the owned processes drained.
The login environment lacked Node's additional CA path, although Chromium had NSS trust.
That source observation is a candidate explanation, not an exclusive cause proven by the retained generic failure.
A narrow trust-environment and diagnostic correction remains under test.

Both application rollback dependency graphs resolved offline and passed their registry and public-path audits.
Both metadata commands physically closed with terminal code zero; only their two private locks changed.
The root review is `.superpowers/rom-010-upgrade-refresh-prep/application-rollback-plan/fixture/evidence/root-metadata-review.json`.
Native compilation and actual application cutover remain separate gates.

### Native builds and corrected fault workload

Both application rollback Hosts subsequently compiled in release mode with actual terminal code zero.
The root checked the Cargo artifact records, detached binary hashes, source hashes, and recorded process drains.
See `.superpowers/rom-010-upgrade-refresh-prep/application-rollback-plan/fixture/evidence/build-20261008-02/root-build-review.json`.
The consumer sources and audited dependency lock are prepared. Installation, browser traffic, cutover, and rollback remain unproven.

The next filesystem attempt reached zero available blocks after bounded filling and successful fsync.
Its native mutation returned `TooLarge` before storage admission. No native database ENOSPC witness was established.
The generated command exceeded the unchanged 16 KiB command limit.
The private experiment now uses an 8 KiB title. Production limits remain unchanged.
Four actual native tests passed, including the exact generated command on normal-space SQLite and redb.
The corresponding release build passed. Both test and build processes physically closed.
See `.superpowers/rom-010-runtime-r10-prep/faults/evidence/native8192-compile-proof.json`.
These results establish workload validity and compilation. They do not establish full-disk recovery.

The corrected mixed-load login passed with Node CA trust. The first operator request then failed before mutation traffic.
The retained generic failure does not identify its exclusive cause.
A separate diagnostic observer passed forty-four tests reproduced by the root.
It records bounded failure categories without tokens, headers, URLs, or raw error messages.
Actual mixed-load acceptance remains open.

### Actual SQLite ENOSPC recovery

The separately admitted SQLite attempt completed with actual terminal code zero.
Its causal witness records native `pwrite64` ENOSPC on the sampled database WAL during the armed mutation interval.
The filesystem had zero available blocks. The native file limit exceeded the filesystem size.
The caller received an unknown outcome. After space was released, the same-key recovery produced one committed mutation.
The recovery evidence contains one row, receipt, event, and source-work record for the target.
The authorized read returned revision one. The second replay preserved the complete snapshot.
All 10,001 work records were done. The original seed remained unchanged.
The owned processes physically closed, and the isolated filesystem was unmounted.
See `.superpowers/rom-010-runtime-r10-prep/faults/run-29741baf2752c7ec233dd3b8/result.json`.
The secondary archive review checked all five archive envelopes, checksums, complete snapshots, and target counts.
The root checked the current archive hashes, replay equality, kernel witness, recovery proof, and empty cgroups.
See `.superpowers/rom-010-runtime-r10-prep/faults/evidence/root-sqlite-v9-result-review.json`.
This is a private-source SQLite experiment, not redb recovery or final-package acceptance.

The diagnostic mixed-load attempt again stopped at the first operator request before mutation traffic.
Its proxy recorded a Host route-profile rejection, not a proxy budget rejection.
The intended operator route satisfies that profile. The rejected request path was not retained.
An incidental browser request remains a hypothesis. Source-only diagnostics are being refined without relaxing the route profile.

### Installed application consumer

The application consumer installed offline through the reviewed dependency lock.
Its first type-check failed with 57 diagnostics for explicit `.ts` imports in the supplied public package.
The consumer configuration now enables `allowImportingTsExtensions` with its existing `noEmit` and Bundler resolution.
The [TypeScript option documentation](https://www.typescriptlang.org/tsconfig/allowImportingTsExtensions.html) describes this bundler-based contract.
No type-check suppression or production package change was used.
The next separately admitted type-check passed with zero errors. It retained one warning because the consumer contains no Svelte inputs.
The Vite build passed and produced the two expected Host assets.
The root checked the actual phase closures, current empty cgroups, log hashes, asset hashes, and single configuration delta.
See `.superpowers/rom-010-upgrade-refresh-prep/application-rollback-plan/fixture/evidence/consumer-prep/npm-20261008-02/root-consumer-review.json`.
The successful install was reused without another `npm ci`.
This is a supplied-source consumer experiment. Actual application traffic, upgrade, rollback, and final-package identity remain open.

### Mixed-load fixture cause and redb fault limit

The next diagnostic mixed-load attempt identified the rejected request as `GET /favicon.ico`, without query parameters.
The private proxy rejected that incidental browser request under its Host-prefix profile, then stopped the fixture.
The intended operator request received no response. No mutation workload ran.
This result identifies a fixture failure. It does not establish a production Work authorization defect.
See `.superpowers/rom-010-r7-refresh-prep/mixed-v9-sqlite-outer-result.json` and its retained coordinator result.
A bounded local favicon response is being designed without forwarding it or admitting other routes outside the Host prefix.

The separately admitted redb attempt filled its isolated filesystem to zero available blocks.
All 120 native mutations were acknowledged. No native ENOSPC candidate or recovery result was established.
The fixed attempt bound stopped the experiment. Existing allocated database pages are a possible explanation, not a proven allocation trace.
The owned processes drained, and the isolated filesystem was unmounted. The original redb seed remained unchanged.
See `.superpowers/rom-010-runtime-r10-prep/faults/evidence/redb-v1-post-witness.json`.
This result does not satisfy the redb disk-full recovery gate.

### Continued independent review at 23:10 UTC

The private mixed-load proxy now returns a bounded local response for bodyless favicon requests.
It does not forward those requests or admit other routes outside the Host prefix.
The source tests passed. The root checked the 19-file freeze and the 476-source native build identity.
The original provider window closed naturally. Its processes drained before one fresh provider window started.
The root then admitted one SQLite mixed-load attempt with the existing 300-second execution bound.
That attempt was live at this checkpoint. No completion result was available.
See `.superpowers/rom-010-r7-refresh-prep/root-mixed-v11-sqlite-admission.json` and `mixed-v11-sqlite-outer-live.json` in the same directory.

The private redb preparation will compare complete snapshots before and after compaction of an isolated copy.
The root reproduced ten pure tests and checked the source freeze.
Offline lock generation completed and drained. All 33 registry tuples match the previously admitted native dependency graph.
The ten maintained lockfiles remained unchanged. Compaction and database access have not run.
Focused Rust tests and a separate release build are admitted next. Database execution requires a further review of the resulting binary.
See `.superpowers/rom-010-runtime-r10-prep/faults/evidence/root-compact-lock-review.json`.

The root reproduced 29 tests for the application upgrade controller and its source and provider-window checks.
The controller verifies the accepted Git source once, then compares bounded source bytes at each stage.
The same authentic provider window can serve successive stages. Its original deadline cannot be extended.
The proposed execution budget retains all seven 60-second maintenance bounds.
Actual TLS relay setup, callback configuration, application traffic, and rollback remain unexecuted.
See `.superpowers/rom-010-upgrade-refresh-prep/application-rollback-plan/fixture/evidence/root-source-review-20261008-continued.json`.
These results do not establish final-package acceptance or completion of ROM 0.1.0.

### Actual mixed-load result and dependency audit

The SQLite mixed-load attempt completed with failure. All 12,000 groups were scheduled and drained without reaching the deadline.
All four operator requests succeeded. The proxy reported no rejected requests, TLS errors, or route failures.
The driver recorded 11,132 successful HTTP outcomes, 242 overloads, and 626 failures across 12,000 requests.
It recorded eight stream errors. Upload and download requests did not run after reservation failures.
These results do not satisfy the unchanged mixed-load acceptance criteria.
See `.superpowers/rom-010-r7-refresh-prep/mixed-v11-sqlite-outer-result.json` and its retained client result.

The Host returns blob reservations and uploads as a status plus a Resource projection.
The driver incorrectly expected a bare Resource projection. A focused correction now verifies the actual envelope and Resource identity.
The root reproduced 12 focused tests. Malformed envelopes and incorrect identities still fail.
No corrected mixed-load attempt has run. The overloads and stream errors remain separate open findings.
The historical seed, native build identity, and failed-run evidence remain unchanged.

The root ran cargo-audit 0.22.2 against the current root lockfile and a freshly cloned RustSec database.
The database revision is `550efd3d587a29b2e2c2b21b17a440da4fede999`, committed on 2026-10-08.
The command denied warnings, used no advisory exclusions or target filters, and retained default yanked-package checks.
It exited zero with no reported vulnerabilities or warnings. The lockfile hash remained unchanged.
See `.superpowers/rom-010-root-advisory-20261008-2315/root-review.json` and the raw result in the same directory.
This audit covers that lockfile. It does not establish license completeness, upstream bundled-native patch status, or final-package security acceptance.

### Actual compaction, validator timing, and browser trust preparation

The root executed one private redb compaction preparation. The owned parent and native process exited zero and drained.
The copied database decreased from 59,772,928 bytes to 34,185,216 bytes. The whole work completed in 3,978.5 milliseconds.
Both public backup archives contain 17,659,350 bytes. Their bytes and SHA256 hashes are identical.
The full snapshot retains 10,000 Resources and 10,000 completed work records. The original seed hash remains unchanged.
See `.superpowers/rom-010-runtime-r10-prep/faults/evidence/root-compact-actual-review.json`.
This result permits preparation of another isolated redb experiment. It does not satisfy the redb disk-full recovery gate.

The provider window closed naturally before the root admitted validator timing. No ROM database, browser, provider, or build phase ran concurrently.
Other projects and OS activity remained untouched. CPU, affinity, and load observations remain in the admission record.
The first benchmark process exited zero, but its parser rejected the log.
Rust attached the first JSON record to the test-name prefix. The raw log contained all 216 records.
A separate framing helper removes only that exact prefix. Four regression tests and three existing parser tests passed.
The original parser, raw log, failed result, and recovered summary remain unchanged or separately retained.
Three subsequent processes each passed one benchmark test and produced 216 validated records. Their owned groups drained.
See `.superpowers/rom-010-r1-validator-overhead-prep/copy-review/root-timing-20261008-02/result.json` and all three summaries in that directory.
The measurements compare internal registered validators, callbacks, and codecs. They exclude authorization, commit, storage, and contention costs.
Paired differences and negative values remain in the summaries. These small samples do not establish statistical significance or final-package performance.

The root checked the 61-file application fixture and reproduced 33 controller tests.
It then executed two bounded NSS commands to prepare a private browser trust store. Both commands exited zero and drained.
The accepted 344-file source, current 574-file source, and fixture remained unchanged. Three NSS database files were retained.
See `.superpowers/rom-010-upgrade-refresh-prep/application-rollback-plan/fixture/evidence/root-nss-v2-actual-review.json`.
No application browser, TLS relay, provider configuration change, upgrade, or rollback ran during this preparation.

Both fresh Studio npm audits exited zero with no reported vulnerabilities: production dependencies and the complete dependency set.
The package manifest and lockfile remained unchanged. No installation or dependency upgrade occurred.
See `.superpowers/rom-010-root-npm-advisory-20261008-2320/` for raw results and command exit records.
These results cover the audited lockfile. They do not prove license completeness or final-package acceptance.

### Fresh whole-worktree verifier and authenticated SQLite source

The root executed `./scripts/check` in the ROM development container with Rust 1.99.0.
The command exited zero. Its source inventories are identical before and after execution.
All 17 OpenSpec items passed. All 451 Node tests passed, with no failures, skips, or cancellations.
The Rust commands include formatting, Clippy, workspace tests, OpenRouter test-support tests, documentation, public consumers, compile fixtures, and adapter dependency guards.
The authentication and identity verification scripts also completed. Explicit ignored fixtures retain their separate release gates.
See `.superpowers/rom-010-root-verifier-20261008-late/root-review.json` and the complete logs in that directory.
This run uses the default bundled SQLite profile. It does not establish final-package, browser, upgrade, or workload acceptance.

The root read the native dependency report and the existing native build route.
It then downloaded the pinned SQLite 3.53.4 archive from the official release site into a new private directory.
The archive SHA256 and sqlite3.c SHA3-256 match the existing reviewed pins.
Three source inputs were copied from that archive. Their retained identities include hashes, sizes, owners, inodes, and permissions.
The header identifies version 3.53.4 and the source ID published in the [official release record](https://sqlite.org/releaselog/3_53_4.html).
See `.superpowers/rom-010-root-verifier-20261008-late/sqlite-source-ef53f230b882575c4c913448/result.json`.
No compiler, native workload, or dependency change ran during this source acquisition.
The source inventory does not prove the identity or behavior of a future linked executable.

The application upgrade phase did not start in provider window V8.
The remaining authentic window was shorter than the reviewed 1,055-second application entry requirement.
The deadline and test thresholds remain unchanged. No replacement provider starts automatically.
Source review also found missing application-root metadata in the generated phase-plan file.
A separately tested preparation correction is in progress. Historical source files and evidence remain preserved.

### Actual V9 registration failure and isolated redb admission

The V9 application sequence performed one provider registration attempt. Its tool exited one, and the owned process group drained.
The provider accepted the new callback. Its response included redirect_uri_type: "authorization", which the expected record omitted.
The three original redirects and all other provider fields remained unchanged in the retained before and after snapshots.
The strict comparison rejected the response. Application preparation, upgrade, rollback, and automatic restoration did not start.
The original expected record and failed-run evidence remain unchanged.
See /var/tmp/rom-r11-registration-8192837465a4b3c2d1e0f9a8/ for private snapshots and terminal evidence.

The root reviewed the pinned upstream API serializer. It supplies the authorization type when the request omits that field.
See the [pinned Authentik serializer](https://github.com/goauthentik/authentik/blob/e5a0d2f7572cb776eee7a3e9355937ce38973761/authentik/providers/oauth2/api/providers.py#L33).
A separate correction declares that type explicitly. It rejects unknown metadata and requires an unchanged current provider snapshot before restoration.
The root reproduced four contract tests. This evidence does not prove live restoration or image-to-source attestation.
The V9 provider retains its original finite deadline. No application retry or replacement window follows automatically.

The root reviewed the isolated derived-redb parent wrapper and reproduced its 19 preparation tests.
Fresh read-only admission found no conflicting fault mounts, loop devices, or process groups. Available space was 210,491,781,120 bytes.
The manifest hash was a281867af7489dbda6a937595e8dfe9d976c2edbe9a6891b7f128da03939b808.
See .superpowers/rom-010-redb-derived-fault-preparation-20261008/root-parent-prelaunch-review.json.
One bounded execution was admitted with unchanged native inputs, recovery rules, 120 attempts, and 8,192-byte titles.
The parent limit is 430 seconds. Planned filesystem and evidence allocation is approximately one GiB, without a hard quota.
The wrapper repeats admission before launch and verifies physical, process-group, cgroup, and source closure after execution.
Admission and preparation tests do not satisfy the actual redb disk-full recovery gate.

The admitted derived-redb attempt exited one before arming the disk-full fault.
The new parent passed its derived-controller grant, but omitted the original filesystem helper's required grant.
The native preparation and isolated image were created. The filler rejected execution before the fault workload began.
The controller unmounted the filesystem and retained the failed image, baseline archive, trace, and original inputs.
The actual parent exited one, closed its streams, and left an empty owned process group and cgroup. Source fences passed.
See .superpowers/rom-010-redb-derived-fault-preparation-20261008/parent-v1-evidence/terminal.json and faults/run-f2b80d39068f25d22bdf5934 under the R10 preparation directory.
This result is a harness failure, not a redb recovery result. No retry is admitted by the exhausted execution grant.
A separate parent-environment correction will test both existing grants without changing the workload or native recovery rules.

### Isolated SQLite engine-witness source preparation

The root copied the current source inventory to a private directory. It copied 35,301,377 bytes from 3,671 inventoried paths.
The inventory records 55 absent paths. No build targets, dependency installations, database files, or historical evidence were copied.
The native-profile proposal changes exactly two existing paths and adds one helper in that copy. Maintained source hashes remained unchanged.
See .superpowers/rom-010-native-copy-preparation-20261009/isolated-patch-review.json.
An initial patch command inside the repository skipped the paths. The root detected the unchanged copy before proceeding.
The actual patch applied from /tmp with an explicit destination. No maintained path was patched.
The proposed Rust helper failed rustfmt checking. The root retained its original bytes and the failed output, then formatted only the isolated helper.
The subsequent rustfmt check exited zero. See format-result.json and both logs in the same private directory.
This preparation does not establish compilation, linked SQLite identity, compile options, or native-profile acceptance.

### Actual provider restoration and derived-redb recovery

The guarded provider restoration exited zero. The actual final provider response matches the entire original snapshot, including all three redirects.
The root independently compared both retained response pairs. The owned process group and current cgroup are empty; physical closure exited zero.
Application execution remains absent. The provider retains its original independently owned finite window.
See .superpowers/rom-010-upgrade-refresh-prep/application-rollback-plan/fixture/evidence/application-prep/registration-fix/root-actual-restoration-review.json.
This result proves restoration of the isolated provider configuration. It does not establish application upgrade or rollback.

The corrected derived-redb attempt exited zero. Its first candidate produced a sampled pwrite64 ENOSPC on the actual database file.
The trace binds TID 21122 to TGID 21090, birth 73723459, FD 10, device 1795, inode 13, and mount 4480.
The armed interval has start 63, failure 68, and end 70. Available filesystem blocks were zero, with free inodes remaining.
The root independently checked the native witness, sampled thread identity, interval, descriptor binding, and recovery contract.
The candidate outcome was Unknown. The reopened pre-replay archive equals the complete baseline archive byte for byte.
Authorized same-key recovery produced one Resource row, receipt, event, and source-work record. Revision is one, and the counter is zero.
The second replay archive equals the complete first replay archive byte for byte. Work drain completed with 10,001 Done records.
The parent, native recovery, and unmount commands closed. Fresh checks found no fault mounts, loops, or process groups.
Original seed and source fences passed. Allocated retained evidence was 271,949,824 bytes.
See .superpowers/rom-010-redb-derived-fault-preparation-20261008/root-actual-redb-result-review.json and the preserved run-b7019f24bbf87f1233326e89 directory.
This result uses the preserved native probe and compacted copy of the original seed. It is not packaged-consumer or mixed-load acceptance.
The failed first parent attempt remains unchanged. No further retry follows from these execution grants.

### Corrected future upgrade sequence: source and static admission

The root reviewed the new provider runner, sequence changes, and unused browser-profile checks.
Eight focused tests passed. All 45 frozen source hashes and the exact configuration hash matched.
The actual static preflight completed at /var/tmp/rom-r11-sequence-72a63f9810b4e57cd209184f/preflight.json.
See application-prep/corrected-sequence/root-source-preflight-review.json under the preserved upgrade fixture.
Registration declares the authorization redirect type explicitly and verifies every original provider field.
Final restoration compares the complete current record with the saved after-state before restoring the complete original before-state.
The application deadline remains 1,055 seconds, and entry still requires 1,139 seconds of authentic provider time.
The root verified that the proposed V10 supervisor changes only the V9 output names.
V9 remained live at 2026-10-09T00:17:36.246Z, with its original process births and unchanged deadline.
No corrected watcher, V10 provider, application cutover, or rollback ran during these checks.

The root also corrected docs/release-support.md from native format 10 to the actual candidate format 11.
The conversion range now matches the current native-upgrade procedure: formats 3 through 10 into format 11.
Archive format seven and accepted 0.0.3 history remain unchanged. All 17 local support-document links resolve.
See .superpowers/rom-010-native-copy-preparation-20261009/support-description-review.json.
This documentation correction does not admit the candidate release or its final packages.

### Corrected R11 sequence, 2026-10-09

Root independently verified V9 physical closure, current empty cgroups, absent prior processes and all seven vacant ports.
The corrected watcher ran once as tool `14401`, followed by the separately admitted V10 provider as tool `23128`.
Its authentic entry retained 1,148,375 ms, above the unchanged 1,139-second admission requirement.
Registration and preparation exited zero. The application stage exited one and retained its failure evidence.
The sequence did not run final restoration or retry the application.
The callback can remain registered until separately reviewed recovery confirms the whole original provider configuration.
The inner application failure is under investigation; this result does not establish upgrade or rollback acceptance.

Evidence: `/var/tmp/rom-r11-sequence-72a63f9810b4e57cd209184f/ready-entry.json`, `failure.json` and `app-terminal.json`.
Provider closure evidence: `.superpowers/rom-010-r7-refresh-prep/provider-window-v9-root-closure-review.json`.

### Corrected R11 recovery and native-profile admission

The separate provider recovery ran once as tool `94563` and exited zero.
Root compared complete provider records: recovery-after equals original-before, and recovery-before equals registered-after.
All three original redirects and all other provider fields were restored.
Fresh closure checks confirmed physical completion, absent owned processes and an empty current cgroup.
Evidence: `corrected-sequence/recovery/root-actual-restoration-review.json` under the retained R11 application preparation.

The isolated SQLite 3.53.4 parent ran once as tool `73963` and exited one before any native stage.
Its allocation check invoked the resolved coreutils multicall executable without the `du` command selector.
No C compiler or Cargo stage launched. Created native inputs and admission evidence remain preserved.
Root independently checked the failed physical terminal, absent parent and empty current parent cgroup.
The unsaved inner-cgroup identity remains an evidence limitation.
A narrow dispatch correction and real allocation-command regression are under preparation; no native acceptance is established.

### Native profile and public composition results, 2026-10-09

The corrected isolated native run, tool `4530`, linked SQLite 3.53.4 with 39 recorded compile options.
The engine witness and 42 focused work tests passed. The `rom` unit suite passed 179 tests.
SQLite units passed 65 tests, failed seven, and ignored five. The run stopped before shared and consumer integration tests.
The existing planner recognizes only version 3.53.2. A two-version proposal preserves the exact plan checks and unknown-version fallback.
Actual query-plan witnesses and a separate 3.53.2 compatibility run remain necessary before integration.
Evidence: `.superpowers/rom-010-native-copy-execution-preparation-20261009/v2-actual-post-witness.json` and `engine-profile-v3/README.md`.

The public composition consumer completed its locked installation, source comparison, Svelte check, and build.
Its browser phase executed 16 tests: ten passed and six failed, with no skipped or flaky results.
Both engines reported compact-panel geometry failures and focus loss after a breakpoint change.
These results do not distinguish animation-time measurements from steady layout errors. Diagnosis remains open.
The independent post-run review confirmed unchanged inputs, empty current cgroups, absent owned processes, and a vacant preview port.
Evidence: `.superpowers/rom-010-public-layout-acceptance-prep-20261009/browser-stdout.log` and `actual-failure-independent-review.json`.
The original runner records `browser_executed: false` after the failed phase. This flag does not describe whether execution started.
The separate review records the correct distinction: browser execution occurred, but browser acceptance failed.

### R11 proxy correction and provider restoration, 2026-10-09

The local proxy regression passed an actual verified HTTPS forwarding request and owned shutdown.
The next full sequence, tool `55504`, passed registration, preparation, proxy readiness, initialization, and application startup.
It stopped during the original provider login. No upgrade, backup cutover, or old-format restoration acceptance was established.
The retained browser result identifies only the login stage. It does not establish the precise cause.
Evidence: `/var/tmp/rom-r11-app-a5d962c34e7801fb5324176d/accepted-login-traffic-result.json`.

Separate recovery, tool `32233`, exited zero. Root compared the complete original, registered, and restored provider records.
Recovery-after equals original-before. Recovery-before equals registered-after. All original provider fields were restored.
The recovery process closed, and its current cgroup was empty. The application was not retried.
Evidence: `proxy-sequence-v3/recovery/root-actual-restoration-review.json` under the retained R11 application preparation.
The full upgrade and rollback gate remains open.

### Focus repair, planner witnesses, and login diagnostics, 2026-10-09

The private `rom-ui` alpha.2 candidate passed 26 focused and 82 full browser cases in Chromium and WebKit.
Root inspected the source repair, actual reports, packed components, physical completion, and current empty cgroups.
The repair retains valid editor focus and text selection when the details panel moves between desktop and compact hosts.
Invalid or deliberately departed focus is not restored. These results do not establish acceptance of the full ROM application.
Evidence: `.superpowers/rom-010-public-layout-acceptance-prep-20261009/root-upstream-alpha2-review.json`.

Root rejected the first manifest proposal before integration or publication.
Its YAML replacement removed unrelated dependency records and put package metadata under the wrong key.
The replacement proposal must preserve every unrelated package and snapshot record.
The original proposal remains available for the regression test. Maintained manifests were unchanged during this review.

SQLite 3.53.4 units passed 76 tests, including all seven earlier planner failures. Five tests were ignored.
Root independently validated all 14 retained query-plan witnesses against the unchanged strict validator.
A separate exact test also passed and emitted all 14 witnesses.
Two coordinator parsing failures remain preserved: an interleaved record and the standard `0 measured` summary field.
Neither failure establishes a product test failure. Shared and consumer integration tests still require execution.
SQLite 3.53.2 compatibility and maintained-source integration remain open.
Evidence: `engine-profile-v3/root-retained-eqp-review.json` under the native execution preparation.

Root executed all nine scripted login-diagnostic tests. They passed without a live provider or browser login.
The diagnostic distinguishes TLS navigation, missing forms, and refused callbacks without recording credentials or raw private URLs.
The previous live login failure remains unexplained until the next actual sequence supplies this evidence.
The callback, authorization, traffic, and recovery acceptance conditions remain unchanged.

### Maintained verification and remaining integration failures, 2026-10-09

The maintained full verifier, tool `28328`, exited zero. All 451 Node tests passed.
Rust formatting, Clippy, tests, compile fixtures, documentation, authentication, and identity checks passed.
The before and after snapshots match across 3,675 source entries. The current process group and cgroup are empty.
Evidence: `.superpowers/rom-010-native-copy-execution-preparation-20261009/maintained-integration-v5/actual-final-witness.json`.
This verifies current source. It does not verify final 0.1.0 packages, deployment, or load acceptance.

Both SQLite engine profiles passed 76 unit tests and all 14 strict query-plan witnesses.
SQLite 3.53.4 also passed 401 shared and consumer tests across the outer Cargo suites.
Nested child reports are not counted again. The maintained planner change preserves unknown-version fallback and exact plan checks.

The public alpha.2 composition consumer passed 14 browser cases and failed two.
Both failures concern editor focus when an open compact panel returns to desktop.
The Sheet hides its old host before the relocation action captures focus.
A private alpha.3 candidate is under investigation. Its full upstream suite passed 81 cases and failed one WebKit focus case.
The combined consumer built successfully, but its browser command lacked the WebKit executable setting and refused before tests.
The next private combined attempt installed alpha.2 through a stale transitive lock. Its 14 passes and two failures are not alpha.3 evidence.
An installed-source comparison is now required before the corrected candidate's browser phase.
Neither the private package nor a weakened browser assertion establishes acceptance.

The next R11 sequence, tool `7082`, stopped at the original provider callback with HTTP 401 instead of 303.
The diagnostic recorded no request failures. This narrows the observed boundary but does not identify the exact rejection step.
Source inspection found an older Host compatibility limit: its JWK parser rejects metadata and key identifiers in the provider fixture.
This source evidence is separate from attribution of the actual callback refusal.

Recovery tool `4423` exited zero. Root compared the complete saved and restored provider records byte for byte.
The original configuration was restored, and the recovery process and cgroup were empty.
Root requested the existing provider's owned stop. Provider tool `63443` then exited zero.
Evidence: `proxy-sequence-v3/diagnostic/next-v12/recovery/root-restoration-stop-review.json` under the retained R11 preparation.
The upgrade and rollback acceptance gate remains open.

### Matching candidate versions, 2026-10-09

The maintained Rust, Studio, and extension-profile versions now identify the 0.1.0 candidate.
The change affects 26 files, including eight Cargo locks. Historical source snapshots and evidence remain unchanged.
Full locked, offline Cargo metadata passed for all 22 workspace packages and seven separate lock roots.
Registry package blocks and checksums remain byte-identical to the pre-change locks.
The first fresh lock resolution changed registry dependencies. Root rejected it and retained its output before restoring the pinned graph.
Evidence: `.superpowers/rom-010-final-producer-prep-20261009/matching-version-change/`.
This is a source-version change, not publication or packaged acceptance. The complete verifier must run after the remaining source changes.

The producer now checks the Rust workspace version against the declared release profile before any gate runs.
The installed AI consumer uses the same source-version helper.
The regression first reproduced acceptance of a committed Rust 9.9.9 fixture with the unchanged Studio and release-profile versions.
After correction, all 35 affected package, producer, archive, and AI provenance tests passed in the configured development container.
The first host-side check failed because `rustc` was absent from PATH; it is not a product acceptance result.
Evidence: `matching-version-change/current-source-review.json` under the final producer preparation.

The complete Node suite then passed all 452 tests, with no skipped cases.
The first attempt found three consumer locks that still identified the extracted Studio package as 0.0.3.
Root changed only those own-package version records to 0.1.0 and preserved every other dependency value.
The failed attempt and correction are retained in `matching-version-change/consumer-locks/` and `all-node-v2-review.json`.
The complete Rust verifier and packaged acceptance remain separate, unfinished gates.

Root reread the latest referenced Astral Plane conversation and inspected consumer revision `71fb093608d05bcf4edd6ecdf1381540166797c3`.
The consumer now uses wider expandable conversation panels, independently scrolling history, and an always-visible composer.
These patterns map to existing AP-UX-003, AP-UX-009, AP-UX-011, and AP-UX-022 requirements.
Reported consumer deployment tests are not current ROM package acceptance. Application-specific calculation changes remain outside ROM.
Evidence: `latest-consumer-ui-source-review.json` under the final producer preparation.

### Private editor lifecycle and current load build, 2026-10-09

The second private UI candidate passed all 82 upstream browser cases and all 16 composition cases on both engines.
The composition run checked the installed archive's source and manifest before execution.
These results preserve the original focus, selection, draft, history, composer, and geometry requirements.
Additional boundary tests then passed 20 cases and failed two Escape-crossing cases.
One failure used a role lookup for an intentionally hidden editor. The other requires inspection of the actual Sheet transition.
Root requested stronger connected-origin destruction and stale-intent checks before publication.
The private candidate is not public package acceptance.
Evidence: `.superpowers/rom-010-public-layout-acceptance-prep-20261009/root-alpha3-v2-source-review.json`.

Root reviewed the current load compiler wrapper and requested two corrections.
The wrapper now witnesses its full executable helper import closure and preserves terminal evidence after a failed allocation check.
The fresh admission binds 477 current source entries and 13 executable helpers.
Root launched compiler session `80847` with a 900-second compiler deadline and a 930-second outer deadline.
The wrapper monitors an 8 GiB target-growth allowance and a 120 GiB free-space floor. This monitoring is not a filesystem quota.
The compiler remains live at this checkpoint. No new mixed workload, provider window, or detached-artifact acceptance is established here.
Historical seed data and its build identity remain separate from the current execution build.
Evidence: `.superpowers/rom-010-mixed-v13-preparation-20261009/root-compiler-admission.json`.

Compiler session `80847` subsequently exited zero. Root verified the exact compiler artifact and all 477 source entries again.
Both compiler wrappers are absent, and both owned cgroups are empty.
The target allocation grew by 91,803,648 bytes. The final observed free space was 196,034,056,192 bytes.
The public finish command then exited zero and preserved the detached 25,039,640-byte executable.
Root checked its inode, hash, source fence, and equality with the compiler output.
Its build identity SHA256 is `aaba067a31b8bee1e15730a6ad7a612e93194ba1a8f7bf604986400ba268b71d`.
Evidence: `root-compiler-terminal-review.json` and `root-detached-build-review.json` under the V13 preparation.
The mixed-load gate remains open; compilation and binary preservation do not prove workload acceptance.

A stronger private editor destruction test reproduced stale focus restoration on both browser engines.
The original editor host stayed connected after the relocation action was destroyed.
A pending focus intent then restored focus to that editor. Earlier tests did not preserve the relevant intermediate observation.
Root requested explicit intent cancellation and one pending owner per control in a separate private candidate.
No public UI package was changed because of the earlier passing composition suite.

The third private UI candidate passed 82 upstream, 22 boundary, and 16 composition browser cases.
Root verified all 22 referenced evidence hashes and three terminal process closures.
The connected-origin destruction regression now passes without discarding its pre-remount observation.
A final unpublished versioned archive is under preparation. Public package integration remains open.
Evidence: `root-alpha3-v3-full-review.json` under the private layout preparation.

The dual-provider application facades completed both approved offline Cargo metadata commands.
The accepted graph uses ROM 0.0.3; the current graph uses ROM 0.1.0.
The registry tuples remain within their respective immutable producer graphs.
Both commands exited zero, and source snapshots match before and after execution.
These checks establish graph resolution only. They do not establish compilation, login, or upgrade acceptance.
Evidence: `metadata-20261009-01/actual-summary.json` under the retained R11 dual-provider preparation.

### Public UI and failed mixed-load acceptance, 2026-10-09

The upstream UI release `v0.1.0-alpha.3` is public at commit `a596253f7fc060f1b8128e9a4f36dc96cab4327e`.
Its maintained verifier passed 106 browser cases, 60 unit cases, and two installed headless cases.
The downloaded archive matches the reviewed candidate SHA256 `c91fb9c4c70aa35c7101e756bab9ea9e7dafffc89002acd5f8934035197caf07`.
ROM integration and packaged consumer acceptance remain separate gates.
Evidence: `.superpowers/rom-010-public-layout-acceptance-prep-20261009/upstream-alpha3-release-witness.json`.

The actual SQLite V13 mixed workload failed before cleanup.
The client attempted 12,321 HTTP requests. Only 3,646 requests reached the proxy.
An upstream `EPIPE` set the proxy's global failure state and closed its listeners.
Later transport failures include this cascade. Earlier overloads, denied or unknown outcomes, and eight stream errors remain separate failures.
The proxy did not exhaust its recorded request, byte, or concurrency budgets.
The final ledger contains 10,776 completed work records. Acceptance requires 12,000.

Independent inspection confirmed process closure, vacant workload ports, and the unchanged original database hash.
The source and detached binary fences also match. These checks do not change the failed workload verdict.
Root requested an owned stop of the unused provider window. Session `88667` subsequently exited zero with empty owned cgroups.
The redb mixed workload has not started. No workload oracle was weakened.
Evidence: `.superpowers/rom-010-mixed-v13-preparation-20261009/sqlite-v13-failure-diagnosis.json`.

The accepted dual-provider fixture failed compilation with two `E0308` errors.
Its idempotency calls passed owned strings where the API expects borrowed strings.
The original source and failed build output are retained. The current-version compile did not start.
The correction borrows the two formatted strings without changing the public API or provider contracts.
Root reviewed a separate bounded compile retry. Compilation does not establish login, upgrade, or rollback acceptance.
Evidence: `.superpowers/rom-010-final-producer-prep-20261009/root-dual-borrow-retry-review.json`.

The corrected accepted-version application fixture compiled and produced a preserved executable.
The current-version compile then stopped because its allocation scan reported disappearing temporary object files during linking.
The monitor requested owned termination. The current wrapper closed with `SIGTERM`; its owned cgroup is empty.
The last successful scan recorded 469,970,944 allocated bytes and 190,789,427,200 free bytes.
No current binary acceptance follows from this interrupted run. The failed scan and compiler output remain retained.
A bounded monitor correction and a separate retry require review before execution.
Evidence: `build-20261009-02/current-terminal.json` under the retained R11 dual-provider preparation.

An unchanged-proxy regression reproduced global teardown in 1.98 seconds.
An abrupt upstream close returned 502. The subsequent unrelated request failed with `ECONNREFUSED`.
This reproduction records `ECONNRESET`, not the original workload's exact `EPIPE` errno; both use the same failure handler.
A graceful 503 response served as a negative control and did not stop unrelated requests.
The proposed diagnostic correction retains a failed verdict after transport errors while separating that verdict from listener shutdown.
Security and budget violations must still stop the fixture. This correction does not relax mixed-workload acceptance.
Evidence: `.superpowers/rom-010-mixed-v13-proxy-diagnosis-20261009/abrupt-original-red.log`.

The fresh ROM 0.1.0 consumer installed the public UI alpha.3 archive through locked dependency installation.
Its type checks, build, and installed-source checks passed. All 16 composition cases passed on Chromium and WebKit.
Root independently verified 563 input hashes, both empty owned cgroups, absent wrappers, and the vacant preview port.
The retained consumer uses 356,958,208 allocated bytes. This proves scoped UI composition, not complete release acceptance.
Evidence: `.superpowers/rom-010-public-layout-acceptance-prep-20261009/alpha3-ROM-integration/root-terminal-review.json`.

### Recovery build, HTTP deadline, and legacy login, 2026-10-09

The current recovery host compiled with the explicit SQLite 3.53.4 build profile.
The native engine test reported SQLite 3.53.4. Root verified 523 source entries and the preserved executable.
Both compiler phases exited zero. Their owned process groups are absent, and their recorded cgroups are empty.
This result establishes compilation and engine selection. Runtime recovery acceptance remains open on SQLite and redb.
The default bundled SQLite profile remains separate from this explicit profile.
Evidence: `.superpowers/rom-010-public-layout-acceptance-prep-20261009/recovery-current-execution-prep/native-build/root-terminal-review.json`.

The first full verifier stopped after temporary compiler files disappeared during an allocation scan.
The second full verifier reached an actual Node test failure: a stalled HTTP response body exceeded its test deadline.
Both failed runs remain retained. Neither run establishes full verifier acceptance.
Evidence: `final-maintained-verifier-prep/root-terminal-review.json` and `final-maintained-verifier-v2/root-terminal-review.json` under the private layout preparation.

A controlled experiment reproduced the HTTP timeout failure with forced garbage collection.
Direct cancellation of the retained body reader made the regression pass without changing its timing assertions.
The agent recorded 34 passing affected checks. Root independently reviewed six source files and reran all 34 checks; they passed.
Full verifier acceptance remains open.
The controlled experiment establishes this failure mechanism. The earlier full verifier did not record garbage collection at the failure point.
The load source inventory now has 481 entries. The earlier 477-entry build identity remains historical and cannot establish current workload acceptance.
Evidence: `.superpowers/rom-010-http-stalled-body-diagnosis-20261009/diagnosis.md` and `final-source-freeze.json` in the same directory.
Independent review: `root-affected-review.json` in the same directory.

The accepted 0.0.3 application completed a genuine identity-provider callback with status 303 and created an authenticated owner session.
Its subsequent public client traffic test failed. The cause remains unproven; the browser report recorded no 4xx or 5xx responses.
The original test combined several operations in one browser evaluation. Separate diagnostic stages are under preparation.
Independent inspection found absent process groups and vacant fixture ports.
The failed run did not retain every phase's cgroup identity or physical termination record. This evidence gap remains explicit.
The current-version smoke test and full upgrade/rollback sequence have not passed.
Evidence: `legacy-smoke/accepted-failure-independent-closure.json` under the retained R11 dual-provider preparation.

The third full verifier used 3,681 source entries and passed all 456 Node checks.
It then failed the SQLite late-header recovery case in `rom-ai` after a two-second wait for work completion.
The other 51 cases in that test suite passed. The cause of this timeout remains unproven.
Root confirmed exit code 101, physical termination, an empty cgroup, and unchanged source.
The recorded target growth was 1,263,792,128 bytes. The failed run remains retained; full verifier acceptance remains open.
Evidence: `.superpowers/rom-010-maintained-verifier-v3-preparation-20261009/root-terminal-review.json`.

### Recovery execution and AI test diagnosis, 2026-10-09

The installed recovery consumer passed 20 browser cases and two HTTP suites across SQLite and redb.
The SQLite host used the explicit 3.53.4 profile. Root verified six successful phases, current source hashes, and installed browser graphs.
All six phases recorded physical exit zero and empty owned cgroups.
This staged execution does not establish canonical clean-release admission or complete upgrade acceptance.
Evidence: `.superpowers/rom-010-public-layout-acceptance-prep-20261009/recovery-current-execution-prep/installed-runtime-v2/root-terminal-review.json`.

The AI recovery timeout occurred again with the original parallel test suite. The original serial suite passed.
A diagnostic run captured six leased source records without pending work. Their due times did not exceed the test clock.
The runtime did not report failure. This evidence supports scheduling sensitivity in the synchronous test poll, not proven mutex starvation.
The correction replaces that poll's `yield_now` with a one-millisecond sleep. The two-second timeout and recovery assertions remain unchanged.
Three temporary parallel runs passed all 52 cases each. Root then independently ran the maintained suite: all 52 cases passed.
Root verified 576 current source entries, physical exit zero, an absent wrapper, and an empty owned cgroup.
The earlier failures remain retained. Full verifier acceptance remains open until the release producer sources are stable.
Evidence: `.superpowers/rom-010-work-flows-drain-diagnosis-20261009/affected-maintained/root-terminal-review.json`.
Diagnostic evidence: `captured-failure-state.json` and `fairness-trial-findings.json` in the parent directory.

The legacy consumer's safe integer decoder returns a number. Its fixture compared that number directly with a bigint.
The unchanged built fixture failed a scripted control. A private correction compares both values through a checked unsigned-integer conversion.
The correction rejects unsafe, negative, fractional, and out-of-range values. It does not change authorization, receipt, revision, or Resource identity assertions.
The corrected derivative passed type checks and build. Genuine provider traffic with this derivative remains untested.
Evidence: `legacy-smoke/diagnostic-v3/root-check-build-terminal-review02.json` under the retained R11 dual-provider preparation.

The current 481-entry load host compiled successfully and produced a preserved executable through the public build workflow.
Root verified current source hashes and both compiler process closures. Recorded target growth was zero bytes.
This compilation does not establish mixed-workload throughput or resilience acceptance.
Evidence: `.superpowers/rom-010-load-481-warm-build-preparation-20261009/root-terminal-review.json`.

The current R11 host also compiled against its fresh 575-entry source witness. The accepted 344-entry source witness remained unchanged.
Root verified physical exit zero, an absent wrapper, and an empty owned cgroup. Both earlier preserved executables remain unchanged.
Four NSS commands prepared fresh TLS materials for the accepted and current fixtures. They did not start providers, browsers, or ROM hosts.
Genuine login, upgrade, and rollback acceptance remain open.
Evidence: `build-20261009-04/root-terminal-review.json` and `legacy-smoke/diagnostic-v3/root-nss-terminal-review03.json` under the retained R11 dual-provider preparation.

### Genuine legacy-client traffic, 2026-10-09

The corrected private consumer completed genuine login and public traffic against both accepted and current hosts.
Each execution completed nine stages, a callback with status 303, and five receipt replays. The current host reported format 11.
Root verified 165 current input hashes per execution. Six owned terminal records per execution retain physical process closure.
The current cgroups are empty, wrappers are absent, and fixture listeners are vacant.
The original failed consumer remains retained. The correction changes its integer comparison, not ROM's wire decoder or mutation guarantees.

The accepted execution has an explicit root prelaunch-record gap.
Root's auxiliary listener inspection used a restricted helper with unrelated ports. That inspection threw before it saved its review record.
The next tool call still dispatched the authorized finite runner. The runner executed its own input, source, port, and cgroup gates.
Root reviewed the actual terminal evidence afterward. This does not create a missing prelaunch record or establish canonical release admission.
The current execution has a separate successful prelaunch review and independently verified terminal evidence.

These results establish scoped legacy-client login and traffic. They do not establish production Authentik behavior or the complete 23-stage upgrade/rollback sequence.
Evidence: `legacy-smoke/diagnostic-v3/root-accepted-runtime-terminal-review04.json` and `root-current-runtime-terminal-review04.json` in the same directory under the retained R11 preparation.

### Public engine build and upgrade TLS preparation, 2026-10-09

The public SQLite profile builder compiled the official 3.53.4 source and produced its static library.
Root verified the retained archive against the published SHA3-256 digest before execution.
Both compiler commands recorded physical exit zero. Their current owned cgroups are empty.
Root verified the frozen source and tool hashes, then independently recalculated the library and header hashes.
This result proves profile construction. It does not establish the Rust-linked engine or final release-package acceptance.
Evidence: `.superpowers/rom-010-public-sqlite-profile-independent-preparation-20261009/root-build-terminal-review.json` and `root-independent-terminal-audit.json` in the same directory.

Three separate NSS profiles were prepared for the baseline, current, and restored upgrade fixtures.
All six commands recorded physical exit zero and empty current cgroups.
The preparation did not start an identity provider, browser, or ROM host.
The complete 23-stage upgrade and rollback sequence remains open.
Evidence: `dual-provider/full-upgrade05/root-nss-terminal-review05.json` under the retained R11 preparation.

The previous mixed-workload material preparation failed before NSS execution or workload launch.
Its private-file reader rejected the public Playwright entry because pnpm uses a package symlink and shared hardlinks.
A narrow derivative verifies that one public alias, its physical inode, content hash, ownership, and alias identity.
Other private-file guards remain unchanged. Three independent controls passed.
The old failed preparation and closed provider window remain retained. A fresh provider window is required before another workload execution.
Evidence: `.superpowers/rom-010-mixed-v14-481-runtime-preparation-20261009/root-material-prelaunch-failure.json` and `.superpowers/rom-010-mixed-public-tool-witness-preparation-20261009/root-tool-test-review.json`.

### Current producer and example-version integration, 2026-10-09

The final producer source passed 123 focused controls. Static fixture validation does not establish operational R1–R14 acceptance.
The current public SQLite builder also compiled successfully under inherited `TAR_OPTIONS=--exclude=sqlite3.c`.
Its immutable extractor preserved the official source. Both compiler commands recorded physical exit zero and empty current cgroups.
Root verified all 68 frozen source entries and the produced profile. Rust-linked engine acceptance remains separate.
Evidence: `.superpowers/rom-010-public-sqlite-profile-current-execution-20261009/root-independent-terminal-audit.json`.

The fourth full verifier failed after workspace tests, documentation tests, and compile fixtures.
The AI example still required ROM 0.0.3 through a path dependency. The current path crate declared version 0.1.0.
The maintenance example had the same stale requirements. This was a version mismatch, not a missing offline registry entry.
Root verified physical exit 101, an absent wrapper, empty current cgroups, and unchanged executed source.
Recorded target growth was 358,772,736 bytes. The original failure remains retained.
Evidence: `.superpowers/rom-010-maintained-verifier-v4-preparation-20261009/root-terminal-review.json`.

Both example manifests now require the current ROM package version. Their lockfiles were refreshed offline.
An independent comparison confirmed that only local ROM package versions changed. External dependency records remain unchanged.
This correction still requires a new full verifier execution.
Evidence: `.superpowers/rom-010-example-version-alignment-20261009/root-lock-review.json`.

### Full verifier and linked-engine review, 2026-10-09

The fifth full verifier passed against 3,694 source entries after the example-version correction.
All 476 Node controls passed. Root independently verified physical exit zero, empty current cgroups, and unchanged source and tools.
Recorded target growth was 492,355,584 bytes. This result applies to that executed source, not a clean-source release artifact.
Evidence: `.superpowers/rom-010-maintained-verifier-v5-preparation-20261009/root-terminal-review.json`.

Independent producer review found two issues: stale v2 instructions and a Rust-linked engine witness that measured only the version.
A controlled execution confirmed that the old test accepted an intentionally incorrect source ID for the same version.
The corrected Rust test measures the actual source ID and compile options. Selected profiles require the declared source ID and required options.
The incorrect-source control now fails with the intended assertion. Both default-profile tests pass.
The producer parser also rejects missing or duplicate identity lines, incorrect source IDs, and absent required compile options.
Its affected artifact and archive controls passed 125 tests. These controls do not establish selected-engine runtime or final package acceptance.
The producer instructions now describe v3 requirements, inputs, output location, and execution bounds.
A new full verifier is required for these changes.
Evidence: `.superpowers/rom-010-linked-engine-identity-root-20261009/root-targeted-review.json` and `.superpowers/rom-010-producer-v3-independent-review-20261009/report.md`.

### Latest verifier and provider closure, 2026-10-09

The sixth full verifier failed in the operator reconciliation suite. Eighteen tests passed and one failed.
The failed assertion expected `Scheduled` and received `Unresolved`.
Root verified physical exit 101, an absent wrapper, empty current cgroups, and unchanged source.
Recorded target growth was 53,248 bytes. This execution does not establish full-verifier acceptance for the current candidate.
The fixture gives immediate-result verifiers the same 20 ms deadline as its timeout case.
Scheduling sensitivity is a hypothesis. The retained failure does not distinguish a timeout from a callback panic or invalid result.
Evidence: `.superpowers/rom-010-maintained-verifier-v6-preparation-20261009/root-terminal-review.json`.

A fresh identity provider window started successfully. The GET-only pre-test read matched the retained complete provider configuration.
The mixed-workload material then failed before NSS execution or Host launch.
The public Playwright file had unchanged content, SHA256, canonical path, device, inode, and package alias identity.
Its hardlink count changed from 27 to 29. Its `ctime` also changed.
The strict historical witness comparison rejected that change. No evidence identifies the process that changed those attributes.
No mixed-workload acceptance or automatic retry followed.
Evidence: `.superpowers/rom-010-public-tool-drift-diagnosis-20261009/comparison.json` and `.superpowers/rom-010-original-provider-public-tool-preparation-20261009/root-material-prelaunch-failure.json`.

The GET-only post-test read matched the actual pre-test snapshot. Both snapshots have SHA256 `64cf9b91767d768e79e06725bdc3a7322399c9311d65c43ebaf4650962b8d6d5`.
Root requested the actual owned stop file and observed provider tool 70146 terminate with exit zero.
Root then verified empty current cgroups, absent owned processes, unchanged sources, three stopped containers, and vacant fixture ports.
This proves closure of that provider window. It does not prove a database rollback or a completed load trial.
Evidence: `.superpowers/rom-010-original-provider-public-tool-preparation-20261009/root-state-equality-and-stop.json` and `root-provider-terminal-review.json` in the same directory.

Root also verified the full-upgrade source handoff without executing it.
Its 22 private source files and 202 static inputs match their recorded sizes and hashes.
The complete 23-stage upgrade and rollback sequence still requires a separate fresh provider window and runtime acceptance.
Evidence: `dual-provider/full-upgrade05/root-source-review10.json` under the retained R11 preparation.

The reconciliation failure did not recur in three isolated controls, three parallel suites, one serial suite, or one observed parallel suite.
The original cause remains unproven. The first observer failed compilation; its error and source remain retained.
The corrected observer completed all 19 tests. Original source was restored before the maintained fixture change.

The fixture now separates semantic result tests from its deliberate timeout case.
The pending verifier keeps its 20 ms deadline. Other modes use 2 seconds, matching the existing race fixture.
All result, receipt, snapshot, retry, and provider-send assertions remain unchanged. Production code and deadlines remain unchanged.
The affected parallel suite passed all 19 tests. Root verified current source, physical exit zero, and empty current cgroups.
This is a fixture contract correction, not proof of the original failure's cause. A new full verifier is still required.
Evidence: `.superpowers/rom-010-operator-reconciliation-diagnosis-20261009/root-affected-terminal-review.json` and `findings.md` in the same directory.

## 2026-10-09: verifier V7 and failed mixed SQLite trial

The subsequent full local verifier completed with physical exit zero and 478 passing Node tests.
Its 3,694 selected source entries stayed unchanged during execution. The current owned cgroup was empty after execution.
This accepts that executed local verifier. It does not accept a clean-source package or subsequent source changes.
Evidence: `.superpowers/rom-010-maintained-verifier-v7-preparation-20261009/root-terminal-review.json`.

The actual mixed SQLite trial completed with exit one. Its client recorded 12,487 HTTP requests.
The outcomes were 5,045 successes, 6,201 denials, 1,240 overloads, and one timeout.
The post-traffic operator capability request was denied after two successful initial inspection requests.
Streams recorded four overload errors and four early EOF errors.
The final work ledger contained 11,025 Done records, below the required 12,000.
These results fail the unchanged workload oracle. They do not establish SQLite as the cause.

The proxy recorded one upstream EPIPE and retained a failed verdict.
Its fatal-shutdown flag was false; its final socket and upstream counts were zero.
The coordinator's `proxy-drain-failed` label does not establish a process or connection leak.
Root independently verified empty current cgroups, absent owned PIDs, vacant ports, stopped provider containers, and unchanged provider state.
The failed trial and copied data remain preserved. No redb mixed trial was admitted after this failure.

Evidence is under `.superpowers/rom-010-mixed-prospective-public-tool-preparation-20261009/`:
`mixed-v14-sqlite-outer-result.json`, `root-provider-terminal-review.json`, and `root-state-equality-and-stop.json`.
The actual coordinator result is `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-mixed-cbc1f723265355425ae8c111/result.json`.
Independent closure diagnosis is under `.superpowers/rom-010-r10-proxy-failure-diagnosis-20261009/`.

The owner requires primary-source web research, a bounded reproduction, and a verified correction before accepting the cause.
The credential mutex and proof-expiry interaction is a hypothesis, not a confirmed explanation of this trial.
Token expiry, current authority, authentication admission, provider acquisition, and transport failure require separate controls.

The separate full23 watcher passed 15 pure Node controls in a root rerun.
No full23 watcher, provider, or upgrade sequence was launched by those controls.
Fresh source and build bindings remain required before operational admission.
Evidence: `.superpowers/rom-010-full23-owned-watcher-preparation-20261009/handoff.md`.

## 2026-10-09: reproduced credential expiry race and affected verification

A deterministic test reproduced the credential race on the actual `Credentials::actor` method.
The test held its cache mutex, queued a caller at time 129, and advanced the shared trusted clock to 130.
The original token remained valid until 400. The cached proof expired at 130.
Original code used its pre-wait time and returned `Denied` instead of renewing the proof.
The actual RED run compiled successfully: two controls passed and this regression failed.

The correction acquires the cache mutex before sampling the trusted clock.
Original token expiry, proof lifetime, signature validation, provider activation, and current Resource authority remain unchanged.
The same three tests then passed. Two additional negative tests cover queued token expiry and a corrupted signature during renewal.
The complete affected Host suite passed 49 tests, including all five credential controls.
Root independently verified source equality, physical exit zero, absent wrapper, and an empty current cgroup.

Evidence: `.superpowers/rom-010-auth-expiry-red-execution-20261009/root-red-review.json`,
`.superpowers/rom-010-auth-expiry-green-execution-20261009/root-green-review.json`,
and `.superpowers/rom-010-auth-expiry-affected-host-execution-20261009/root-affected-review.json`.
The [primary-source diagnosis](rom-0.1.0-mixed-auth-load-primary-source-diagnosis-2026-10-09.md) distinguishes evidence from hypotheses.

This proves the narrow regression and its correction. It does not attribute every earlier load error to that regression.
The full local verifier, fresh load binary, and unchanged mixed workload remain required before release acceptance.

## 2026-10-09: complete verifier and fresh post-fix load build

The complete local `./scripts/check` passed after the credential correction and five regression controls.
Root verified 3,699 executed source entries against the current source at terminal review.
The wrapper exited zero and disappeared. Its current cgroup was empty.
Target allocation grew by 737,280 bytes during this verifier.
Evidence: `.superpowers/rom-010-maintained-verifier-v8-preparation-20261009/root-terminal-review.json`.

A fresh release load host then compiled successfully from 482 captured source entries.
Compared with the historical 481-entry build, only `oidc.rs` changed and `credentials_tests.rs` was added.
Root verified the compiler and parent exits, source fences, empty current cgroups, and the preserved binary.
The preserved binary contains 25,044,816 bytes and has SHA256 `9c06632ffcfd5f6388285777e7a8f7a952e01c657d82ae09fe8d99381139c2c4`.
Its build used two jobs, offline dependencies, and a 900-second compiler deadline.
The target growth monitor observed zero allocation growth. It is not a filesystem quota.
Evidence: `.superpowers/rom-010-load-post-auth-fix-warm-build-preparation-20261009/root-terminal-review.json`
and `.superpowers/rom-010-load-post-auth-fix-warm-build-preparation-20261009/root-binary-review.json`.

These results establish complete local verification and a fresh load binary, not mixed-load or release acceptance.
The unchanged mixed workload still requires 13,280 successful requests, zero HTTP or SSE errors, and 12,000 Done records.
SQLite and redb, genuine provider upgrade and rollback, final clean-source artifacts, and deployment remain release gates.
This ledger update occurred after the verifier source snapshot.

## 2026-10-09: unchanged post-fix mixed trial failed

The fresh 482-source SQLite trial failed. The client recorded 12,170 HTTP requests:
1,684 successful, 9,181 denied, and 1,305 overloaded. It recorded no timeout or unknown outcome.
All measured read, commit, replay, query, and reservation requests were denied after the warmup.
Operator inspection recorded two successful requests and one denied request.
Four normal stream readers recorded four server overload events and four early EOF errors.
The admitted original token had 298,000 milliseconds remaining against a 225,000-millisecond requirement.

The proxy recorded no failure or EPIPE in this trial.
Thus this result does not establish stream proof expiry as its stream failure cause.
The earlier mutex correction remains regression-tested, but it did not make this workload pass.
A second hypothesis concerns proof expiry during awaited current-identity binding, followed by session removal.
Its reproduction remains pending. Do not attribute the trial to this hypothesis before that test.

Root verified terminal process exits, four empty current cgroups, three stopped provider containers, and vacant owned ports.
The failed workload exited one. The provider lifecycle exited zero.
The same-run full provider configuration was equal before and after the trial.
The original seed remained unchanged, source fences passed, and unmount exited zero.
Redb was not admitted after this SQLite failure.

Evidence: `.superpowers/rom-010-mixed-post-auth-fix-preparation-20261009/root-failure-summary.json`
and `.superpowers/rom-010-mixed-post-auth-fix-preparation-20261009/root-runtime-and-provider-terminal-review.json`.
The private independent attribution review is `.superpowers/rom-010-post-auth-fix-failure-attribution-20261009/report.md`.
The failed trial and its artifacts remain preserved. Closure is not workload or release acceptance.
