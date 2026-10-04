# ROM 0.0.2 Studio packaging results

The [frozen source manifest](evidence/rom-0.0.2/task7/source.json) identifies 24 owned script and README files. The recorded baseline is commit `e2f999d` with uncommitted artifact changes, reviewed on 2026-10-04. No shared manifest, lock, version, frontend, native, demo, or browser source changed in this task. The coordinator owns integration and commits.

## Implemented contract

New production invocations emit manifest version 2, verification profile `rom-studio-v2`, and distribution `source-and-studio-assets`. Historical manifest version 1 retains the exact original six-gate source-only verification contract. A preserved historical wire-shape fixture exercises version 1. Old artifacts were not modified.

The producer has eight fixed gates. Gates 1–6 retain their command identities. Gate 7 runs the frontend's actual browser verifier. The intervening package step uses a fresh copy of complete frontend source inputs, exact package/lock version and dependency correspondence, `npm ci --offline --no-audit --no-fund`, and `npm run build`. It hashes original and copied inputs before and after the build. It removes copied inputs and node_modules before gate 8. Gate 8 always runs `./demo/verify-studio --assets-dir ABSOLUTE_EXTRACTION`; the script has no production command substitution or skip option.

The Studio archive contains complete regular production assets. The verifier checks entry references under `/rom-studio/`, exact file inventory and hashes, package/lock/source identities against the extracted committed source, and the exact eighth gate arguments. Asset hashes must remain unchanged after host acceptance. The HTML check establishes a bounded static packaging contract; it does not claim browser execution.

The source archive rejects tracked frontend output/cache directories using the same root-only policy as the copy. Conventional `.env` and key files are rejected rather than copied, including ignored frontend inputs. Legitimate nested `src/dist` paths remain source. All subprocesses reuse the existing finite Linux process-group ownership helper, one-hour deadline and 32-MiB output bound. Source/tree/lock fencing and exclusive local publication retain their prior behavior.

`node scripts/check-studio-package.mjs ROM_ROOT OUTPUT_DIR` exposes the same fresh-copy package and actual-host acceptance as a finite standalone consumer. It rejects existing output. Its completed report is a Studio package check, not a full release. Trusted fixture runner injection remains a module-only testing interface.

## Executed verification

`nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/release-query-index && node --test scripts/packages/*.test.mjs scripts/release-artifacts/*.test.mjs'` passed **43 tests**, no skips or failures. Node was 22.16.0. The final log is [final Node log](evidence/rom-0.0.2/task7/rom-0.0.2-artifact-final-node-green.log). Owned `git diff --check` and `node --check` on all affected package/artifact modules and the entrypoint passed.

RED evidence records absent source/asset/build/consumer APIs and concrete admission failures. The gate-profile regression required eight commands but initially received six. Metadata tests reproduced v2-to-v1 relabeling with a Studio payload. Cache tests reproduced committed frontend output passing admission. Private-input and package-version tests reproduced copy admission and late version failure. The corresponding GREEN cases reject before the required subprocess boundaries.

The first actual fresh-copy offline npm run in rom-dev failed with `ENOTCACHED` for locked zimmerframe 1.1.5. The [cache prerequisite failure](evidence/rom-0.0.2/task7/container-build-failure/studio-npm-1.stderr.log) is retained as incomplete evidence. It did not produce an accepted package. No dependency install or network fetch was added to bypass this result.

The same real fresh-copy build on the host succeeded with its existing npm cache. `npm ci --offline` and production Vite build both exited 0. Original/copied frontend identities remained equal. The archive was extracted and all three production files matched. The asset digest was `01b7b731156d105aa5395647a2b2c86f2b58eb8da03494aaf0e13e2b061ffdfd`; exact records and logs are in the [build result](evidence/rom-0.0.2/task7/host-production-build/result.json) and [Vite output](evidence/rom-0.0.2/task7/host-production-build/studio-npm-2.stdout.log). The production extraction remains in the private task ledger for possible follow-up acceptance.

This actual build proves locked package production and extraction for the recorded source slice. It does **not** prove actual backend/provider/browser behavior. The 43 module tests use finite synthetic fixtures and never claim that mock execution establishes real release gates. The actual-host author separately reported both browsers and both stores passing its ordinary-production asset gate; this task did not execute that reported gate. Attachment UI work subsequently started, so final release assets must be freshly built from the frozen final frontend.

## Acceptance scenarios

- Fresh exclusive copied build; exact offline npm arguments; complete source and asset identities; no copied cache or development output.
- Missing source, incompatible lock, links, private environment/key input and release-version mismatch reject.
- npm failure or copied-lock mutation retain incomplete evidence and prevent acceptance.
- Actual-host command failure or mutation of the accepted extraction prevents publication.
- Frontend source/lock/inventory, asset inventory/digest, npm arguments and gate-profile metadata mutations reject after refreshed manifest checksums.
- Historical six-gate manifest admission passes; modern downgrade with a Studio payload rejects.
- Source dirty state, gate failure, source/lock changes, archive traversal/link/mode/tree/commit/profile/selected-inventory changes and output collision retain the existing behavior.

## Remaining acceptance

Independent source review, coordinator full verifier and the real clean integrated producer remain required. That producer must execute all eight actual gates and serve its own freshly extracted final assets. No full producer was run against this dirty shared worktree. Source and browser runtime caches are excluded from published archives. No registry publication or deployment occurred.
