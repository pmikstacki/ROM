# ROM 0.1.0 coordinator regression review

Date: 2026-10-07. This record does not establish release readiness.

## AI attempt knowledge

The coordinator independently ran the frozen AI routing, reservation, and flow regressions.
All 63 tests passed: 15 routing, 14 reservation, and 34 flow tests.
Pre-run and post-run hashes matched the complete AI inventory.
Inventory: /var/tmp/rom-ai-task4-late-observed-source-hashes.txt.
Execution: container:/var/tmp/rom-010-root-ai-task4-late-observed-review.log.
Hash checks: /var/tmp/rom-010-root-ai-task4-source-check.log and /var/tmp/rom-010-root-ai-task4-source-post-check.log.

Source review covered held-attempt enrichment, trusted adapter observations, account proof, current authority, and revision retries.
Enrichment reconstructs the permitted state from the prior record and compares the complete result.
It cannot change output, operation identity, cancellation, or the run state through this transition.
Missing knowledge can be added. Conflicting known facts are rejected.
This narrow result does not accept the concrete OpenRouter adapter or external consumers.

## Opaque signing key identifiers

The coordinator ran rom-auth with all features enabled.
All 35 integration tests and five doctests passed.
Execution: container:/var/tmp/rom-010-root-auth-key-id-all-features-review.log.
An earlier default-feature run skipped signed-profile tests. It remains evidence, but is not signing-profile verification.
Earlier execution: container:/var/tmp/rom-010-root-auth-key-id-review.log.

Source review confirmed a 256-byte UTF-8 limit, no empty value, and no Unicode control characters.
Matching does not normalize Unicode, change case, or truncate identifiers.
Signed tests cover the original provider shape, the byte boundary, invalid key acquisition, and exact matching.
Real Host callback and production TLS acceptance remain separate requirements.

## Studio bootstrap

The shared backend/frontend fixture preserves the complete unsigned retry epoch as a decimal string.
The Host passed three bootstrap tests and all-target Clippy.
Execution: container:/var/tmp/rom-010-host-bootstrap-corrected-green.log.
The HTML test checks JSON roundtrip and unchanged assets after size rejection.

Frontend bootstrap and IndexedDB checks passed nine tests after formatting.
Execution: /var/tmp/rom-010-studio-bootstrap-formatted-green.log.
Svelte checking reported no errors or warnings.
Execution: /var/tmp/rom-010-studio-bootstrap-current-check.log.
The new negative test first accepted a control character that Rust rejects.
Execution: /var/tmp/rom-010-studio-bootstrap-unicode-red.log.
The fix rejects Unicode control characters and lone surrogates while preserving valid Unicode scalars.

The broader controller suite ran during active form implementation: 50 tests passed and three failed.
Execution: /var/tmp/rom-010-studio-bootstrap-related-current-failed.log.
These results do not establish aggregate recovery acceptance.
Current bootstrap source hashes: /var/tmp/rom-010-studio-bootstrap-current-source-evidence.json.

## Open release requirements

The actual installed App matrix, original consumer acceptance, normal demo bootstrap, and concrete provider adapter remain incomplete.
Production identity, backup/restore, upgrade, load/resilience, complete verification, and clean-source publication still require their own evidence.
No candidate was deployed and no release tag was published by this review.

## Subsequent adapter and Host evidence

The adapter worker reported a passing initial 14-test candidate and all-target Clippy.
The coordinator inspected the retained log and checked every file in the recorded source inventory.
Log: /var/tmp/rom-openrouter-expanded-final-green-clippy.log.
Inventory: /var/tmp/rom-openrouter-initial14-source-hashes.txt.
Hash check: /var/tmp/rom-010-root-openrouter-initial14-source-check.log.
The log records 11 loopback tests and three wire tests. This is inspected worker execution, not an independent rerun.
Endpoint eligibility, catalog refresh, TLS, durable tool integration, and external consumers remain open.

The corrected identity fixture now starts the real Host on SQLite and redb.
Its source-authoring result records failure on both adapters. Production TLS and artifact admission are false.
Evidence: /var/tmp/rom-010-authentik-20261007/run/volume/evidence/host-authoring-ad7b11156fa0c89652f08af4f59e4a5f/result.json.
The worker reports callback HTTP 401 after the original provider login.
The earlier browser startup failures remain separate fixture evidence.
The JWKS correction requires new focused tests and another complete callback run before acceptance.
