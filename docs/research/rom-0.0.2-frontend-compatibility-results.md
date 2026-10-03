# ROM 0.0.2 frontend compatibility results

Date: 2026-10-03. Status: bounded executable trial; not the Studio product.

## Result

The selected Svelte 5, Vite 8, Tailwind 4, Bits UI, and imported shadcn controls passed the trial's Chromium checks.
A clean locked install, type checks, production build, and npm audit also passed.
WebKit acceptance remains incomplete because downloaded Linux binaries cannot use the current NixOS runtime directly.
This limitation must be repaired before the full Studio browser acceptance gate.

The trial does not contain a ROM backend, session flow, discovery renderer, or Resource mutation.
It establishes component and build compatibility. It does not establish the release's end-to-end guarantees.

## Source and tool identity

The trial source remains at `/var/tmp/rom-0.0.2-frontend-stack`.
The [retained source archive](evidence/rom-0.0.2/frontend-stack/trial-source.tar.gz) excludes dependencies, generated assets, and browser failure output.
The [verification record](evidence/rom-0.0.2/frontend-stack/verification.json) records commands and results.

| Item | Executed value |
| --- | --- |
| ROM worktree base | `91954b66a1d2808a8747ec79700e3c41196daf21` |
| Worktree state at trial | New frontend evidence; trial source is outside the ROM tree. |
| Node | `v22.16.0` |
| npm | `10.9.2` |
| Browser used | System Chromium `138.0.7204.49` |
| npm lock SHA-256 | `06c59e8bbb8a57acc4394eef9b1dac5e24f70ab0cd435dd99ea39e29bbdc8e24` |
| Base path | `/rom-studio/` |

The standalone package pins Svelte `5.57.1`, Vite `8.3.2`, plugin `7.3.1`, Tailwind `4.3.3`, and Bits UI `2.19.5`.
It also pins TypeScript `6.0.3`, svelte-check `4.7.6`, shadcn CLI `1.7.0`, and Playwright `1.63.0`.
The [package manifest](evidence/rom-0.0.2/frontend-stack/package.json) contains all exact direct versions.
The [lockfile](evidence/rom-0.0.2/frontend-stack/package-lock.json) preserves the resolved graph.

## Imported component provenance

The trial imported actual Button, Input, Dialog, Table, and utility sources from the official registry.
The current working registry path is `https://shadcn-svelte.com/registry/styles/vega/`.
Earlier attempted `/r/` and `new-york` paths returned HTML errors. Those paths were not used as component sources.

Each raw registry response is retained. The [provenance manifest](evidence/rom-0.0.2/frontend-stack/provenance/manifest.json) records URL and SHA-256.
It records both upstream content and adapted content hashes for each component file.
The upstream MIT license is retained without prose changes.

The importer replaced registry alias placeholders with trial aliases. Dialog's upstream `IconPlaceholder` was resolved to the actual Lucide `XIcon`.
The adaptation is explicit in the importer and provenance record. No handwritten Button, Input, Dialog, or Table substitute was used.
The trial supplied its own CSS tokens and page layout. These are separate from imported component behavior.

## Executed checks

| Command | Result | Evidence |
| --- | --- | --- |
| `npm ci --ignore-scripts` | Exit 0; clean install from lock. | [Install log](evidence/rom-0.0.2/frontend-stack/verified-clean-install.log) |
| `npm run check` | Exit 0; zero errors and warnings. | [Type log](evidence/rom-0.0.2/frontend-stack/verified-typecheck.log) |
| `npm run build` | Exit 0; production assets built for the configured base path. | [Build log](evidence/rom-0.0.2/frontend-stack/verified-build.log) |
| `npm test -- --project=chromium` | Exit 0; three browser tests passed. | [Browser log](evidence/rom-0.0.2/frontend-stack/verified-browser-chromium.log) |
| `npm audit --json` | Exit 0; zero reported vulnerabilities at execution. | [Audit log](evidence/rom-0.0.2/frontend-stack/verified-audit.log) |
| Deliberately invalid Button variant | Type check rejected the invalid literal at the fixture source. | [Negative diagnostic](evidence/rom-0.0.2/frontend-stack/negative-prop-diagnostic.log) |

