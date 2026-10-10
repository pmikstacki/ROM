# Public Studio controls corrective review

Date: 2026-10-07. Reviewer: ROM release coordinator, independent of the corrective implementation author.

## Reviewed changes

The review covered `studio/tests/public-controls/verify.mjs`, admission helpers, negative tests, the minimal consumer, and stylesheet dependencies.
The consumer uses explicit dependencies and a frozen lock. It does not copy the producer development graph.
The verifier uses `npm ci` and checks realized package versions, installed source hashes, and lock stability.
An occupied evidence output is rejected before old success records or archives can change.
Archive admission checks paths, links, duplicate members, compressed size, expanded size, and member count before extraction.

Release admission requires supplied source and actual successful Chromium and WebKit executions.
The candidate archive has a fixed `rom-studio-source/` prefix and includes licenses and notices.
The root reran seven fast admission tests; all passed.
The corrective author retained red cases and six actual browser passes in `/var/tmp/rom-010-r2-corrections-evidence-20261007/`.
The locked candidate result is `/var/tmp/rom-public-controls-locked-admission-20261007/result.json`.
The reviewer inspected the browser fixture and admission path. The reviewer did not repeat that browser run.

## Findings and remaining limits

No must-fix defect was found in the reviewed corrective patch.
The supplied-source mode is not proof of a published release identity. The release producer must supply previously verified extracted source.
The release must bind that source, its version, frozen consumer lock, and browser evidence to the same candidate.
The fixture supports controls and styles. It does not establish alias-free imports from the application root entry point.
Public client and mutation recovery acceptance remain separate work.

The published `rom-studio-v2` profile keeps its existing meaning. A new profile must include the required public-consumer gates.
The full verifier must run again after implementation sources freeze. Earlier verifier evidence predates these corrections.
This review does not establish production readiness or close the 0.1.0 release goal.
