# ROM 0.0.3 final artifact review — c66c470

## Verdict

The completed distribution and the persistent VPN preview both pass their available acceptance checks. I found no archive, source identity, lock, or gate-integrity blocker. The exact c66c470 artifact passes independent verification, and separate actual HTTPS UI checks passed both before and after a full container restart. The installed evidence bundle retains older “pending” metadata from before those later checks; treat the newer live reports as additive acceptance evidence, not as a rewrite of that immutable receipt.

I ran the documented `verifyArtifacts` entrypoint against `/root/ROM/dist/release-0.0.3-c66c470`. It returned exit status 0, `complete: true`, manifest version 2, profile `rom-studio-v2`, source commit `c66c470eb894b8449f9ab707d94179c88af09113`, and tree `f7503bfd37571376c2d54bdd48053c3394ab305a`. All eight manifest command results have exit code 0, no spawn failures, and no bounded abort. The verifier checked the source and Studio archives, file inventories, archive paths, lock identity, extracted skills preflights, and recorded acceptance contract.

The artifact includes source, skills, and Studio archives. Their SHA-256 values are:

- Source: `1b30aa8d6b5318739396f6d4c047a4544862f232d5ca19d8aebf022037097c6f`
- Skills: `3990de9207e6381c2fe6e2b7a642d35301aa6d721d887ce741a4ef99c24ea5ea`
- Studio: `556edf29d5d3bdf853c5cd1559105a833212bbfb8a499ef0ddac708dc20c8f1f`

The manifest has publication disabled, consistent with the repository's local release procedure. This review did not run builds or browsers.

## Preview identity and live acceptance

I independently recomputed every tracked-source and production-asset digest in the prior and current preview source facts. The inventories match their receipts. The bridge's binary, receipt, report, and log hashes also match. The prior report records 28 passing actual-host cases, zero unexpected/skipped/flaky cases, across SQLite/redb and Chromium/WebKit. That execution belongs to `553832325dbc602505646ae42bdc9bd7a2fde432`, not c66c470. The bridge says this clearly, and it lists the two external-author changes outside that 28-case harness.

This is valid continuity evidence for unchanged binary/assets/harness inputs. It is not fresh c66c470 browser execution. The completed producer supplies fresh extracted-package and host integration evidence. Separately, actual persistent-preview reports now exist at `/var/tmp/rom-003-preview-live-acceptance/after-1791211434295.json` and `after-1791211528511.json`. Both report `status: pass` at `https://10.66.0.2`; the second was captured after a full container restart. Both record curl CA/hostname verification, the expected ROM Studio title and neutral favicon, 10 Resource kinds and 14 rows, all 13 baseline rows unchanged (`differences: []`), the field showcase, plugin Settings, right Work inspector, custom ticket editor, and zero prohibited mutation requests. The browser bypass was limited to this known internal-CA origin; curl separately verified the private CA and hostname. This is host-side VPN-origin evidence and does not establish that every remote client network path was tested. Parent reports a separate attachment persistence audit passed at 163,040 bytes (`persistent=1`, `isolated=0`) and that the preview units remain enabled and active after restart; those two facts were not independently recomputed in this source review.

## Acceptance map

| Requirement or owner scope | Evidence and result | Limit |
| --- | --- | --- |
| Resource presentation metadata, authorized discovery, Rust/TypeScript compatibility | Gate 1 full local check and gate 6 extracted-package check passed. Gate 8 ran the semantic catalog through the packaged host. | The independent verifier validates the recorded commands and archive identities; it does not rerun them. |
| Human Resource title with exact identity retained | Gate 7 browser suite includes presentation/title regressions; gate 8 runs human-auth and generic Resource browser flows. | These are synthetic demo Resources and identities. |
| Shared right inspector, filters, details, focus, and retained drafts | Gate 7: 221 passed. Its suite includes right-inspector and responsive editor cases. | One WebKit physical-touch case is explicitly skipped because the test uses Chromium CDP. Chromium touch and keyboard behavior are tested. |
| Work inspection and recovery semantics | Gate 7 Work presentation/recovery tests; gate 8 exercises real-host work flows, accepted recovery, and shutdown behavior. | The source correctly treats Work as a current snapshot, not a historical timeline. |
| Settings as Resources and plugin Settings composition | Gate 7's plugin Settings scenario verifies disclosed group selection and a revision-checked ordinary patch request. The built-in host Settings Resource is also present in the real-host build. | The plugin-specific group scenario uses an API fixture, not a dedicated plugin Settings kind against the real host. Core/host operations are separately exercised against real storage. |
| Generic semantic controls and backend support | Gate 7 exercises standard and custom controls; gate 8 passes 28 packaged-host cases against SQLite/redb in Chromium/WebKit. | The one-sample UI timing records are workload-specific, not general performance claims. |
| Custom renderer composition by an application | Gate 8 executes the demo application's explicitly registered renderer against production assets. A fresh extracted SDK consumer also installs, type-checks, builds, and passes 4/4 cases across SQLite/redb and both browser engines. | This verifies explicit application registration. The standard bundle does not auto-load application extensions, as intended. |
| DRY/module quality | Gate 1 `./scripts/check` passed on the exact source; gate 6 separately tested extracted packages. | No claim is made that every future module has been independently reviewed. |
| AI author workflows | Gate 5 `./scripts/check-skills` passed; the skills archive is independently verified. | Archive preflight does not execute every example from the extracted archive. The source gate runs the documented skill checks. |
| README screenshots and neutral icon | The source archive contains the updated README and captured screenshot paths. The Studio archive contains the neutral `/rom-studio/` icon asset and its verified digest. | Screenshots are synthetic local-host captures, not live deployment evidence. |

## Remaining notes

The release task file in the accepted source still leaves 2.5 and 5.6 unchecked. Packaged renderer composition has fresh producer and external-consumer evidence, and missing-renderer behavior is covered, so task 2.5 appears administratively stale. The later live reports close the preview acceptance evidence for 5.6, although its checkbox and copied install metadata were not rewritten in the immutable c66c470 artifact.

`README.md` still says that 0.0.2 is the last completed local source-and-Studio distribution and calls the 0.0.3 artifact unreleased. Those statements describe the source snapshot before this producer completed. They do not invalidate the artifact, but they are now stale or ambiguous for readers of this newly created 0.0.3 distribution. Correcting them would require a new source commit and another complete producer run.

The gate 7 physical-touch skip is explicitly documented in `studio/tests/components/sortable-fields.spec.ts`; it is the only browser-suite skip. It is not a blocker for the written OpenSpec keyboard/focus requirements, but WebKit physical touch remains unverified. The installed `application-source-facts.json`, identity bridge, and `installation-inputs.json` still state that live preview acceptance/activation is pending; those files predate the two passing live reports above. This is a stale-recordkeeping gap, not evidence that the currently served preview failed. A separate old isolated `0c` unit restart returned `Denied` while diagnosing a copied database; root reports that this is not the Caddy target and that originals/snapshot were preserved. I did not inspect or independently validate that separate diagnostic.
