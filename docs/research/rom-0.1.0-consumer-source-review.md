# Astral Plane primary-source review for ROM 0.1.0

Research date: 2026-10-07. This is a source review and inspection of existing logs. No tests, builds, deployments, or repository edits were executed.

## Observations and evidence limits

| Boundary | Source observation | Evidence limit |
| --- | --- | --- |
| Captured validators | Producer `/root/ROM/crates/rom/src/resource/definition.rs:49` uses a function pointer. Consumer `/root/astral-plane/vendor/rom/crates/rom/src/resource/definition.rs:49` stores `Arc<dyn Fn(...) + Send + Sync>`. Its public method accepts `impl Fn(...) + Send + Sync + 'static` at line 171. | The diff changes storage, registration, and borrowing at invocation. It does not change validation ordering or receipt replay code. This review does not execute compatibility tests. |
| Runtime-specific catalog | `/root/astral-plane/src/application.rs:47` captures a supplied `CatalogView` in conversation validation. `/root/astral-plane/src/knowledge/catalog.rs:104` stores a snapshot behind `Arc<RwLock<Arc<CatalogSnapshot>>>`; `snapshot()` clones the snapshot under a read lock. | The consumer use is concrete. The vendor test at `definition_tests.rs:159` checks mutable captured state and isolation between two Definition objects. That test alone does not establish isolation between Runtime instances. |
| Studio exports | Producer `/root/ROM/studio/src/index.ts` exports Input. The consumer entry adds Button, `createBrowserAuth`, `SessionExpiredError`, Textarea, NativeSelect and its option components, Checkbox, Slider, and Label. | The feedback report's “Input and Button” baseline does not match the current producer entry. Control exports and authentication exports need separate scope decisions. |
| Control boundaries | `/root/ROM/studio/src/lib/components/ui/native-select/native-select.svelte` places `class` on `data-slot="native-select-wrapper"`; the select has `data-slot="native-select"`. Textarea binds `ref` and `value` and defaults to `field-sizing-content`. | These source contracts support the reported form friction. They do not establish browser behavior outside the reported consumer checks. |
| Installation paths and notices | Producer `/root/ROM/studio/build/notices/ownership.mjs:33` classifies lexical paths under Studio's node_modules. Consumer resolves that configured installation and package directories, then retains logical `node_modules/...` IDs. | The consumer fix addresses a whole node_modules symlink. It does not establish support for every package-manager layout, individually linked packages, or pnpm stores. |
| Notice regression | `/root/astral-plane/vendor/rom/scripts/packages/studio-notices.test.mjs:156` simulates canonical emitted paths from a shared installation. It checks portable module IDs and exact license bytes. It also rejects unrelated external modules and missing licenses. | Existing package identity validation remains in `packageOwner`. No new test run was performed during this review. |
| Distribution boundary | `/root/astral-plane/web/vite.config.ts:9` aliases ROM Studio and styles to vendored source and deduplicates Svelte. `/root/ROM/studio/package.json` is private and has no package exports. Its Vite config builds application/component HTML entries. | Successful source consumption is useful integration evidence. It does not demonstrate installation or imports from a published Studio package. |

The patch rationale is `/root/astral-plane/docs/rom-catalog-validator-patch.md`. The upstream feedback handoff is `/root/ROM/docs/research/astral-plane-production-feedback-2026-10-07.md`.

Existing log endings were inspected: `/var/tmp/astral-rom-studio-check.log` reports zero errors and warnings. `/var/tmp/astral-rom-studio-unit.log` reports 146 passing tests. `/var/tmp/astral-rom-notice-admission.log` reports 15 passing tests. `/var/tmp/astral-rom-components-fixed.log` reports a completed component build. `/var/tmp/astral-fields-browser.log` reports 32 passing browser tests. These artifacts do not establish a full producer verifier run on the final combined revision.

## Official sources and implications

Rust permits non-capturing closures to coerce to function pointers. A captured validator therefore needs a different callable boundary. [Rust Reference: closure call traits and coercions](https://doc.rust-lang.org/reference/types/closure.html#call-traits-and-coercions).

`Arc` provides shared ownership. Its value must separately satisfy Send and Sync for thread-safe sharing. The patch's explicit callback bounds address that requirement. They do not establish bounded duration, absence of I/O, or freedom from reentry. [Rust standard library: Arc thread safety](https://doc.rust-lang.org/std/sync/struct.Arc.html#thread-safety).

Vite defaults `resolve.preserveSymlinks` to false and uses real paths for file identity. This supports the notice regression's path explanation. Linked dependencies can also need deduplication. [Vite shared options](https://vite.dev/config/shared-options#resolve-preservesymlinks), [Vite dependency deduplication](https://vite.dev/config/shared-options#resolve-dedupe).

Vite documents library builds, dependency externalization, package exports, and CSS exports. Those are distribution choices to investigate; adding source exports alone does not establish packaged acceptance. [Vite library mode](https://vite.dev/guide/build.html#library-mode), [Vite library CSS exports](https://vite.dev/guide/build.html#css-support).

## Proposed experiments

1. Compile an external Rust fixture with named function pointers, non-capturing closures, and captured immutable catalog snapshots. Reject callbacks that fail Send, Sync, or lifetime requirements.
2. Register separate captured catalogs in two Runtime instances. Change one catalog and verify that the other runtime retains its validation result.
3. Exercise denied transitions, accepted transitions, callback panic, retry, matching receipt replay, and changed replay authorization. Verify persisted revisions and events independently of application response quality.
4. Consume the candidate Studio distribution through its declared public package entry and style entry. Remove ROM source aliases. Check supported controls, authentication exports if included, Svelte deduplication, and public types.
5. Exercise ordinary node_modules and the consumer's whole-directory symlink. Verify identical package owners, complete license bytes, portable IDs, and rejection of unrelated modules, invalid identities, and missing licenses.
6. Investigate individual package links and other supported installation layouts separately. Decide whether they are supported before widening ownership admission.
7. Document select wrapper/control styling and a bounded chat composer. Check ref binding, numeric properties, date/time inputs, disabled states, and keyboard access through external imports.
8. Reproduce autosave through public APIs: persist command identity, lose acknowledgement, retry, reload, and change permission between steps. Assert one committed mutation and safe replay disclosure.

Treat AI provider deadlines, generic question selection, personal-experience routing, view sequencing, and chat scrolling as application defects unless a ROM boundary reproduces them. The reported enrichment coordinator suggests work API discoverability research. It does not prove missing ROM capabilities.
