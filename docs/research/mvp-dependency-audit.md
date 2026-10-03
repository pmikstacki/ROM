# Initial MVP dependency audit

Date: 2026-10-02. Scope: the pinned integrated Resource probe dependency set proposed as the starting point for the maintained MVP. This is an adoption review, not authorization to update dependencies and not a claim that the maintained MVP is complete. Recheck the eventual maintained workspace lockfile and feature graph. Its additions and removals are outside this baseline.

## Result

**The executed advisory audit passed:** cargo-audit 0.22.2 reported zero vulnerabilities and zero warnings. Warnings were treated as errors. No advisories were ignored, no target filters were set, and yanked-package checking was enabled. The core also passed a fresh isolated build without database or transport dependencies. The native dependency licenses have no missing manifest declaration in this inventory, but release notice assembly and package metadata still need verification. A clean RustSec result does not establish that bundled SQLite has every relevant upstream fix.

The release/adoption decisions still requiring evidence are:

1. Audit the maintained workspace's actual lockfile and features after integration, including redb, HTTP, authentication, storage or notification dependencies if added. This report covers none of those additions.
2. Review SQLite fixes after bundled 3.53.2 for the supported deployment profile. Then document the selected version or tested backport. Do not upgrade merely because a newer release exists; do not infer native-engine patch coverage from RustSec alone.
3. Verify license declarations and included license files in each distributable ROM package, and assemble the applicable dependency notices. The probe packages declare neither `license` nor `license-file` and are explicitly unpublished; the repository itself has an MIT [LICENSE](../../LICENSE).
4. Retain and test an explicit supported Rust version. The baseline declares and builds on 1.99; no lower project MSRV has been demonstrated. Any lower selection needs an actual compiler run and dependency-policy review.
5. Validate the packaged consumer and the maintained feature matrix independently of workspace feature unification. The probe's unconditional derive dependency does not establish a macro-free core profile.

These implement the dependency, MSRV, isolation and packaged-consumer gates in [docs/quality.md](../quality.md). They are not newly invented feature requirements.

## Executed source and provenance

| Item | Recorded value |
| --- | --- |
| Probe commit | `86780041a9824c434054fae25509dff0e56a7212` |
| Host source | `/root/ROM/.worktrees/integrated-core-probe/prototypes/integrated-core` |
| Native container source | `/workspace/ROM/.worktrees/integrated-core-probe/prototypes/integrated-core` |
| Source status | Probe worktree clean before and after inspection; no manifests or lockfiles modified |
| Lockfile SHA-256 | `5eb5a73d7a2ba595d62c2150988abd68e03c62d86da287a1d5b08e9d1443fdf3` |
| Compiler | `rustc 1.99.0 (b940084d7 2026-09-28)` |
| Cargo | `cargo 1.99.0 (5f94df478 2026-08-27)` |
| Native target | `x86_64-unknown-linux-gnu` in `rom-dev` |
| Lockfile inventory | 52 packages: four local packages and 48 registry package/version entries |
| Native normal/build graph | 34 registry entries; 14 other-target entries remain in the lockfile |
| Audit tool | `cargo-audit 0.22.2`, locally installed under `/tmp/rom-dependency-audit/tools` |
| RustSec database | 1,280 advisories; commit `117edb3bed98e9be112f277b7615eea3252e7c43` |
| Database timestamp | `2026-10-02T10:58:33+02:00` |
| Audit JSON SHA-256 | `367025ae54255d94384670dfd5985040ecc218c69138bb0eaa09cb00ac8c0228` |

The four local manifests inherit edition 2024, `rust-version = "1.99"` and `publish = false`. All registry entries use the crates.io registry source and have lockfile checksums. No Git dependency occurs in this lockfile. These are source/metadata observations, not a source-code security audit or proof of publisher authenticity.

## Commands and results

Commands below ran inside `nixos-container run rom-dev -- sh -lc '...'`; scratch logs are in that container's `/tmp/rom-dependency-audit`. The tool installation used `CARGO_BUILD_JOBS=2` and an isolated target directory. It took 2m20s, installed cargo-audit 0.22.2, and changed no product dependency. No host toolchain or global Cargo binary was installed.

