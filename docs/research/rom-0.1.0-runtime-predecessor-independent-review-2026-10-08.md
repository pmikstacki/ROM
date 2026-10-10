# Independent review: accepted Runtime and custom-codec replay

Date: 2026-10-08. Scope: the authentic predecessor Runtime replay subset of R11.
This review inspected source, immutable inputs, execution witnesses, and existing logs. It ran no native builds or test executables.
It changed no implementation files. Release admission remains false.

## Result

No blocker was found for the declared subset. The evidence supports actual accepted Runtime receipts and their current replay on SQLite and redb.
The test also supports conflicting-request rejection, current policy revocation, and preservation of the compared business state.
This result supplements the preceding whole-state upgrade proof. It does not replace that proof or establish full application rollback.

## Accepted source and dependencies

The historical extraction identifies accepted commit `584c01b614127b3f62799f1a26a1cdf3f734c3dc`.
The reviewer compared all 2,658 extracted files with that commit's Git blob identities. All matched.
The files also matched the SHA-256 values in the preparation witness.

The compiled fixture manifest points to the extracted `rom`, `rom-sqlite`, `rom-redb`, and `rom-backup` crates.
Each direct ROM dependency requires version 0.0.3. The standalone workspace prevents current workspace feature unification.
The metadata identifies the extracted ROM manifests, including `rom-derive`.

The reviewer parsed the accepted and resolved locks independently. All 90 resolved dependency packages matched accepted name, version, source, and checksum.
The fixture itself is excluded from that comparison. The resolved lock differs from the complete accepted workspace lock by design.
The exact resolved lock matches `lock-conformance.json` and the execution witness.

The historical executable is 18,362,648 bytes. Its inspected SHA-256 matches the recorded identity:
`a1f86d2f955e19190eebc6908d6a2cc152e2d550ab946e8c8b2fd4b4e23b25e7`.
The build log identifies the corrected standalone application. Failed earlier fixture builds remain preserved.
The maintained source has later formatting changes. The exact compiled copy remains available and its three source hashes match.

## Runtime and codec behavior

The historical writer calls public Runtime APIs. It does not insert receipts directly or replace native format markers.
Its custom `TicketCode` field declares codec identity `upgrade-ticket-code`, version 1.
The decoder trims whitespace and converts ASCII letters to uppercase.

The writer executes create, patch, and the typed `increment` action. The action adds four to quantity and produces one stored effect.
The retained results have revisions 1, 2, and 3. Their values are FIRST/1, SECOND/1, and SECOND/5.
Each historical adapter retains three real receipts, three events, one row, and one effect.

The retained receipt identities distinguish standard create, standard patch, and custom increment.
Their fingerprints contain canonical field values or the action input. Each receipt has replay version 1.
The writer asserts native format 8 and archive version 6 through the original public backup reader.

The current test upgrades into a new destination. It repeats the exact three commands with their original keys, revisions, and principal.
Each returned ID, revision, and encoded value must equal the predecessor result.
A different action input with the original action key must return `Error::IdentityMismatch`.

The test then opens a Runtime whose current Resource policy denies the same original principal.
All three original commands must return `Error::Denied`. This is application-policy revocation, not an external identity-provider test.
The test compares rows, receipts, events, effects, descriptors, and references with the predecessor snapshot after these attempts.
It also compares original native source and archive bytes before and after upgrade and replay.

## Execution identity and raw results

Evidence root: `.superpowers/rom-010-runtime-predecessor-6l52Y2`.
The reviewer recalculated the following hashes from the files named in `witnessed-replay/execution-witness.json`:

- All 265 selected current source and configuration files.
- All three exact compiled historical application files.
- All six historical input files: native source, archive, and summary for each adapter.
- All three current logs, including compiler identity.
- The current lock and resolved historical lock.
- The detached current test executable.

Every value matched. The current test executable is 86,868,384 bytes, SHA-256:
`9f2cbf8fd69ef6287c3b6fa74ceacae9de8015e8858939b9b7b295282daf380f`.
The compiler record identifies rustc 1.99.0, commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, for `x86_64-unknown-linux-gnu`.

The current replay command uses `ROM_RUNTIME_PREDECESSOR_EVIDENCE` and `--ignored --nocapture`.
Its raw log reports one passing test and explicit success messages for SQLite and redb. It reports no failed or ignored case in that invocation.
The scoped Clippy log reaches successful completion. Both logs match their execution-witness hashes.

The following historical logs were also read:

| Evidence | Observed result |
| --- | --- |
| `corrected-v2/build.log` | Corrected historical writer build completed. |
| `corrected-v2/sqlite-writer.log` | Actual accepted SQLite Runtime population completed. |
| `corrected-v2/redb-writer.log` | Actual accepted redb Runtime population completed. |
| `witnessed-replay/current-replay.log` | Current replay and current policy denial passed on both adapters. |
| `witnessed-replay/current-clippy.log` | Scoped Clippy completed. |

## Limits and remaining release work

The new trial compares the declared state categories. It does not compare the complete durable-work snapshot.
Keep the preceding stronger whole-state proof for work generations, unknown delivery state, operator receipts, and recovery metadata.

This application uses the same codec identity and version in both writer and reader.
It does not establish compatibility across a codec rename, a codec-version change, or different canonicalization rules.
The fixture has one Resource, one custom field, and three command forms. Its result does not establish arbitrary application compatibility.

The test proves stored-effect preservation. It does not execute an external delivery provider.
The policy-denial case does not establish production OIDC behavior. Other identity evidence remains separate.
Neither fixture is an installed-package consumer or a whole application cutover and rollback test.

The maintained test is ignored unless explicit historical inputs are supplied. Default workspace success does not execute this gate.
Final-source verification and matching release artifacts remain necessary. Preserve historical inputs, failed builds, and detached executable identities.

The evidence budget was a plan, not a filesystem quota. The detached current executable exceeds the original 64 MiB evidence allowance.
The investigation report records that correction. This review does not describe the earlier allowance as enforced.

The prose follows `docs/writing.md` and the ASD-STE100 guide. This report is not a certificate of dictionary compliance.
