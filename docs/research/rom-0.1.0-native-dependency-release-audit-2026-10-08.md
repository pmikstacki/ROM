# SQLite native release profile for ROM 0.1.0

Date: 2026-10-08. This is a read-only release-preparation review of the current lockfile, adapter, and native profile.
It is not a dependency update, a new build result, or acceptance of an untested release artifact.
Only this report changed. No source bundle was downloaded, no build ran, and no provider or browser launched.

## Decision

Use the existing hash-pinned SQLite **3.53.4 static native profile** for a released native executable, after validating the current release source with that profile.
Keep the pinned Rust bindings. Avoid a ROM-maintained SQLite fork or selective backports unless a specific deployment requires them.
This reuses an existing build route and takes the complete upstream patch release. It is the lowest-maintenance route identified here.

Keep ordinary Cargo builds explicitly identified as the **bundled 3.53.2 compatibility profile** until the dependency or build policy changes.
A Cargo source package cannot force the native environment override onto its consumers.
Do not describe default Cargo consumers as receiving 3.53.4, or describe historical patch-profile tests as current 0.1.0 acceptance.

This recommendation is an engineering judgment. The inspected fixes do not establish a reproduced corruption or exploitable defect in ROM's supported topology.
The clean, fresh RustSec audit reported by the coordinator remains separate evidence. It does not establish SQLite patch completeness.

## Profiles actually selected by the source

| Profile | Selection and observed source | Important difference |
| --- | --- | --- |
| Default Cargo | Workspace selects `rusqlite = "=0.40.2"` with `bundled`; lock selects `libsqlite3-sys 0.38.2`. Its installed header and amalgamation identify 3.53.2. | SQLite source is carried by the checksum-pinned Rust crate. |
| Explicit native check | `scripts/check-sqlite-native` builds official SQLite 3.53.4 statically and sets `LIBSQLITE3_SYS_USE_PKG_CONFIG=1`, `SQLITE3_STATIC=1`, and a private pkg-config path. | Native source and C compiler provenance sit outside Cargo.lock. |