```sh
cd /workspace/ROM/.worktrees/integrated-core-probe/prototypes/integrated-core
sha256sum Cargo.lock
cargo metadata --locked --format-version 1 > /tmp/rom-dependency-audit/metadata.json
cargo tree --locked --offline -e features > /tmp/rom-dependency-audit/features.txt
cargo tree --locked --offline -p rom-probe -e normal > /tmp/rom-dependency-audit/core.txt
cargo tree --locked --offline --target x86_64-unknown-linux-gnu \
  --edges normal,build --prefix none --format '{p} features=[{f}]' \
  > /tmp/rom-dependency-audit/native-tree.txt

CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/rom-dependency-audit/tool-target \
  cargo install cargo-audit --locked --root /tmp/rom-dependency-audit/tools

cd /tmp/rom-dependency-audit
/tmp/rom-dependency-audit/tools/bin/cargo-audit audit \
  --file /workspace/ROM/.worktrees/integrated-core-probe/prototypes/integrated-core/Cargo.lock \
  --db /tmp/rom-dependency-audit/advisory-db --deny warnings --json \
  > /tmp/rom-dependency-audit/audit.json 2> /tmp/rom-dependency-audit/audit.stderr

cd /workspace/ROM/.worktrees/integrated-core-probe/prototypes/integrated-core
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/rom-dependency-audit/probe-target \
  cargo check --locked --offline -p rom-probe --no-default-features
```

All commands above succeeded. The audit exited 0, reported `vulnerabilities.found=false`, `count=0`, `list=[]`, and `warnings={}`. Its settings recorded empty `ignore`, `target_arch` and `target_os`, with `unmaintained`, `unsound` and `notice` informational categories enabled. No home audit configuration existed. The fresh core check exited 0 in 3.83s. For a reproducible future tool install, add `--version 0.22.2`. The original install selected that version. Advisory data must still be refreshed on recheck.

An initial `cargo metadata --locked --offline --format-version 1` failed because an other-target crate, `bumpalo 3.20.3`, was not cached. The successful online metadata command downloaded the missing target-specific sources without changing the lock. This is a cache limitation, not a failed native build.

The audit's raw JSON result was:

```json
{"database":{"advisory-count":1280,"last-commit":"117edb3bed98e9be112f277b7615eea3252e7c43","last-updated":"2026-10-02T10:58:33+02:00"},"lockfile":{"dependency-count":52},"settings":{"target_arch":[],"target_os":[],"severity":null,"ignore":[],"informational_warnings":["unmaintained","unsound","notice"]},"vulnerabilities":{"found":false,"count":0,"list":[]},"warnings":{}}
```

