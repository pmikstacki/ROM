# Installed AI consumer validation

## Result and scope

On 2026-10-08, the isolated installed-consumer command completed with exit code 0.
The consumer used 17 extracted Cargo packages from one frozen provisional producer.
The producer contained 3,529 files and 34,771,824 source bytes.
Its manifests still declare version `0.0.3`.
This result is not acceptance of final ROM `0.1.0` release artifacts.

| Subject | Observed result |
| --- | --- |
| Public publication and ticket-triage examples | 17 tests passed on SQLite and redb |
| Extracted `rom-ai` | 165 tests passed |
| Extracted `rom-openrouter`, with `test-support` | 44 tests passed |
| Public `ReadContext` compile fixture | Read/query and action-route fixtures compiled; `execute` failed with exactly E0599 at `cannot_execute.rs:4` |
| Command lifecycle | All 17 command records report exit code 0 and `groupAbsent: true` |
| Final package and consumer inputs | Expected and observed file inventories match |

The runtime oracle requires the named domain and recovery tests, not only successful summary lines.
The OpenRouter oracle requires both real-HTTP alternate-endpoint tests.
The tests do not contact a paid provider.
The browser results below cover progress and recovery through an application-owned transport.
Original application adoption and human usability remain separate acceptance work.

## Installed browser composition

The coordinator ran the same frozen consumer on SQLite and redb.
Each adapter passed 12 Chromium cases and 12 WebKit cases: 48 browser cases in total.
Each run also passed installation, Svelte type checking, and production building.
All eight command records report exit code 0 and `groupAbsent: true`.
The 27 fixture tests also passed in a separate coordinator execution.

The consumer installed an independent Studio source snapshot with 515 files.
Its native host used the unchanged extracted packages from the provisional producer described above.
The coordinator verified current fixture, copied inputs, native sources, package archives, and binary identities before and after each run.
The host execution used translated container paths with identical source and binary bytes.
It did not rebuild the native host for that path translation.

The cases cover progress, uncertain acknowledgements, exact retries after reload, restart, stale revisions, concurrent recovery, budgets, cancellation, and delayed responses after logout.
Three fresh grant cases distinguish tool, row, and owner revocation.
Tool or row revocation records `Failed(Denied)` without another read or effect.
Owner revocation returns 403.
Restored grants do not revive the failed flow; its recovery control remains disabled.

Earlier tests expected 403 for all three revocations.
Actual native results exposed that incorrect expectation; all failed traces remain preserved.
The corrected cases check safe projections, unchanged effect counters, and durable terminal state.
They do not weaken the domain contract to make the result pass.

The selected browser runs used actual Chromium and WebKit executables on the host.
The fixture transport does not establish production authentication or external-provider acceptance.
Restart tests do not establish power-loss durability.
These results must be repeated against matching final `0.1.0` packages.

The output directories are `/var/tmp/rom-010-ai-browser-sqlite-v7-host-20261008/` and `/var/tmp/rom-010-ai-browser-redb-v1-host-20261008/`.
Root reviews are `browser-sqlite-v7-host-root-review.json` and `browser-redb-v1-host-root-review.json` under `.superpowers/rom-010-ai-installed-root/`.

## Producer and installation

The coordinator copied tracked and nonignored source files into an independent producer directory.
It preserved executable modes and recorded each file's SHA-256 digest.
Cargo generated the package archives offline, without native verification or manifest edits.
The consumer then extracted the source and package archives independently.
It used exact package versions and explicit local package patches.
Resolved ROM dependencies could not select the working checkout.

The runner checked compiler `1.99.0`, registry lock entries, graph paths, and input identities between commands.
Initial resolution could change only the subject's Cargo.lock.
Each registry entry had to match an admitted producer, package, or original consumer lock entry.
Subsequent commands used locked dependencies.
The final result retains `publication: false` and `acceptance_scope: provisional-working-source`.

## Defects found during execution

The first extraction attempt rejected valid Cargo archives because they omitted an explicit root-directory entry.
A dedicated crate extractor now accepts that layout while the source-archive extractor remains unchanged.
Twenty Node/tar checks cover archive layout, path and duplicate rejection, links, sparse sizes, and inherited `TAR_OPTIONS`.
Independent review checked the frozen helper and its logical-size boundaries.

Package inspection then found that `rom-ai` lacked LICENSE.
The coordinator added an exact copy of the repository's MIT LICENSE and generated a new producer.
All 17 new package archives passed payload, original-manifest, and license checks.
The earlier producer and failed evidence remain preserved.

A native attempt under the working checkout passed the example and compile stages, then failed standalone package resolution.
Cargo found the parent workspace and rejected the extracted `rom-ai` manifest.
The coordinator moved acceptance output and its target directory outside the checkout.
The same source and package archives then passed the complete native gate.
No package manifest was changed to bypass workspace discovery.

## Allocation and limits

Commands used offline dependencies, two Cargo jobs, and disabled incremental compilation.
The allocation had an outer limit of 2,400 seconds and explicit per-command time limits.
Each command had an 8-MiB combined captured-output limit.
The new target directory used 1,072,717,824 allocated bytes after completion.
The planned 12-GiB target allowance was an estimate, not an enforced disk quota.

The inherited command helper signals process groups and records their absence after completion.
It does not prove stable-identity containment of an escaped descendant that retains stdio.
No such failure was observed in these selected commands.
This limitation remains separate from the successful native result.

## Evidence

Root selection, actual failures, and the successful review are retained under `.superpowers/rom-010-ai-installed-root/`.
The successful review is `native-external-v3-root-review.json`.
It binds result digests, source options, compiler identity, test results, and all command outcomes.
The source and package witness is in `producer-v2-20261008/options-external-v3.json`.

Inside `rom-dev`, the successful output is `/var/tmp/rom-010-ai-installed-v3-20261008/`.
On the host, it is `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-ai-installed-v3-20261008/`.
Both `acceptance.json` and `final-input-observation.json` are preserved there.
Independent review is in `.superpowers/rom-010-ai-installed-independent-review/`.

These private evidence paths identify this local run; they are not distributed artifacts.
Repeat the gate against matching clean `0.1.0` packages before publication.
The remaining release requirements are in the [consumer research plan](rom-0.1.0-consumer-research-plan.md).
