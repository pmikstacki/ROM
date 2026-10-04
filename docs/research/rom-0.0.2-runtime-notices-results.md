# ROM 0.0.2 production runtime notices

Date: 2026-10-04 UTC.

The production asset archive now retains original third-party license and notice files.
The build emits a module ownership inventory. Package admission checks its notice bytes and emitted output hashes.
This is an engineering inventory. It does not certify legal completeness or authenticate an arbitrary archive.

## Source contract

Vite supports build output hooks and emitted assets through its [official plugin API](https://vite.dev/guide/api-plugin).
Rolldown exposes the modules included in a chunk through [`OutputChunk.modules`](https://rolldown.rs/reference/Interface.OutputChunk#modules).
Its [`RenderedModule.renderedLength`](https://rolldown.rs/reference/Interface.RenderedModule#renderedlength) identifies modules that render code.
The collector includes positive-length JavaScript modules. It excludes zero-length modules and unrelated installed build packages.

The installed tool versions are Vite 8.3.2 and Rolldown 1.2.12.
The pinned [Vite helper source](https://github.com/vitejs/vite/blob/v8.3.2/packages/vite/src/node/plugins/modulePreloadPolyfill.ts)
delegates module-preload generation to the pinned [Rolldown native plugin](https://github.com/rolldown/rolldown/blob/v1.2.12/crates/rolldown_plugin_vite_module_preload_polyfill/src/lib.rs).
The inventory therefore binds that virtual runtime module to both providers and retains their original notices.

Each npm module resolves to its installed package name and version.
The collector retains LICENSE, LICENCE, NOTICE, COPYING, and COPYRIGHT files, including filename variations and nested notice files.
The collector copies their bytes without text conversion. A missing SPDX field does not replace full license text.
Vendored UI modules retain the repository's upstream `LICENSE.md`.
Generated Tailwind CSS retains its installed license and must have a matching emitted version banner.

The shared inventory module serves the build plugin and package admission.
Admission rejects missing inventory, missing or changed notices, unknown owners, malformed metadata, and uncovered JavaScript/CSS outputs.
Unknown virtual JavaScript modules require an explicit reviewed classification before the build can continue.
The complete release asset inventory includes all notice files and their inventory.
Historical version-one source-only manifest verification retains its existing contract.

## Executed checks

The [behavioral RED log](evidence/rom-0.0.2/runtime-notices/admission-red.log) shows admission accepting assets without the required inventory.
The [module-ID RED log](evidence/rom-0.0.2/runtime-notices/module-id-red.log) shows distinct transformed IDs losing their query suffixes.
Both failures have corresponding passing regressions.

The [final Node log](evidence/rom-0.0.2/runtime-notices/node-final-green-49.log) records 49 passing package and artifact tests in `rom-dev`.
No tests failed or were skipped.
The [frontend check](evidence/rom-0.0.2/runtime-notices/svelte-check.log) reports zero errors and zero warnings.
Production and component builds passed. JavaScript syntax and the scoped diff check passed.
The coordinator still owns the complete verifier and real release acceptance.

The [fresh build log](evidence/rom-0.0.2/runtime-notices/fresh-fixed-build.log) records an offline installation, production build, archive, and extraction.
Both commands ran against copied inputs. Original and copied inputs retained their source identity.
The archive contains 13 files: HTML, JavaScript, CSS, the inventory, and nine notice files.
The asset inventory SHA-256 is `3b43e6974b722026f2b1035d3f70e9d5ce7371abf41d25589b063cbcbc0b6f6e`.
The [build identity](evidence/rom-0.0.2/runtime-notices/build-identity.json) records the exact inputs, lockfile, command results, and asset hashes.
This build did not execute the actual-host browser gate or the full producer.

The [runtime inventory](evidence/rom-0.0.2/runtime-notices/runtime-inventory.json) records these observed owners:

| Owner | Version | Contribution |
| --- | --- | --- |
| clsx | 2.1.1 | Emitted JavaScript |
| cn | 0.3.3 | Emitted JavaScript |
| svelte | 5.57.1 | Emitted JavaScript |
| tailwind-variants | 3.3.1 | Emitted JavaScript |
| tailwindcss | 4.3.3 | Generated stylesheet |
| vite | 8.3.2 | Module-preload helper |
| rolldown | 1.2.12 | Module-preload helper implementation |
| shadcn-svelte-vendored | source-checkout | Vendored UI modules |

The installed `svelte-toolbelt` 0.10.6 package has no SPDX field but supplies a 1,166-byte LICENSE.
It contributes no positive-length module to this production build.
An isolated library build of the real installed package retained that LICENSE unchanged.
The [byte proof](evidence/rom-0.0.2/runtime-notices/installed-byte-proof-fixed.log) compares every retained notice with its original installed or vendored file.
The [probe source](evidence/rom-0.0.2/runtime-notices/probe.mjs) reproduces both builds using a supplied checkout.
The initial probe mishandled Vite's array return shape; its [failed preparation log](evidence/rom-0.0.2/runtime-notices/installed-toolbelt-initial.log) remains available.
It was not a product defect or a passing test.

An independent peer reproduced one additional admission gap: JavaScript emitted as an asset had no module provenance.
The [regression RED log](evidence/rom-0.0.2/runtime-notices/unclassified-js-red.log) records that bypass.
The collector now refuses JavaScript and MJS assets unless they are emitted chunks with module metadata.
The corrected production and installed-package probes passed.
The peer owns the independent correction review; this author report does not claim that review is complete.

## Limits and handoff

The inventory covers emitted JavaScript modules, explicit helper provenance, vendored UI, and generated Tailwind CSS.
It does not infer undocumented upstream code ancestry, remote content, fonts, or arbitrary future plugins.
The original Vite and Rolldown notice files include their supplied third-party notices; their text remains unmodified.
Unrelated compiler packages remain outside this runtime inventory.
Legal review and the coordinator's aggregate dependency audit are separate activities.

The [source hashes](evidence/rom-0.0.2/runtime-notices/source.sha256) identify the frozen collector and admission patch.
No new dependency, workspace lockfile, native format, or identity policy changed.
The coordinator must rerun the complete fixed release profile after integration.
