# Task 3 artifact author freeze

Status: source frozen on 2026-10-03, revised after the scoped metadata review; no commit or real release invocation was made.
Baseline: `60960510d455bee77906e4cb9807778a21f4082c`.
Ownership: `scripts/release` and `scripts/release-artifacts/` only.
The coordinator owns support/audit prose, packaged application helpers, shared check wiring, integration, and final release execution.

## Result and boundaries

The shell entrypoint delegates to named artifact modules.
The unchanged no-argument route now produces an exclusive directory under `dist/rom-VERSION-SHORT_REVISION/`.
One explicit output-directory argument is supported. There is no CLI or environment gate-skip route.
Existing outputs, including empty directories and dangling links, are rejected without replacing them.
Old flat archive files remain untouched.

The production producer always selects six commands: check, release build, default demo, actual-provider demo, skills, and packages.
It captures source HEAD/tree, tracked paths/digests/modes, selected skills identity, native profile, lock, and host tool versions.
It rechecks the source after the gate and immediately before directory publication.
It creates source and skills archives, retained gate logs, a versioned JSON manifest, and complete checksums inside a private sibling stage.
The manifest hashes payloads; the checksum file hashes all files except itself, including the manifest and logs.

Archive verification checks safe paths/types before extraction.
It checks every extracted source file and executable mode, the Git archive commit marker, and the reconstructed Git tree.
Git performs reconstruction in an isolated repository with filters disabled and inherited Git configuration removed.
The complete artifact inventory has no cache-directory exclusions.
All four extracted bundle preflights must accept the extracted source.
This admission does not claim that every workflow executed from the extraction.

The selected skills identity covers workspace Cargo.toml/Cargo.lock and Rust/Cargo files under crates, demo, and examples.
It excludes `tests/persistence` and other checkout files.
The captured Git tree and source archive digest are separate identities.
The declared complete native profile, package version, and complete selected identity must match the extracted source.

The production path requires a usable clean ordinary Git checkout.
The container's mounted worktree Git pointer is unsuitable for actual release execution.
The coordinator will run the producer from a final integrated clean checkout.
The author did not attempt a real release from this dirty worktree.

## Quality and shared mechanisms

Named modules separate source fencing, fixed gates, directory publication, archives, complete contents, manifest/checksums, verification, and Git tree reconstruction.
The main entrypoint and shell wrapper are small.
Tests occupy three named modules and one shared fixture support module.
No Rust API or crate-root implementation changed.

The producer reuses accepted skills assembly, profile/asset admission, selected source identity, and process-group ownership.
It uses a separate complete contents walker because artifact closure differs from the skills' filtered source identity contract.
No copied skills codec, fixture, or compatibility contract was introduced.
No new runtime, Rust dependency, storage format, provider policy, or publication permission was introduced.

## Host and process contract

Supported publication profile: Linux, GNU no-copy/no-replace move, ext4, with `findmnt` available.
The observed utility is GNU coreutils 9.5.
Runtime admission requires ext4 and probes preservation of an existing destination.
The stage and destination parent must share a filesystem.
Publication uses `mv -T --no-copy --update=none-fail`; an existing destination fails rather than copying or replacing it.
Concurrent contenders expose one complete directory and retain the losing stage.

The primary probe records an actual successful `renameat2(..., RENAME_NOREPLACE)`.
Preserved evidence: `../task3-artifacts/evidence/publication-capability-probe.json` and `publication-capability.trace`.
The original remains at container `/var/tmp/rom-release-mv-capability-PFywD6/`.
The probe observed initial success, existing-output rejection, and concurrent exits 0 and 1 with one complete winner.
These results do not claim adversarial-host isolation or power-loss durability.

Each gate receives two Cargo build jobs, a one-hour deadline, and a combined 32-MiB output limit.
The shared helper owns a Linux process group; its already accepted descendant and output-budget regressions were rerun.
Primitive archive/source utilities have shorter finite command limits.
Source archive buffers are bounded at 128 MiB.
Failed gates, bounded aborts, and spawn failures retain command results in private incomplete evidence.
The final output never contains a partially copied tree.