This is an executed lockfile audit, not a conclusion drawn from search results. RustSec describes cargo-audit as checking lockfiles against reported Rust advisories; it also distinguishes license-policy tooling such as cargo-deny. No cargo-deny scan or organizational license allowlist was applied here. [RustSec](https://rustsec.org/), [cargo-audit documentation](https://github.com/rustsec/rustsec/blob/main/cargo-audit/README.md).

## Feature and boundary review

The following enabled sets were observed in the native dependency tree, including transitive feature unification. Unless stated otherwise, dependencies use their defaults. The probe does not globally disable defaults.

| Direct dependency | Native enabled features | Adoption observation |
| --- | --- | --- |
| Tokio 1.53.1 | `default, macros, rt, rt-multi-thread, sync, time, tokio-macros` | Tokio execution and synchronization are intentional. `net`, filesystem and TLS integrations are absent. `macros` is requested in shared workspace configuration and therefore also reaches the core; the maintained package should justify its own feature requirements independently of examples. |
| Rayon 1.12.0 | none | Shared CPU execution is intentional; transitively brings rayon-core and Crossbeam synchronization. |
| Serde 1.0.229 | `default, derive, serde_derive, std` | Used for infrastructure persistence values. Its derive is a build-time procedural macro dependency. |
| serde_json 1.0.151 | `default, std` | No `preserve_order`, `arbitrary_precision` or `raw_value` feature is selected. Public exposure of `Value` is a compatibility decision, not a transport-isolation failure. |
| Syn 3.0.6 | `clone-impls, default, derive, full, parsing, printing, proc-macro` | This is the native union; unfiltered metadata additionally lists other-target feature requests. Do not infer the native feature set solely from unfiltered metadata. |
| Quote 1.0.47; proc-macro2 1.0.107 | `default, proc-macro` | Macro implementation dependencies, outside the domain runtime's database/transport boundary. |
| rusqlite 0.40.2 | `bundled, cache, default, ffi-sqlite-wasm-rs, hashlink, modern_sqlite` | Adapter only. `cache` and the WASM backend come from rusqlite defaults. The WASM dependency is target-conditioned and is not compiled for this native graph; its presence in Cargo.lock is not evidence of ROM WASM support. |

The `rom-probe` normal dependency tree contains neither rusqlite/libsqlite3-sys nor redb, HTTP, TLS, authentication-provider or cloud SDK dependencies. The isolated core check confirms that boundary compiles. However, `rom-probe-derive` is unconditional, and `--no-default-features` does not remove it because ROM defines no optional derive feature here. That successful check must not be described as a macro-free build.

Cargo features are additive and may be unified across dependencies. Tests of a workspace alone can hide consumer-specific omissions. Preserve separate core, adapter and downstream checks when the maintained manifests diverge. [Cargo feature documentation](https://doc.rust-lang.org/cargo/reference/features.html).

## Bundled SQLite: separate native-engine evidence

Downloaded `libsqlite3-sys 0.38.2/sqlite3/sqlite3.h` identifies SQLite **3.53.2**. A separate scratch consumer using exactly `rusqlite = "=0.40.2"` with `bundled` executed `SELECT sqlite_version(), sqlite_source_id()` and confirmed:

```text
SQLite 3.53.2
source-id 2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24
```

The scratch package is `/tmp/rom-dependency-audit/sqlite-version`. It started with a copy of the audited lockfile. All 32 retained registry package/version/checksum entries matched the audited lock, with zero differing entries. Executed command: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/rom-dependency-audit/probe-target cargo run --offline` from that package. Exit 0; runtime version and `PRAGMA compile_options` output are in `sqlite-version.txt`. This validates the selected dependency build, not the final maintained application's binary.

Observed C options include `THREADSAFE=1`, `ENABLE_API_ARMOR`, `ENABLE_FTS3`, `ENABLE_FTS5`, `ENABLE_RTREE`, `ENABLE_LOAD_EXTENSION` and `MAX_LENGTH=1000000000`. SQLite was compiled with GCC 13.3.0. This C feature set is broader than ROM's exposed query API; the absence of rusqlite's Rust `load_extension` feature does not mean the C library was compiled without extension support. Engine capabilities and application permissions remain separate boundaries.

SQLite's official CVE table lists CVE-2026-11822/CVE-2026-11824 as fixed in **3.53.2**. That specific fix is included in the measured version. Official patch notes also describe later fixes in **3.53.3** and **3.53.4**. Their applicability to ROM's fixed SQL, trusted database-file assumptions and supported ownership topology has not been reproduced or exhaustively assessed in this audit. Record that assessment before claiming native-engine patch readiness; a zero RustSec count is insufficient evidence for it. [SQLite CVE status](https://sqlite.org/cves.html), [3.53.3 release notes](https://sqlite.org/releaselog/3_53_3.html), [3.53.4 release notes](https://sqlite.org/releaselog/3_53_4.html).

### Concrete route to SQLite 3.53.4

As checked on 2026-10-02, **there is no newer published rusqlite/libsqlite3-sys release to obtain this patch through a version bump**. Both `cargo search --limit 1` and direct crates.io API reads reported maximum/default versions **rusqlite 0.40.2** and **libsqlite3-sys 0.38.2**, neither yanked. The published README confirms that their bundled source remains 3.53.2. Registry responses were saved as `rusqlite-registry.json` and `libsqlite3-sys-registry.json` in the scratch audit directory. [rusqlite registry metadata](https://crates.io/api/v1/crates/rusqlite), [libsqlite3-sys registry metadata](https://crates.io/api/v1/crates/libsqlite3-sys), [published upstream build instructions](https://docs.rs/crate/rusqlite/latest/source/README.md).

If the maintained MVP selects **3.53.4 now**, the actionable route is to supply an explicitly pinned native SQLite 3.53.4 build and link the existing Rust binding to it. Upstream documents disabling `bundled` and selecting the library/header directories through `SQLITE3_LIB_DIR` and `SQLITE3_INCLUDE_DIR`; `SQLITE3_STATIC=1` requests static linking. An alternative already supported by the inspected 0.38.2 build script is `LIBSQLITE3_SYS_USE_PKG_CONFIG=1`, which selects the linked-library path before its `bundled` branch. That permits a host build profile to keep the existing Cargo graph while selecting an explicit SQLite package through its library/pkg-config paths. Merely installing SQLite 3.53.4 while continuing the ordinary bundled build will not replace the embedded 3.53.2. [Published upstream build instructions](https://docs.rs/crate/rusqlite/latest/source/README.md), [libsqlite3-sys build script](https://docs.rs/crate/libsqlite3-sys/latest/source/build.rs).

For the selected linking profile, record the native source/package hash outside Cargo.lock. Examine the final application's `sqlite_version()` and `sqlite_source_id()`. Rerun ROM's database conformance tests with that exact binary. The official 3.53.4 release records source ID `2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc` and sqlite3.c SHA3-256 `67f423e9ebbbdc473cbc4772c872ee6b89f31fde4ed0279a5c25d5f65c043a16`. A maintained, pinned source patch to libsqlite3-sys is another possible route. It creates ROM-owned upstream patch maintenance. This audit did not prepare one. No 3.53.4 linking profile or native build was executed here. [SQLite 3.53.4 release identifiers](https://sqlite.org/releaselog/3_53_4.html).

## License review

The inventory below comes from the downloaded, checksum-pinned crates' `Cargo.toml` manifests through `cargo metadata`. A scripted inspection opened 88 root license/notice files across those archives, and the nonstandard license texts were inspected directly. This is a manifest inventory with license-file checks and focused text review, not an exhaustive file-by-file provenance scan.

- 34 entries declare `MIT OR Apache-2.0`; one declares the equivalent reversed ordering, and three older entries retain the literal `MIT/Apache-2.0` spelling. Preserve original metadata in the inventory and normalize deliberately in any future policy tooling.
- Seven entries declare MIT only. `memchr` offers `Unlicense OR MIT`, so MIT is an available branch.
- `foldhash 0.2.0` declares **Zlib**, with an included `LICENSE`. Its text requires preserving the source notice, accurate origin attribution and marking altered source. [Zlib license text](https://spdx.org/licenses/Zlib.html).
- `unicode-ident 1.0.26` declares **`(MIT OR Apache-2.0) AND Unicode-3.0`**, with `LICENSE-MIT`, `LICENSE-APACHE` and `LICENSE-UNICODE`. Selecting MIT does not remove the Unicode requirement. Retain the Unicode copyright/permission notice with relevant distributions or documentation. [Unicode-3.0 text](https://spdx.org/licenses/Unicode-3.0.html).
- SQLite's bundled C implementation is dedicated to the **public domain**, separately from the MIT licenses on rusqlite/libsqlite3-sys. Do not describe all bundled SQLite material merely as MIT. [SQLite copyright statement](https://sqlite.org/copyright.html).
- Other-target `rsqlite-vfs 0.1.1` declares MIT but its downloaded archive contains no root license/notice file and no manifest repository URL. Its declaration is recorded; complete notice provenance remains unresolved if that target is adopted or the full source dependency bundle is redistributed. This is not a native Linux runtime dependency.

Every inventory entry has a license declaration. None declares GPL, LGPL, or AGPL. That is a metadata observation, not a general legal compatibility certification. Release artifacts still need their actual notices and package contents checked.

## Rust version review

The project declares **1.99**, and the executed isolated core check used **1.99.0**. Thus, this audit supplies a build check at the declared version for that core profile. The probe's earlier workspace verification is reported in its integration report; it was not rerun as part of this dependency audit. No lower compiler, alternate architecture or WASM target was tested, and `rustup` is not installed in `rom-dev`.

The highest explicitly declared registry dependency MSRV is **1.85** (`hashbrown 0.17.1` / `hashlink 0.12.2`), but six entries declare none: `fallible-iterator`, `fallible-streaming-iterator`, `libsqlite3-sys`, `rusqlite`, `smallvec` and `vcpkg`. This maximum therefore cannot establish a minimum compiler for the whole project. Rusqlite's generic downloaded README says its policy follows latest stable at release time, while its more specific **0.40.2 release notes state an MSRV of 1.88.0**. Record that upstream release claim separately from the absent manifest field and from ROM's actually tested 1.99; do not infer 1.85 compatibility from the inventory. [rusqlite 0.40.2 release notes](https://github.com/rusqlite/rusqlite/releases/tag/v0.40.2).

Package `rust-version` is a support declaration and a Cargo compatibility check, not a substitute for compiling the actual supported profile. Adopt a maintained MSRV policy deliberately, and rerun it whenever the lockfile or selected features change. [Cargo Rust-version reference](https://doc.rust-lang.org/stable/cargo/reference/rust-version.html).

## Complete registry inventory

“Direct” means explicitly selected by the probe workspace; “native” means present in its x86_64 Linux normal/build tree, including host procedural macros. “Other target” means retained in the lockfile but absent from that native tree. License strings and MSRVs are package declarations, not independently proven minimums.

| Package | Version | Relationship and scope | Declared license | Declared Rust version |
| --- | --- | --- | --- | --- |
| `bitflags` | 2.13.2 | transitive; native | MIT OR Apache-2.0 | 1.56.0 |
| `bumpalo` | 3.20.3 | transitive; other target | MIT OR Apache-2.0 | 1.71.1 |
| `cc` | 1.5.1 | transitive; native | MIT OR Apache-2.0 | 1.65.0 |
| `cfg-if` | 1.0.5 | transitive; other target | MIT OR Apache-2.0 | 1.32 |
| `crossbeam-deque` | 0.8.8 | transitive; native | MIT OR Apache-2.0 | 1.61 |
| `crossbeam-epoch` | 0.9.21 | transitive; native | MIT OR Apache-2.0 | 1.61 |
| `crossbeam-utils` | 0.8.23 | transitive; native | MIT OR Apache-2.0 | 1.60 |
| `either` | 1.18.0 | transitive; native | MIT OR Apache-2.0 | 1.63.0 |
| `fallible-iterator` | 0.3.0 | transitive; native | MIT/Apache-2.0 | not declared |
| `fallible-streaming-iterator` | 0.1.9 | transitive; native | MIT/Apache-2.0 | not declared |
| `find-msvc-tools` | 0.1.14 | transitive; native | MIT OR Apache-2.0 | 1.65.0 |
| `foldhash` | 0.2.0 | transitive; native | Zlib | 1.60 |
| `hashbrown` | 0.16.1 | transitive; other target | MIT OR Apache-2.0 | 1.65.0 |
| `hashbrown` | 0.17.1 | transitive; native | MIT OR Apache-2.0 | 1.85.0 |
| `hashlink` | 0.12.2 | transitive; native | MIT OR Apache-2.0 | 1.85 |
| `itoa` | 1.0.18 | transitive; native | MIT OR Apache-2.0 | 1.68 |
| `js-sys` | 0.3.106 | transitive; other target | MIT OR Apache-2.0 | 1.81 |
| `libsqlite3-sys` | 0.38.2 | transitive; native | MIT | not declared |
| `memchr` | 2.8.3 | transitive; native | Unlicense OR MIT | 1.61 |
| `once_cell` | 1.21.4 | transitive; other target | MIT OR Apache-2.0 | 1.65 |
| `pin-project-lite` | 0.2.17 | transitive; native | Apache-2.0 OR MIT | 1.37 |
| `pkg-config` | 0.3.34 | transitive; native | MIT OR Apache-2.0 | 1.63 |
| `proc-macro2` | 1.0.107 | direct; native | MIT OR Apache-2.0 | 1.71 |
| `quote` | 1.0.47 | direct; native | MIT OR Apache-2.0 | 1.71 |
| `rayon` | 1.12.0 | direct; native | MIT OR Apache-2.0 | 1.80 |
| `rayon-core` | 1.13.0 | transitive; native | MIT OR Apache-2.0 | 1.80 |
| `rsqlite-vfs` | 0.1.1 | transitive; other target | MIT | 1.81.0 |
| `rusqlite` | 0.40.2 | direct; native | MIT | not declared |
| `rustversion` | 1.0.23 | transitive; other target | MIT OR Apache-2.0 | 1.31 |
| `serde` | 1.0.229 | direct; native | MIT OR Apache-2.0 | 1.56 |
| `serde_core` | 1.0.229 | transitive; native | MIT OR Apache-2.0 | 1.56 |
| `serde_derive` | 1.0.229 | transitive; native | MIT OR Apache-2.0 | 1.71 |
| `serde_json` | 1.0.151 | direct; native | MIT OR Apache-2.0 | 1.71 |
| `shlex` | 2.0.1 | transitive; native | MIT OR Apache-2.0 | 1.46.0 |
| `smallvec` | 1.16.2 | transitive; native | MIT OR Apache-2.0 | not declared |
| `sqlite-wasm-rs` | 0.5.5 | transitive; other target | MIT | 1.81.0 |
| `syn` | 3.0.6 | direct; native | MIT OR Apache-2.0 | 1.71 |
| `thiserror` | 2.0.21 | transitive; other target | MIT OR Apache-2.0 | 1.77 |
| `thiserror-impl` | 2.0.21 | transitive; other target | MIT OR Apache-2.0 | 1.77 |
| `tokio` | 1.53.1 | direct; native | MIT | 1.71 |
| `tokio-macros` | 2.7.2 | transitive; native | MIT | 1.71 |
| `unicode-ident` | 1.0.26 | transitive; native | (MIT OR Apache-2.0) AND Unicode-3.0 | 1.71 |
| `vcpkg` | 0.2.15 | transitive; native | MIT/Apache-2.0 | not declared |
| `wasm-bindgen` | 0.2.129 | transitive; other target | MIT OR Apache-2.0 | 1.81 |
| `wasm-bindgen-macro` | 0.2.129 | transitive; other target | MIT OR Apache-2.0 | 1.81 |
| `wasm-bindgen-macro-support` | 0.2.129 | transitive; other target | MIT OR Apache-2.0 | 1.81 |
| `wasm-bindgen-shared` | 0.2.129 | transitive; other target | MIT OR Apache-2.0 | 1.81 |
| `zmij` | 1.0.23 | transitive; native | MIT | 1.71 |

## Limits and next recheck

This audit does not establish freedom from undisclosed vulnerabilities, malicious upstream code, unsafe implementation defects, compiler bugs or application-level authorization/lifecycle flaws. It does not replace database conformance tests. Advisory coverage is time-dependent; the recorded database commit makes this run reproducible but should not be frozen for release audits.

The final maintained workspace must be compared against this exact package/version/checksum inventory and native feature graph. Run the advisory audit again on its own Cargo.lock. Regenerate license/MSRV metadata. Repeat core-only and packaged-consumer checks. Record any changed bundled native-library version. No dependency changes, suppressions, commits or pushes were performed by this audit.

After writing this report, `./scripts/check` passed all four strict OpenSpec validations; at that execution point the main checkout had no Cargo workspace, so the script correctly skipped main-workspace Rust checks. This does not extend the core-only build evidence above to the maintained implementation being developed separately.

## Maintained auth delta and native SQLite patch profile

At maintained `041ee12` the coordinator reran cargo-audit 0.22.2 with
`--deny warnings --no-fetch` against the same RustSec commit above: 211 locked
dependencies scanned, no findings. JWT now selects AWS-LC and the tested Rustls
version is 0.23.45. This updates the earlier probe audit without treating an
advisory scan as a review of all native source.

The default rusqlite 0.40.2 bundle is still SQLite 3.53.2. A separate, reproducible
native profile now tests SQLite 3.53.4 using `scripts/check-sqlite-native`. It
fetches the official amalgamation archive, verifies both archive SHA-256 and the
release page's independent `sqlite3.c` SHA3-256, builds a static library in a
fresh temporary directory, and uses libsqlite3-sys's supported pkg-config override.
It does not replace the system library or vendor a modified driver. Sources:
[SQLite 3.53.4 release](https://sqlite.org/releaselog/3_53_4.html),
[libsqlite3-sys build source](https://github.com/rusqlite/rusqlite/blob/v0.40.2/libsqlite3-sys/build.rs).

The coordinator executed that profile: the Rust-linked engine identified itself
as **3.53.4**, and all 55 consumer/persistence tests passed, including the 14
actual subprocess exits within the persistence parent test. The baseline engine
check also passed with the ordinary bundled **3.53.2** build. No performance or
machine-power-loss comparison was performed. Thus, patch-release conformance is measured. Default Cargo consumers still receive the version
bundled by the pinned upstream crate. A packaged executable release using the
native profile must carry its tested engine and notices; a Cargo source package
cannot force that environment override on downstream hosts.

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && CARGO_NET_OFFLINE=true ./scripts/check-sqlite-native'
```

The command requires network access for the hash-pinned SQLite archive even with
Cargo offline. It intentionally does not turn an existing cached old library
into a silent fallback if download or verification fails.