The inspected build script checks the pkg-config override before the bundled branch: installed `libsqlite3-sys/build.rs:79–106`.
Installing another system SQLite without selecting this path does not replace the bundle.
Upstream documents native library/header selection and static linking in its [pinned build instructions](https://github.com/rusqlite/rusqlite/blob/v0.40.2/README.md).

The explicit script pins archive SHA256 `0e9483900e92cd5de8fd48d16bf9200145a61f7fd5be542a5ac81d8a9516eb9c`.
It separately verifies sqlite3.c SHA3-256 `67f423e9ebbbdc473cbc4772c872ee6b89f31fde4ed0279a5c25d5f65c043a16`.
The official 3.53.4 source ID is `2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc`.
The amalgamation digest and source ID match the [official release identifiers](https://sqlite.org/releaselog/3_53_4.html).
This review inspected the script; it did not fetch or verify that archive again.

The profiles also differ in compile options, not just engine version.
The explicit C command enables threading, column metadata, and URI support.
The bundled build additionally enables API armor, STAT4, DBSTAT, FTS3/FTS5, RTree, memory management, and other options: installed `build.rs:151–175`.
JSON support is built in by default in modern SQLite without needing the old enable macro.
[SQLite JSON build documentation](https://sqlite.org/json1.html#compiling_in_json_support).
These distinctions can affect available APIs and query planning. Record them instead of calling this an engine-only comparison.
SQLite explains optional features and their defaults in its [compile-option reference](https://sqlite.org/compile.html).

Retaining the existing small native option set minimizes profile maintenance.
If release policy requires bundled API armor or other options, make that a reviewed profile change and repeat validation.
Do not silently copy all bundled options or assume their removal is harmless to every downstream consumer.

## Changes after the bundled engine

The official release history currently lists 3.53.4 as its newest release.
Both patch pages summarize fixes through linked source timelines rather than supplying an exhaustive application-impact assessment.
The comparison below uses changes after the 3.53.2 release boundary, not features introduced earlier in 3.53.0.
See [release history](https://sqlite.org/changes.html), [3.53.3](https://sqlite.org/releaselog/3_53_3.html), and [3.53.4](https://sqlite.org/releaselog/3_53_4.html).

| Changed area | Primary-source observation | ROM applicability assessment |
| --- | --- | --- |
| Ordinary query and API correctness in 3.53.3 | Later fixes cover REAL-to-INT conversion in an UPSERT and missing connection mutex acquisition in several public C APIs. | ROM uses UPSERT templates and revision/counter integers. Its connection mutex reduces concurrency exposure, but this is not proof every numerical case is unaffected. |
| Corrupt storage handling in 3.53.3 | Later changes strengthen freelist/index bounds checks and handling of a corrupt page size in a read-only WAL/SHM. | Trusted ownership restricts who supplies files. Crash damage and storage corruption still need bounded recovery checks. |
| Recovery in 3.53.4 | A later fix rolls back a hot journal when a crash leaves a particular damaged super-journal record. | ROM normally uses one WAL database and no attached database set. The exact reported super-journal route is not established in ROM. |
| Planner and JSON handling in 3.53.4 | Changes address expression indexes with subtypes/unary plus, and bounds in malformed JSONB handling. | ROM's persisted JSON is encoded through Rust, and the inspected adapter does not execute JSONB operators. Unsupported SQL forms must not become an exemption for all parser or storage bugs. |
| Optional tools and extensions | Later fixes also affect FTS, RTree, session/RBU, CLI, and miscellaneous extensions. | ROM exposes no arbitrary SQL/extension console through this adapter. Several affected facilities exist in the bundle even when ROM does not invoke them. |

The first two rows and optional-extension examples follow the [3.53.3 branch timeline](https://sqlite.org/src/timeline?from=version-3.53.0&to=version-3.53.3&to2=branch-3.53&y=ci).
The latter recovery, planner, and JSON examples follow the [3.53.4 branch timeline](https://sqlite.org/src/timeline?from=version-3.53.0&to=version-3.53.4&to2=branch-3.53&y=ci).
Applicability statements are inferences from these upstream descriptions and the narrowly inspected ROM code.
No upstream reproducer or complete patch diff was executed here.

### WAL and the deployment boundary

The well-known WAL-reset race is **already fixed in both candidate profiles**.
SQLite reports the fix in 3.51.3 and later. Its described trigger needs multiple connections attempting a write/checkpoint concurrently.
It is not a reason to claim that moving from 3.53.2 to 3.53.4 newly fixes that race.
[SQLite WAL-reset explanation](https://sqlite.org/wal.html#walreset).

ROM's adapter requests WAL plus `synchronous=FULL`, uses one `Mutex<Connection>`, and starts Immediate write transactions.
Filesystem stores acquire `NativeOwnership`; the ownership module explicitly targets cooperating processes on trusted local storage.
The adapter rejects SQLite URI paths and validates persisted ROM schemas before accepting an existing store.
These observations come from `crates/rom-sqlite/src/store.rs:12, 42–64, 81–105` and `crates/rom-backup/src/native_ownership.rs`.
They do not prevent an unrelated program from bypassing cooperative ownership.

The engine opens and reads the file before ROM finishes semantic schema validation.
Consequently, ownership and ROM validation are not a proof that SQLite safely parses every malicious raw database or recovery sidecar.
Keep trusted local files, cooperative single ownership, and the absence of arbitrary SQL explicit in the supported deployment profile.
SQLite's guidance treats untrusted SQL and untrusted database files as separate threats. [SQLite security guidance](https://sqlite.org/security.html).

The inspected SQL uses adapter templates, bound row values, and validated schema/index machinery.
For example, `persistence.rs:354` binds the resource UPSERT values; `native_work/write.rs` binds work and metadata records.
No general SQL injection audit was performed. No claim is made about author-supplied native code outside this adapter.

## Exact validation required for the release profile

1. Freeze the release source, all relevant Cargo locks, native source digests, toolchain, C compiler, target, flags, and linker environment.
2. Run `scripts/check-sqlite-native` on that source. Preserve its logs and retained native source/build directory.
3. Record `sqlite_version()`, `sqlite_source_id()`, and `PRAGMA compile_options` from the actual final Rust-linked executable.
4. Run the affected `rom-sqlite` unit tests and current runtime reaction tests under the same explicit native environment.
5. Run current shared persistence tests, including atomic claim/result batches, stale claim reads, rollback/reopen, Unknown outcomes, and canonical archive comparison.
6. Run the full local verifier and packaged/downstream consumer checks with the selected native profile and its exact artifact identity.
7. Repeat any release workload or disk-failure claim that is intended to apply to the 3.53.4 executable using that executable.
8. Verify notices and distribution contents. Record platform-specific support and the default source-consumer profile separately.

The current native script selects `rom-storage-conformance` and `rom-consumer`; it does not explicitly select every adapter/runtime unit-test target.
The current engine test in `tests/persistence/tests/native_profile.rs:1–15` checks the linked version against `ROM_EXPECT_SQLITE_VERSION`.
It does not assert the source ID or compile-option inventory. Those remain additional artifact evidence requirements.
Version tests must reject an unintended bundled/system fallback. A pkg-config version string alone is insufficient.
Retain the existing R10 thresholds if the new profile supplies its release workload proof. Do not transfer timings across differently built engines.

The historical dependency report records 55 tests under a 3.53.4 profile at maintained commit `041ee12`.
That result is useful route validation. It predates the current claim-prefix, keyed claim-live, and runtime batching changes.
It is not a current 0.1.0 patch-profile test result. Current bundled-profile results and fresh RustSec results do not fill that gap.
See [historical audit](mvp-dependency-audit.md).
No new test results are claimed by this report.

## Provenance, licenses, and Rust floor

The cached rusqlite and libsqlite3-sys `.crate` SHA256 values match their current Cargo.lock checksums.
Selected cached entries were compared byte-for-byte with bounded tar stdout: rusqlite `Cargo.toml` and libsqlite3-sys `build.rs`.
No registry directory was changed. No missing checksum metadata was treated as provenance proof.
The installed sqlite3.c SHA3-256 matches SQLite's official [3.53.2 release digest](https://sqlite.org/releaselog/3_53_2.html).

| Inspected item | SHA256 |
| --- | --- |
| Current Cargo.lock | `7f62f61cfa33ff94771fa6098110683f195fee77a65f112e716224d89251a5f1` |
| Current Cargo.toml | `9ab382a9de63c970c07b3f5487b4fc98cae9c33a321e7cba8cb8179a94f3b3be` |
| rom-sqlite Cargo.toml | `9bd0237fcf11e32bb5128c594730e896e96f585f072f00fda1afac9c62e64547` |
| scripts/check-sqlite-native | `6e2517da2ec696eeb91970fd1beb83fd54c35d8b3c52664444a7e1f7400c6a3f` |
| Native profile test | `94b8a007de0d39371313a191615a4e075d57e4070bae600e5df3c1507642f961` |
| SQLite store source | `8678e57fa0d02196e9514fd37f2be49ad1c1a852e37f86a019efbb6db08e9acd` |
| SQLite persistence source | `8362175ceeeeee3065bcf91270ae6782a230b3f98d413e0c66f7421295562e07` |
| SQLite maintenance source | `1b2c8123d81b32f9571e9c07e87a8809486802dcf7d81c27082140d88a3294c6` |
| Historical audit | `a309325936b7ea87800a7d2d39c566b14ac69f3200ed2c0fdf22ed208383c10d` |
| rusqlite 0.40.2 crate archive / lock checksum | `23f2a97da3e3873c73cb2a2e71b35c40ff95e0b1eefa8d72d8499a6928c3b5b3` |
| libsqlite3-sys 0.38.2 crate archive / lock checksum | `f1d20bef17f513b9b3004532233187769cd072d790971f4e4da0e346eb6401e8` |
| Installed libsqlite3-sys build.rs | `269acf1864de5046ac5b77814926d41fe24108b0d027f635812a1db3c965f81f` |
| Installed sqlite3.h | `9e69a1353a4288450b0d5239ede11fc7f1f4c8e5eb07491fc8317eacb5b7de7e` |
| Installed sqlite3.c | `0a409f1633283fa31a9126b11fbfd64a1991c5d30defad07e5745d4667f5e23d` |

The installed amalgamation's SHA3-256 is `44fd61b9f93b4155105cb2d80c957ae6c64a8b5bd6ed51a4992f0dbd438e4e11`.
Its header records source ID `2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24`.
Installed registry sources were read under `/root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

Both Rust wrapper manifests declare MIT, and each installed package contains a root LICENSE file.
SQLite itself is public domain, separate from the wrappers' MIT licenses. [SQLite copyright](https://sqlite.org/copyright.html).
This is not a full release-notice assembly or legal certification. Other native dependencies are outside this focused audit.

ROM declares Rust 1.99. The inspected wrapper manifests do not declare their own `rust-version`.
The upstream rusqlite 0.40.2 release states an MSRV of 1.88.0. [Pinned rusqlite release](https://github.com/rusqlite/rusqlite/releases/tag/v0.40.2).
That claim does not lower ROM's support floor or replace a current Rust 1.99 build of the selected native profile.

## Limits

Official timelines, release/build documentation, and narrow installed-source reads support this recommendation.
They do not establish complete security review, every fix's applicability, or behavior under machine power loss.
Crates.io API reads were unavailable through the browsing tool; this report does not claim a fresh exhaustive latest-crate inventory.
Several individual Fossil check-in pages were also unavailable; their official timeline descriptions were used and identified above.
The research skill requested a background agent, but the session's agent limit prevented delegation. Research continued read-only in this agent.
No RustSec scan, build, conformance run, source-bundle download, cleanup, or runtime experiment was performed for this report.
