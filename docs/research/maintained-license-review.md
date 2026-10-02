# Maintained dependency notice review

Review date: 2026-10-02. Question: do the six missing-file messages in the generated notices indicate missing upstream license evidence, and which distribution artifacts do they affect?

Reviewed main `666b85a` and lockfile SHA-256 `93f51f4bfa2a95d96becf960cb8e00ae32c5767c82c7d5ba8891397637681610`. The regenerated all-features Cargo metadata contains 255 external packages. The reported 269-package audit includes local packages; that audit count and absence of advisories do not establish notice completeness. This is an engineering inventory review, not a legal certification.

## Result

All six have attributable upstream license evidence. Five crate archives omit the repository-root license files; `r-efi` actually carries its license and copyright text in `AUTHORS`, a filename the generator does not match. None of the six appears in the current workspace's `x86_64-unknown-linux-gnu` normal/build dependency tree, even with the workspace's all-features selection. Their presence in the lockfile is not evidence that they are linked into the current Linux demo.

The current source release is a `git archive` of ROM, not a vendored dependency bundle or executable release. Cargo `.crate` packages also do not bundle their transitive dependencies. These six missing generated entries therefore do not demonstrate that the current source archive has omitted notices for code it actually incorporates. A future vendored archive, Android/Wasm/UEFI build or executable distribution needs an artifact-specific review. The root generated document should still be corrected because it presents itself as a collection for the all-target graph.

## Six-package findings

The upstream revisions below come from each downloaded crate's `.cargo_vcs_info.json`; these are immutable sources, not inferred current branch licenses. Recursive archive inspection found no separately named nested license files for these six. Source `.rs` scans did not provide a replacement license/attribution set for the five omissions; manifest SPDX expressions are declarations, not complete notice text.

