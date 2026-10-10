# ROM 0.1.0 public guest independent review

Date: 2026-10-08. Status: reviewed candidate; release admission remains open.

This review inspected source and retained execution evidence. It did not rerun browsers, compile code, start servers, or change processes.
Only this report was written. Historical failures and source witnesses remain unchanged.

## Reviewed boundary

The connected consumer uses public `rom-studio/client`, `observe`, `ui`, `ui/components`, and `controls` entries.
It is installed from an independently extracted source archive. Its Vite configuration has no private source aliases.
The native fixture uses public ROM declarations, authorization, projection, query, and observation contracts.
No application-specific shortcut was added to the core for this public view.

Reviewed source includes `studio/tests/ui-composition/consumer/**`, `verify.mjs`, `http.mjs`, and `playwright.config.ts`.
The review also covers `examples/maintenance-portal/src/catalog.rs`, its credential resolver, and the public catalog tests.

## Independent evidence checks

Both completed browser records use archive SHA-256 `661581fa0662b7361d3821afaef476c7bb38d726c61241d62735271480c22e61`.
The reviewer recomputed all 21 recorded fixture hashes before the coordinator started the next test-only increment.
They match the files inspected for this candidate.
The reviewer recomputed the immutable native binary hash and all 209 source hashes in its witness. They match.

Native binary:
`/var/tmp/rom-010-maintenance-public-native-cvHBo1/rom-maintenance-http-fixture`.
SHA-256: `d7290ea6ec2c989934e9fdb6138c0c36eca575e880666c28b509a5f126f97e8a`.
Size: 116,322,544 bytes.
Witness: `/var/tmp/rom-010-maintenance-public-native-cvHBo1/identity.json`.
The witness is an authoring-checkout record. It explicitly denies release admission and production authentication.

| Execution record | Inspected result | Result SHA-256 |
| --- | --- | --- |
| `/var/tmp/rom-010-portal-public-sqlite-final-v3/result.json` | 18 Chromium and 18 WebKit passes | `bafe80a81f8a40b296d90a4816a877e1c835c75d8a8d078e9e7532a431a5a1fa` |
| `/var/tmp/rom-010-portal-public-redb-final/result.json` | 18 Chromium and 18 WebKit passes | `73198e90c6c8853daa862c22a2e01b3b8448f05e8552eecdf828b97419c647a1` |

Both browser reports record zero skips, unexpected results, and flaky results.
Each run also records successful locked installation, Svelte checks, build, and the occupied-port regression.
The occupied-port records confirm child termination before cleanup and successful database reopen on both adapters.
These are inspected author-run results, not independent reruns.

The reviewer read `/var/tmp/rom-010-public-catalog-typed.log`, `-14-consumer.log`, and `-14-clippy.log`.
They record two typed/live tests, all 14 consumer tests, and successful Clippy.
The new typed test file is separate from the earlier 209-file binary witness. Its execution is not inferred from that witness.

## Source findings

No blocking defect was found in this reviewed candidate.

**Identity and authority:** only absent `Authorization` maps to the explicit embedded public Actor.
A present unknown or invalid credential returns `Denied`. An `X-Subject` header cannot grant publisher authority.
The browser test separately verifies that an unsupported body `actor` member returns 400 without private data.
The public reader can discover only the guide type and three declared public fields.
Private fields, private predicates, private Resource reads, and writes remain denied.
Publisher authority remains restricted to the trusted synthetic Alice context.
These fixed credentials are test infrastructure, not production authentication.

**Browser ownership:** the public observation uses an explicit public scope key.
It does not derive a public grant from a null principal.
Direct public entry skips private IndexedDB initialization and private mutation recovery.
Private-to-public navigation disposes observations and mutation lanes and clears private rows, selection, history, recovery, and exports.
Fixture identity-switch buttons remain visible for testing. They are not production administrative controls.

**Projection and computation:** `guide` rejects extra fields and validates the expected kind and bounded values.
The client validates wire correspondence. The computation uses `BigInt`, not lossy floating-point conversion.
Selection is bounded to 20 rows. Input and output have byte limits.
The computation sums selected published values; it does not claim engineering or safety advice.

**Export ownership:** capture records the exact kind, ID, revision, date, format, and public context ticket.
Before export publication, the consumer re-reads each selected guide and verifies its revision and current context.
Selection or row changes increase the context epoch and clear pending or ready output.
The download path checks the captured context before creating and publishing its object URL.
Navigation disposes the observation, cancels pending reads, and prevents late result publication.
The result is temporary visit state. It is not persisted as a private mutation or reusable authorization cache.

## Executed cases and remaining evidence

The three guest browser cases execute these paths on both adapters and both engines:

