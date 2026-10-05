# ROM 0.0.3 dependency adoption

Review date: 2026-10-05.
Baseline: `6d3b6b24b1447a78abb96c13e5957b3654745f07`, with concurrent release changes uncommitted.
This review covers the new sortable-list dependency and the semantic Field crate's direct dependencies.
It does not certify the whole dependency graph or complete the release.

## Selected dependencies and cost

| Dependency | Exact version | Use | Declared license | Incremental adoption cost |
| --- | --- | --- | --- | --- |
| `svelte-dnd-action` | `0.9.80` | Studio ordered-list pointer, keyboard, and touch controls | MIT | One new npm runtime package. It declares no runtime dependencies and reuses the installed Svelte peer. |
| `chrono` | `0.4.45` | Compiled Gregorian date, time, explicit-offset conversion, and UTC normalization | MIT OR Apache-2.0 | Already locked and listed in the maintained inventory. `rom-fields` requests `std` with default features disabled. |
| `url` | `2.5.8` | Compiled WHATWG URL parsing and canonical encoding | MIT OR Apache-2.0 | Already locked and listed. The default `std` feature retains its existing IDNA/parser dependency graph. |
| `serde_json` | `1.0.151` | Exact validated JSON document text through `RawValue` | MIT OR Apache-2.0 | Already locked and listed. The additional `raw_value` feature declares no dependency entries. |

The drag library supplies maintained interaction primitives instead of another custom drag lifecycle.
Its [upstream release notes](https://github.com/isaacHagoel/svelte-dnd-action/blob/master/release-notes.md) describe keyboard lifecycle, ARIA, cleanup, and animation fixes.
The fetched page showed releases through `0.9.79`; it did not establish the specific `0.9.80` change history.
The exact `0.9.80` selection is established by the installed manifest, locked artifact, and registry response.
Its [official npm registry record](https://registry.npmjs.org/svelte-dnd-action/0.9.80) reported 25 archive files and 455343 unpacked bytes.
That archive size does not measure compressed assets or the bytes retained by Studio's bundler.
No new bundle-size comparison was executed in this review.

`url` documents its implementation of the [WHATWG URL Standard](https://docs.rs/url/2.5.8/url/).
ROM adds explicit scheme, credentials, whitespace, control, and length restrictions around that parser.
Parsing a URL does not fetch it.
`chrono` performs calendar and offset arithmetic; ROM's codec adds its own accepted representation and year bounds.
The crate requests no `clock` or system-local timezone feature through its direct declaration.
Other workspace users can enable additional features through Cargo feature unification.
This direct feature choice is not a claim that the complete application excludes local-time code.

## Installed licenses and artifact identity

The installed `studio/node_modules/svelte-dnd-action/LICENSE` contains the complete MIT license with attribution to Isaac Hagoel, dated 2020.
Its SHA-256 is `02c4f4f135b95f7872a8d12cb7904158e767936c1d9b07182ec87f7ece70584a`.
The package manifest and lock entry both declare MIT.
The registry and lock entry agree on this artifact integrity:

```text
sha512-GweBRVBMp0F3T/e3O8EZoukRbbi+NhYlOghn5cE69CFmkDH1AuY0P/FklTlQxAfWw+SUkSm+iZyvYaRljgKUOA==
```

The installed Cargo source archives contain `chrono-0.4.45/LICENSE.txt`, both `url-2.5.8/LICENSE-*` alternatives, and both `serde_json-1.0.151/LICENSE-*` alternatives.
Their manifests declare Rust floors of `1.62.0`, `1.63`, and `1.71`, respectively.
ROM's workspace declares Rust `1.99`.
These manifest declarations do not replace testing of the final feature graph on the declared toolchain.

Studio's [runtime notice contract](../../studio/build/notices/README.md) collects packages that contribute emitted JavaScript.
It copies their notice bytes and refuses missing license text.
The final production inventory must therefore identify `svelte-dnd-action 0.9.80` and retain its exact MIT notice.
This review inspected the installed notice and the collector contract.
It did not regenerate or validate the final release asset archive.

## Executed advisory checks

Executed from `/root/ROM/studio` with Node `v22.16.0` and npm `10.9.2`:

```sh
npm audit --omit=dev --json
npm view svelte-dnd-action@0.9.80 version license dependencies peerDependencies dist --json
```

The audit exited zero. Its report contained an empty vulnerabilities map and zero vulnerabilities at every reported severity.
The report counted 136 production dependencies and 266 total entries, including optional, peer, and development inventory categories.
`--omit=dev` limits the advisory result to the selected production graph.
This result is not a claim about omitted development packages, undisclosed vulnerabilities, or application misuse.
The registry query confirmed the exact version, license, empty dependency map, existing Svelte peer range, and artifact integrity.
No package install, audit fix, broad dependency update, build, or shared browser server was run.

The [RustSec chrono advisory RUSTSEC-2020-0159](https://rustsec.org/advisories/RUSTSEC-2020-0159.html) concerns local-time conversion and lists `>=0.4.20` as patched.
The pinned `0.4.45` satisfies that published patched range.
This targeted advisory check does not establish the absence of other Rust advisories.
The attempted RustSec `url` package page was unavailable through the browsing tool.
The requested exact-version `chrono` and `serde_json` documentation pages were also unavailable through that tool.
Their installed manifests and license files were inspected directly.
No `cargo audit` or complete Rust advisory database scan was executed in this task.

## Inventory changes required for integration

The existing [maintained dependency inventory](maintained-dependencies.md) already lists the three external Rust packages at these exact versions.
A read-only comparison of external package name/version pairs in the baseline and current Cargo locks found no added or removed external packages.
The new `rom-fields` workspace package and release version changes alter the Cargo lock identity without adding an external package row.

After the final Cargo lock is stable, regenerate the maintained inventory and `THIRD_PARTY_NOTICES.md` with the existing generator:

```sh
node scripts/dependency-inventory.mjs
```

The generator executes locked, offline, all-features Cargo metadata and writes both documents.
This review did not run it because these files remain under the release owner's control.
The recorded inventory hash is stale; changing only the header by hand would bypass the generator's graph inspection.
Keep the existing native notice supplements and limitations from the [maintained license review](maintained-license-review.md).

The Studio production build must generate its runtime notice inventory from emitted module ownership.
The new drag dependency requires its actual owner and license bytes in that generated inventory.
It does not require a manually maintained package allowlist or another copied omnibus license file.
Final asset notice validation and package gates remain release integration work.

Observed lock hashes before and after this read-only review were unchanged:

| File | SHA-256 |
| --- | --- |
| `studio/package-lock.json` | `c195945366b91a8738142a6cfe9b9cb419cf56945c48f5d8e166b254e5697a16` |
| `Cargo.lock` | `a080b892d06dd30e65917a9a13715918cbd2b02fe64827f0334d7bde7005b441` |

These hashes identify the observed concurrent release workspace, not a committed release artifact.