| Locked package | Actual evidence and attribution | Current relevance / proposed collection repair |
|---|---|---|
| `jni 0.22.4` | README declares MIT OR Apache-2.0 and refers to missing license files. Root MIT notice at revision `5ae9458a4ec44c5318f37ddc7569c1d4ae8a69e7` attributes **2016 Prevoty, Inc. and jni-rs contributors**. [Exact MIT notice](https://raw.githubusercontent.com/jni-rs/jni-rs/5ae9458a4ec44c5318f37ddc7569c1d4ae8a69e7/LICENSE-MIT). | Android dependency through rustls-platform-verifier; an optional non-target-specific JNI dependency also exists upstream but is not enabled in this Linux graph. A pinned root-license supplement would remove the placeholder. |
| `jni-macros 0.22.4` | README has the same dual-license declaration. Its archive records a **different** upstream revision, `33045a124105c939d1e2cbdcb5a39e5d868ffa03`; that revision's root MIT notice has the same Prevoty/contributors attribution. [Exact MIT notice](https://raw.githubusercontent.com/jni-rs/jni-rs/33045a124105c939d1e2cbdcb5a39e5d868ffa03/LICENSE-MIT). | Macro dependency of JNI. Preserve its own source revision in the supplement even if the notice text is deduplicated. |
| `jni-sys-macros 0.4.1` | Manifest declares MIT OR Apache-2.0. Revision `64d77b7a5f119d7b55b4e2c169a4668067ff59e6` has the root MIT notice attributed to **2015 The rust-jni-sys Developers**. [Exact MIT notice](https://raw.githubusercontent.com/jni-rs/jni-sys/64d77b7a5f119d7b55b4e2c169a4668067ff59e6/LICENSE-MIT). | Reached through jni-sys. The generated document already reproduces jni-sys's notice, but does not establish that mapping for this macro package. Add a pinned, explicit mapping rather than inventing another copyright. |
| `r-efi 6.0.0` | `AUTHORS` includes the MIT permission/warranty text, Apache/LGPL alternatives and copyrights for **2017–2023 Red Hat, Inc.; 2019–2023 Microsoft Corporation; 2022–2023 David Rheinsberg**. README points directly to it. [Exact AUTHORS](https://raw.githubusercontent.com/r-efi/r-efi/7e1b0322d31d625f81a5656096330934f9cd835d/AUTHORS). | Reached by getrandom only under `cfg(all(target_os = "uefi", getrandom_backend = "efi_rng"))`. This is a generator filename omission, not an absent archive notice. Collect this exact AUTHORS file. The SPDX expression is alternatives (`OR`), not a requirement to combine all three licenses. |
| `rsqlite-vfs 0.1.1` | Manifest declares MIT but omits repository metadata. Its archive revision `1bb309784f25c401ddf95131dcde4c4cdad7aee7`, path `crates/rsqlite-vfs`, belongs to sqlite-wasm-rs. Root license attributes **2024 Spxg**. [Exact MIT notice](https://raw.githubusercontent.com/Spxg/sqlite-wasm-rs/1bb309784f25c401ddf95131dcde4c4cdad7aee7/LICENSE). | Reached via sqlite-wasm-rs under rusqlite's Wasm/unknown target selection. It is not the native Linux SQLite VFS. Supplement from the pinned repository root; the registry index URL alone is not a useful attribution source. |
| `rustls-platform-verifier-android 0.2.0` | Manifest declares MIT OR Apache-2.0. The root MIT file at revision `252e25161a91af476cbea620e29d277592d27ec7` attributes **2022 1Password**. [Exact MIT notice](https://raw.githubusercontent.com/rustls/rustls-platform-verifier/252e25161a91af476cbea620e29d277592d27ec7/LICENSE-MIT). | Android-only shim. Its `src/lib.rs` explains that it synchronizes the separately obtained Android component's version with Gradle; the Rust crate is not evidence that an Android AAR was bundled in this Linux release. Review the actual AAR/Gradle graph if Android distribution is added. |

Full license texts were not copied into this research note: they are not missing from an artifact containing these six implementations in the current source-only release, and a second manually maintained omnibus notice document would drift. The exact sources above are sufficient to implement narrow generated-file supplements. They should be copied verbatim, with revision/checksum provenance, if that generator repair or affected artifact packaging is performed; do not replace them with a generic unattributed MIT template.

## Native bundled code and nested-file coverage

The absence of those six Linux dependencies does **not** settle native attribution. The Linux normal/build tree includes `aws-lc-sys 0.45.0` and `libsqlite3-sys 0.38.2`.

- The generator includes both `aws-lc-sys/LICENSE` and `aws-lc-sys/aws-lc/LICENSE`. These are consolidated notices covering AWS-LC, BoringSSL/OpenSSL lineage and named incorporated components, and include the license bodies. They are not byte-identical and should not be deduplicated merely by basename. The nested `aws-lc/third_party/fiat/LICENSE` is outside the generator's traversal, but the consolidated notice already names the fiat-crypto authors, their 2015–2020 copyright, MIT terms and the nested file. This spot check found attributable coverage, not an unexplained missing native crypto license. Preserve the distributed consolidation; don't reduce AWS-LC to the root project's license expression.
- The generated libsqlite3-sys entry includes the Rust binding's MIT license. Its bundled `sqlite3/sqlite3.c` identifies the SQLite amalgamation and retains its public-domain headers. SQLite's official statement applies to deliverable library code, while distinguishing some build scripts; it should not be generalized to every file in a vendor source tree. [SQLite copyright statement](https://sqlite.org/copyright.html). `sqlcipher/LICENSE` is nested but SQLCipher is not enabled in the measured Linux feature tree; enabling or bundling that alternative needs its own notice inventory.
- `ring 0.17.14` is in the all-target lock graph but absent from this Linux normal/build tree. Its top-level `LICENSE` explicitly refers to `src/polyfill/once_cell/LICENSE-APACHE` and `LICENSE-MIT`; those nested files, plus `third_party/fiat/LICENSE`, are missed by the generator's directory-name filter. The current generated ring section should not be treated as exhaustive for a future artifact that uses or vendors ring. Preserve those exact archive files and inspect the indicated source headers when that profile is distributed.
- `object_store`'s Apache `NOTICE` and license are already collected at its crate root; ROM's object-store adapter also preserves its explicit upstream `NOTICE`/license files. These are separate from ROM's MIT license.

This was a targeted native spot check prompted by the six omissions, not a line-by-line survey of all 255 external packages or of the Nix/toolchain/system-library closure. Compiler/build-time inclusion, generated output and statically linked implementation are different scopes; the dependency tree establishes candidates, not binary symbol attribution.

## Reproduction and recommendation

Read-only commands used in native `rom-dev`, from `/workspace/ROM`:

```sh
sha256sum Cargo.lock
cargo metadata --locked --offline --all-features --format-version 1
cargo tree --locked --offline --workspace --all-features \
  --target x86_64-unknown-linux-gnu --edges normal,build --prefix none
```

Metadata was redirected to a file, avoiding an exec capture limit; the existing generator uses a 32 MiB metadata buffer. Package archives were inspected under Cargo's `registry/src/index.crates.io-1949cf8c6b5b557f`, including their manifests, VCS records, README, recursive license filenames and source-header matches. `scripts/release` and `scripts/check-packages.mjs` establish the source-only packaging boundary. No generated notice/inventory, application code, lockfile or main worktree file was changed by this review.

Recommended next change: teach notice collection the one explicit `r-efi/AUTHORS` path; add version/revision-pinned repository-root MIT supplements for the five archive omissions; record nested ring notices before advertising all-target notice completeness. Keep an artifact manifest containing target, features, dependency graph and external native inputs beside any future executable or vendored release. The current source release can accurately be described as carrying an all-target declaration/notice inventory with these documented collection gaps, rather than as a certified complete binary notice bundle.

## Collector repair completed

The identified collection gaps are now resolved in the collector: version-specific archive paths include `r-efi 6.0.0/AUTHORS` and all three identified nested ring license files. Five unmodified repository-root MIT supplements are checked into `licenses/dependency-supplements/`, with exact source URLs, source revisions and SHA-256 hashes in its manifest. Collection verifies each package's archive VCS revision and the supplement bytes before including them. The output retains the crate's original declared license expression and labels the reproduced MIT alternative explicitly.

Generation now uses locked **offline** Cargo metadata and makes no notice download. Focused tests cover positive collection, wrong version, tampered bytes, mismatched source revision, extra archive paths and missing-file failure. An actual generation against the cached 255-package all-features graph completed and all six placeholders were absent; all three nested ring paths appeared. Generated inventory/notice outputs are intentionally left to the coordinator's main-lock regeneration, outside this repair commit. This closes the identified collector gaps, not the broader artifact-specific/native/system-library review boundary above.
