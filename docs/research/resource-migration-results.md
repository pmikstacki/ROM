# Versioned Resource migration results

Status: implemented and verified. Release checklist item 2.2 is complete; the full goal remains open.

## Result

Resource authors declare positive schema versions with `#[resource(version = N)]`.
The default remains 1. A typed `ResourceMigration::new::<Before, After>(convert)`
checks the exact source descriptor and advances one version. Ordered plans can
interleave steps for different kinds. Native `migrate_from` methods read formats
4 or 5 and publish a fresh format-5 destination.

The shared engine converts current rows, historical receipts/events, both journal
copies, work-source snapshots and protected deletion authorization. It memoizes
repeated values, validates codec round trips and rebuilds live references. Identities,
revisions and receipt fingerprints remain unchanged. Frozen action invocations,
notification payloads and effect intentions remain exact. A host callback must
validate unfinished consumer contracts. The engine does not infer callback-code compatibility.

See [the author and operator guide](../resource-migrations.md).

## Corrections from experiments and review

Old requests were normalized with the current codec before receipt lookup. A field
rename could therefore prevent valid retries. Receipts now retain their original
codec version. `Definition::replay_from::<Old>()` registers exact replay interpretation.
Current authorization precedes normalization. A fresh request cannot use an old codec.
Missing codecs fail explicitly; ROM does not try multiple interpretations until one matches.

A custom codec test showed that acceptance by `decode` alone was insufficient.
Decode could normalize bytes, causing typed policy values to differ from raw stored
values. Migration now requires exact decode–encode equality on both sides. Source
rejection happens before conversion. Two failing regressions confirmed the defect
before the correction.

A format review found that older maintenance tools could discard `replay_version`.
Native format 5 and archive version 3 now guard the metadata. Explicit native upgrades
support formats 3 and 4. Archive upgrades support versions 1 and 2. Format-4 readers
retain source-format identity: an empty catalog with live rows is corruption, not
permission to rebind an unversioned store. Both adapters reject that case.

Direct native commits now reject an explicit replay version that differs from the
registered schema. The check shares the native transaction and precedes writes.
Matching receipt replay remains first. Legacy `None` is supported and is bound to
its source catalog version during migration or format upgrade.

## Executed evidence

- Eight core maintenance tests cover all Resource-value locations, rollback on
  conversion error, journal capacity and reserved work capacity without eviction.
- Nine replay tests cover exact version routing, renamed create/patch inputs,
  ambiguous normalizers, missing codecs, current authorization and panic containment.
- Five shared migration tests cover typed rename/type/default conversion,
  interleaved versions, memoization, invalid plans, budgets and validator failure.
- Three codec tests cover source and target canonicality plus valid conversion.
- Five native schema tests run on SQLite and redb. They cover references, tombstones,
  retained effects/work, leased Unknown deliveries, format-4 compatibility,
  backup/restore, rejected destinations and process exit before publication.
- Two runtime integration tests run on both adapters. They resume reactions from
  Source and materialized Action states, retry old requests without duplicate work,
  deny history after authority changes and retain deletion disclosure rules.
- Three native receipt-version tests prove rejection before writes and preserve
  matching receipt replay ordering.
- Derive fixtures cover the default, explicit and maximum version, plus six invalid
  declaration classes with source diagnostics.

The ignored process helper is invoked by its parent test and exits with code 86.
These are process-interruption tests, not physical power-loss certification.
A constructed format-4 fixture verifies layout compatibility; it is not a test of
every historical release binary.

Evidence logs in `rom-dev` include:

- `/var/tmp/rom-resource-map-red.log`
- `/var/tmp/rom-resource-version-red.log`
- `/var/tmp/rom-migration-shared-red.log`
- `/var/tmp/rom-migration-shared-final.log`
- `/var/tmp/rom-migration-runtime-green.log`
- `/var/tmp/rom-native-schema-migration-final.log`
- `/var/tmp/rom-format5-full.log`
- `/var/tmp/rom-schema-migration-full-check.log`

The worktree is `release-relations`, based on `dbb93d6`. Runs use Rust/Cargo 1.99.0
and the unchanged lockfile. Independent reviewers inspected implementation and
reran targeted tests. The coordinator's final `./scripts/check` exited successfully.
It ran strict OpenSpec validation, formatting, Clippy, workspace tests, doctests,
Rustdoc, compile fixtures, consumer execution, dependency isolation and isolated
auth/identity checks. Real-MinIO cases remain opt-in and were not run in this pass.

## Limits and remaining release work

Converters and compatibility validators are trusted, pure, bounded native code.
Panic containment is not a time or memory sandbox. Conversion uses a logical snapshot
and memoization; its serialized byte budget is not a precise heap bound.

This slice supports offline Resource field representation changes. It does not rename
Resource identities, rewrite possibly delivered payloads or replace incompatible
consumer code automatically. The host retains compatible action/channel definitions
and old receipt codecs. Current policies decide disclosure of converted history.

Stage 2 still needs dependency-aware retention and the reference application's
complete upgrade/backup/restore journey. Optimization, operator workflows, host
ownership, real identity setup, extension conformance and release packaging remain
required. The full release goal is open.