## Executed evidence

All author tests ran in `rom-dev` against the shared worktree.
No Rust compilation or shared Cargo target mutation was needed for these Node tests.
Fixture repositories and publication stages are private, fresh ordinary Git repositories on `/var/tmp` ext4.
Their runners return finite synthetic gate results through the directly imported module API.
Those results prove producer sequencing and failure handling; they do not prove the six real verification commands passed.
The production CLI never exposes runner injection.

| Check | Observed result | Evidence under `../task3-artifacts/evidence/` |
| --- | --- | --- |
| Initial producer API | Missing-module API RED | `api-red.log` |
| First producer scenarios | Five passed | `first-green.log` |
| Dangling destination | Behavioral RED; rejected only after gates | `publication-red.log` |
| Output admission fix | Twelve passed | `publication-green.log` |
| Commit identity and complete inventory | Behavioral RED: expected rejection missing | `verification-red.log` |
| Verification fixes | Fourteen passed | `verification-green.log` |
| Altered Git tree identity | Behavioral RED: expected rejection missing | `tree-red.log` |
| Tree reconstruction fix | Fourteen passed | `tree-green.log` |
| Unsupported filesystem | Behavioral RED: expected rejection missing | `filesystem-red.log` |
| Final artifact and shared skills suite | Seventeen artifact plus seven shared skills tests passed | `final-checks-green.log` |
| All owned Node syntax and shell syntax | Passed in the same final command | `final-checks-green.log` |
| Declared profile/version coherence | Behavioral RED for altered format; profile, feature, and package-version negatives pass after repair | `profile-red.log`, `profile-green.log` |
| Complete selected inventory | Behavioral RED for omitted selected entry; repaired full-object equality | `selected-inventory-red.log` |
| Revised final artifact and shared skills suite | Nineteen artifact plus seven shared skills tests passed; owned Node/shell syntax passed | `metadata-final-green.log` |
| Owned whitespace | `git diff --check` passed | Executed before freeze |

The original 17 artifact scenarios cover dirty tracked/untracked source, existing output, unusable Git, obsolete profile, gate failure, changed lock/HEAD,
late output collision, complete successful extraction/admission, traversal/link rejection, changed declared file/commit/tree identity,
incomplete gate metadata, tampered checksums, extra ordinary/cache files, unsupported utility/filesystem, dangling output, empty output, and concurrent contenders.
Each fixture preserves its source and small artifact/log directories for diagnosis.
The two later tests cover complete declared profile/package-version correspondence and the complete selected source inventory.

## Corrections and remaining gates

The first combined final run used fixtures under Node's `/tmp`, which is tmpfs in this container.
The new intentional ext4 requirement correctly rejected those fixtures.
The test helper now defaults to `/var/tmp`, with `ROM_RELEASE_TEST_TMP` for another existing ext4 parent.
The failed run remains in `tmpfs-fixture-failure.log`; the producer constraint was not relaxed.

Independent review reproduced a declared format-9 manifest accepted against an extracted format-8 profile.
The coordinator also identified omission of entries from the declared selected inventory while its digest fields remained unchanged.
Both discrepancies were independently reproduced with behavioral RED.
The verifier now compares the full declared profile and selected identity against fresh extracted values, and binds package version explicitly.
This is metadata coherence, not signing or an artifact authenticity claim.
The original freeze is preserved in `before-metadata-fix/`; the live author package contains the revised 16-file freeze.

`owned-files.txt` lists all 16 frozen files, including the modified shell entrypoint.
`source-sha256.txt` identifies their exact contents; `source/` contains complete copies.
`untracked.txt` is necessary because all artifact modules/tests are new.
Scoped re-review, the coordinator's complete source/application/provider/package gate, clean integration, and actual release execution remain pending.
Only the actual producer run can supply the final committed artifact paths and acceptance manifest.

The script README uses the repository's STE fallback guidance.
Technical terms and exact command names remain unchanged; no official dictionary verification or compliance certificate is claimed.
