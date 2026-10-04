# Evidence storage failure

Date: 2026-10-04.

The bounded producer ran from commit `b4206a6c4fb01cc175bf628c836dfc09cfca2037`.
It stopped during the first release gate after its 2 GiB build image filled.
The payload then failed to save its result because the same filesystem had no space.
The retained image and external controller journal do not prove a successful release gate.

The release tool now reports command status and bounded output tails to stderr when a log write fails.
It marks that command's evidence as unrecorded.
If `failure.json` cannot be saved, the tool reports both errors and preserves the original exception.
A failed evidence write still prevents publication.
The eight release gates are unchanged.

Three regression tests simulate `ENOSPC` at log and summary writes.
The affected artifact suite passed all 32 tests.
Its injected gate runners test release machinery, not the actual application acceptance gates.
Independent source review found no blocking issue.

Terminal output is a fallback, not a guarantee of durable storage.
An external supervisor must capture it if the build filesystem can fill.
The failed producer's image remains preserved.
No new producer run or budget increase followed this change.

Machine evidence is retained under `/root/ipi/research/disk-coordination-2026-10-04/`.
The test output is `ROM-evidence-failure-tests.tap`.
The actual producer result is `/var/tmp/rom-bounded-producer-OxfTwV/guardian-result.json`.
