# Production runtime notices

The build emits `third-party-notices.json` and the original notice files under `notices/`.
The package gate validates these files before it creates the Studio asset archive.

The inventory uses the modules in emitted JavaScript chunks.
It includes modules with positive `renderedLength` from Rolldown's `OutputChunk.modules`.
It excludes modules that render no code and packages used only by the build tools.
This is an engineering inventory. It does not certify legal completeness.

The collector resolves each npm module to its installed package and version.
It retains regular files named LICENSE, LICENCE, NOTICE, COPYING, or COPYRIGHT, including filename variations and nested notice files.
It copies their bytes without text conversion.
A missing SPDX field does not prevent collection when the package supplies license text.
The collector refuses an emitted third-party module when it cannot find full license text.

The profile also records these explicit sources:

- Vendored UI modules and the exact copied `src/lib/hooks/is-mobile.svelte.ts` hook retain `src/lib/components/ui/LICENSE.md`.
- Vendored SVAR filter modules retain the original `src/lib/filters/vendor/LICENSE` bytes.
  Their owner identifies commit `1c581c3312c626c525ee64b8f94446a025fa141c` of `svar-widgets/filter`.
  Collection checks the recorded repository, commit, and MIT license. Admission requires this owner and the pinned license hash.
- The Vite module-preload helper retains Vite and Rolldown notices.
  Vite 8 delegates this helper to Rolldown's native plugin.

Generated Tailwind CSS retains the installed `tailwindcss` license.
Its emitted version banner must match the installed package version.
Other generated CSS is recorded in the output inventory without an invented package owner.
Explicit imports of `shadcn-svelte/tailwind.css` and `tw-animate-css` retain their installed package licenses.
Their generated CSS entries record the resolved source path and source SHA-256.
These entries identify imported source. They do not reconstruct a CSS transformation map.
The collector refuses unknown virtual JavaScript modules and files outside the source workspace.
It also refuses JavaScript emitted as an asset without a chunk module map.
Add reviewed provenance before you enable a new plugin that injects runtime code.

The shared validator checks the schema, owner bindings, notice hashes, output hashes, and complete JavaScript/CSS coverage.
It rejects missing, changed, or unlisted notices.
The release package's complete asset identity covers the inventory and notice files.
These checks establish internal correspondence. They do not authenticate an arbitrary third-party build or archive.

The interface comes from the official [Vite plugin API](https://vite.dev/guide/api-plugin).
Rolldown documents the [chunk module map](https://rolldown.rs/reference/Interface.OutputChunk#modules)
and [rendered code length](https://rolldown.rs/reference/Interface.RenderedModule#renderedlength).
The pinned [Vite 8.3.2 source](https://github.com/vitejs/vite/blob/v8.3.2/packages/vite/src/node/plugins/modulePreloadPolyfill.ts)
delegates the helper to the pinned [Rolldown 1.2.12 plugin](https://github.com/rolldown/rolldown/blob/v1.2.12/crates/rolldown_plugin_vite_module_preload_polyfill/src/lib.rs).