- Direct public entry opens no private storage; two selected values sum to 100229; deselection removes ready output.
- Anonymous observation, query, read, exact JSON download, private-read rejection, forbidden predicate rejection, and no additional success event.
- A held export cannot publish after navigation to a private context; no unhandled browser error appears.

The typed/live tests prove that publisher edits produce revision-two projections without private values on both native adapters.
They do not execute browser result invalidation after that edit.
Add a maintained browser case that changes a Guide after arithmetic/export becomes ready.
Require the live update to clear both outputs before a new computation uses revision two.
Also hold an export, then change selection. Require no late ready state or download.
The source has fences for these paths; the present guest browser cases do not prove their execution.
The coordinator accepted these recommendations and owns the new test-only increment.
This report retains the earlier 21-file, 72-result candidate identity; it does not infer future test results.

This fixture does not exercise policy changes through related identity/settings Resources or principal-sensitive conditional caching.
Those R6 obligations remain separate. The fixed public policy does not establish general cache safety.
No full release, original Astral Plane retest, or human usability acceptance follows from these matrices.

## Preserved failures

`/var/tmp/rom-010-portal-public-guest-red/browser.log` retains four initial missing-public-UI failures.
The reviewer read its report: two guest cases timed out in each engine before the public entry control existed.
The report records zero expected passes and four unexpected outcomes.

`/var/tmp/rom-010-portal-public-sqlite-final/browser.log` retains the later oracle failure: expected 403, received 400.
Its request combined a private read with an unsupported body `actor` member.
The corrected test separates valid private-read denial from strict unsupported-member rejection.
The failure did not require a product change and must not be described as an authorization defect.

## Writing review

Identifiers, hashes, measured counts, and evidence boundaries are preserved.
Vocabulary was checked against known rulings and high-risk patterns only, not against the official ASD-STE100 Part 2 dictionary.
Full compliance requires verification against the official standard.

## Independent review of the subsequent test increment

The coordinator added the two recommended browser cases in `consumer/tests/portal.spec.ts`.
This review inspected those assertions and the final execution records. It did not rerun browsers or change product code.
The earlier 72-result candidate and its limits remain recorded above.

The reviewer recomputed all 21 fixture hashes from the final witness. Each matches the current inspected file.
Both adapter records contain the same fixture and installed-source maps.
Compared with the earlier candidate, only `consumer/tests/portal.spec.ts` changed among the fixture files.
Its final SHA-256 is `e89788604a874d333d8f4756eb065e900c99349b0c5b45f8a5fda63f37226e68`.
The source archive remains `661581fa0662b7361d3821afaef476c7bb38d726c61241d62735271480c22e61`.
The reviewer recomputed the native binary hash; it remains `d7290ea6ec2c989934e9fdb6138c0c36eca575e880666c28b509a5f126f97e8a`.

| Final record | Independently inspected browser result | Result SHA-256 |
| --- | --- | --- |
| `/var/tmp/rom-010-portal-public-invalidation-full-sqlite/result.json` | 20 Chromium and 20 WebKit passes | `4fa7b015a2ad18e6e0e936a9aad8164997277eda088f6cf97c3f6da8941848c9` |
| `/var/tmp/rom-010-portal-public-invalidation-full-redb/result.json` | 20 Chromium and 20 WebKit passes | `0e900965b0f7fe5f720886786f4204e5eb4d90f56be837af3c85e6328616c89c` |

The reviewer parsed both raw `browser.log` reports after their command-record prefix.
Each records 40 expected passes, zero skips, zero unexpected results and zero flaky results.
Both new cases appear with a passed result in both engines and both adapters: eight final-matrix executions.
The complete final matrices contain 80 executions. They are not 80 additional cases beyond the previous 72.
All five retained command records per adapter have exit code zero, including type checks, build and bind-failure cleanup.

The live-revision case computes 230 and exports revision one before an actual authorized publisher action.
It requires the browser to receive revision two with value 240.
The ready sum and export become `none`; the download button disappears before explicit recomputation.
Recomputation yields 240. The next export includes revision two and the new sum, without the private-note sentinel.
The authorized journal contains exactly two events.

The held-export case waits until the proxy holds a real read response.
The guest then deselects the row without changing principal. Releasing that response cannot restore export output or its download button.
The export control remains disabled and the test records no page error.
These assertions cover the two earlier gaps directly. No new blocking defect was found in this test-only increment.

Final witness: `/var/tmp/rom-010-portal-public-invalidation-full-witness.json`.
Recomputed witness SHA-256: `eac146303f9f504d68c723a072ddc3a2e451b0ee4bd69130c3ed006d10b6a989`.
This evidence characterizes existing product correctness; no product regression failure or correction is claimed.
Original Astral Plane adoption, human usability, policy-sensitive cache safety and full release acceptance remain separate obligations.