Install scripts were disabled deliberately. The resulting packages still executed the actual compiler, production bundler, and browser harness successfully.
This is the tested install mode; it does not imply that every future dependency supports disabled scripts.

The first type check found a missing CSS side-effect declaration in the trial.
Adding the standard `vite/client` ambient reference resolved it. It was a scaffold issue, not a component compatibility failure.
The negative variant fixture then failed for its intended reason. After removal from compilation, the type check passed again.

Tailwind automatic scanning produced different CSS sizes after generated test artifacts appeared.
The final trial uses explicit source scanning limited to `src`.
The product must exclude evidence and generated test output from its CSS source selection.

## Browser behavior tested

The browser tests used the production build through a local Vite preview server.
This preview server is a test harness, not the permanent deployment service.

The first test verifies imported controls, input-to-table updates, Enter activation, disabled buttons, and absence of page errors.
The second verifies dialog entry focus, eight Tab steps inside the dialog, Escape closure, and restored trigger focus.
The third runs axe A/AA scans on the base page and the open dialog. Both scans reported no violations.
These results cover the assembled fixture only. They do not certify accessibility or human usability.

The harness used the Nix-installed Chromium executable explicitly. It did not use Playwright's downloaded Chromium build.
All three tests passed on that executable. No assertion depends on treating its version as the bundled browser revision.

## WebKit runtime diagnosis

Playwright downloaded WebKit revision `2359` and Chromium revision `1243` into the retained cache.
Initial browser launch checks failed before any page interaction. Their [failure log](evidence/rom-0.0.2/frontend-stack/playwright-first.log) is retained.
The [installation log](evidence/rom-0.0.2/frontend-stack/browser-install.log) lists missing shared libraries.

The host has no configured `NIX_LD` or `NIX_LD_LIBRARY_PATH`.
Its `/lib64/ld-linux-x86-64.so.2` points to a NixOS stub loader.
An explicit existing glibc loader and existing library paths reached a concrete missing dependency: `libavif.so.16`.
The [loader probe](evidence/rom-0.0.2/frontend-stack/webkit-loader-probe.log) records exit 127 and that error.

The upstream WPE launcher overwrites `LD_LIBRARY_PATH`. A parent environment variable alone cannot supply the missing closure reliably.
The four WPE ELF helpers also need a compatible interpreter.
The [runtime probe record](evidence/rom-0.0.2/frontend-stack/webkit-runtime-probe.json) identifies existing loader, patchelf, pinned Nix source hash, and candidate missing outputs.

The next bounded trial must use a local copied runtime wrapper or a pinned Nix FHS environment.
It must supply matching SONAMEs, route all ELF helpers through the correct loader, and run the WebKit tests.
The candidate closure is not accepted or proven minimal. No system configuration or infrastructure file was changed by this trial.
WebKit evidence cannot be replaced with Chromium evidence. A successful WebKit run would still not test the owner's actual Safari executable.

## Dependency and license review

The installed-package inventory contains 88 entries. One package, `svelte-toolbelt` `0.10.6`, omits its license field.
Its distributed LICENSE identifies MIT. The [unchanged license](evidence/rom-0.0.2/frontend-stack/svelte-toolbelt-LICENSE) is retained.
The [inventory](evidence/rom-0.0.2/frontend-stack/licenses.json) preserves the metadata gap rather than inventing a manifest value.
License metadata and this source review are not legal certification.

The shadcn CLI is an exact development dependency for this trial. Production pages consume copied component code.
The product can avoid shipping the CLI in runtime dependencies. Component updates require source review even when package advisories are clear.

## Adoption implications

Use this locked stack as the implementation starting point. Preserve the real component provenance and the explicit Tailwind source boundary.
Keep generic Resource semantics outside the UI primitives. Keep lossless operations and authorization checks in the shared browser client and Rust pipeline.

The next gates are real discovery rendering, human OIDC sessions, absent/null/value codecs, live lifecycle, and unknown-outcome recovery.
Run them against SQLite and redb. Add a held-out Resource without component edits.
Complete WebKit runtime repair and browser acceptance before release. This trial does not mark those gates complete.
