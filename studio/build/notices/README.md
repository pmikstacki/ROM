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

The profile also records two explicit sources:

- Vendored UI modules retain `src/lib/components/ui/LICENSE.md`.
- The Vite module-preload helper retains Vite and Rolldown notices.
  Vite 8 delegates this helper to Rolldown's native plugin.

Generated Tailwind CSS retains the installed `tailwindcss` license.
Its emitted version banner must match the installed package version.
Other generated CSS is recorded in the output inventory without an invented package owner.
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
