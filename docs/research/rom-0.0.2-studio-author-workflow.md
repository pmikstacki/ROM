# Independent Studio author workflow

An external author example passed four real-browser tests. Chromium and WebKit each tested SQLite and redb with an actual OIDC provider.

The author application compiles against an independently extracted Studio source tree. It adds a custom field renderer through the public source entry. The generic application and SDK remain unchanged.

## Public entry

`studio/src/index.ts` is an export facade. It exposes `App`, `createClient`, `RemoteError`, `registerRenderer`, `RendererProps`, `FieldRenderer`, shared client types and `Input`.

The example imports `rom-studio` and `rom-studio/styles`. Its application source does not import private implementation paths. Vite resolves those names to the extracted public entry and stylesheet.

The custom renderer is selected by codec name and version. It does not inspect Resource names. It normalizes a ticket code through the declared field callback. The framework retains the generic mutation path.

The example also edits an ordinary task through the same application. It supplies no Resource controller, repository or transport route.

## Independent extraction

The verifier uses the maintained `studioSource`, `copyStudio`, `directoryArchive` and `extract` contracts. It captures the complete Studio input inventory before the build.

The verifier makes a source archive and extracts it into a retained directory outside the workspace. It checks the extracted inventory against the source inventory. It checks both inventories again after browser acceptance.

The author package uses the copied, pinned dependency graph. Its package name and build commands change. Dependency versions and integrity records remain unchanged. `npm ci --offline` installs the application and provider fixture.

The extracted SDK resolves dependencies through a symlink to the extracted consumer's `node_modules`. Both paths are inside the independent extraction. The consumer does not resolve modules from the ROM checkout.

The trusted browser fixture is copied with the Studio sources. The actual provider fixture is copied separately. These test fixtures are outside the application's public imports.

The final verifier requires `ROM_STUDIO_DEMO_BINARY`. It hashes that explicit binary before acceptance and checks it again afterwards. It has no implicit mutable build-target fallback.

## Executed acceptance

Each browser completes authorization-code login with the real provider. It then opens the held-out `maintenance-tickets` Resource.

The author renderer appears as `Author code`. Editing `code` through that renderer commits a normalized value at revision 2. The browser then edits an ordinary `tasks` Resource through the unchanged generic view.

| Check | Result |
| --- | --- |
| Offline author install | Passed; 92 pinned packages installed. |
| Type checks | Passed; zero errors and zero warnings. |
| Independent asset build | Passed. |
| Offline provider install | Passed; existing `oidc-provider` lock used. |
| Chromium with SQLite | Passed. |
| Chromium with redb | Passed. |
| WebKit with SQLite | Passed. |
| WebKit with redb | Passed. |
| Studio source identity fences | Passed. |
| Native binary identity fence | Passed. |

## Earlier failures

The first run failed type checking. TypeScript needed a declaration for the source consumer's `rom-studio/styles` alias. The example now declares that public stylesheet module.

The second run passed type checking and build. Its copied provider install failed because the host npm cache lacked `vary` 1.1.2.

A separate preparation command installed the provider's existing pinned lock into the retained scratch. It fetched no new dependency version. The complete third acceptance run then installed both graphs offline.

These failures are retained. The cache preparation is not described as an offline acceptance command.

## Evidence and limits

The passing third run is retained at `/var/tmp/rom-studio-author-MAv3nu`. Its `result.json` records the complete source inventory, archive hash and commands. The browser log records four passing cases.

That run used binary SHA-256 `c93a20cfc1d92cc6e3e6f613105bbbcc61468179008fc118fafabb994d1c88f3`. The parent bound this binary to the inventory-action source and build evidence. It superseded the earlier inspected `60576302` binary before acceptance captured its identity.

Initial passing source SHA-256 was `3ca48224b0e4bba45ef9bcc11d15659915540e421337627d7617684b7aa46193`. Its archive SHA-256 was `25747994e340614e9ca1ac4be3fe5847d6ff81ac59eddef23682c4e2c664c237`.

The final repeat passed with an explicit immutable binary and exact copied author paths. It used `/var/tmp/rom-studio-inventory-action-acceptance/rom-demo`.

Final evidence is retained at `/var/tmp/rom-studio-author-JJa0kn`. Its source SHA-256 is `ea40116c413f05b6030178b4a296bb35342c5fa47bf69f4d57e03c23889f457f`. Its source archive SHA-256 is `1aeea778f754cc96535dae2974ffd11f3b609c829a9ed5964b7281b12f444e8d`.

The final browser run passed four cases in 5.4 seconds. Every recorded command returned zero without a timeout or spawn failure.

The [result inventory](evidence/rom-0.0.2/studio-consumer/result.json), [browser log](evidence/rom-0.0.2/studio-consumer/browser.log), [type check](evidence/rom-0.0.2/studio-consumer/type-check.log) and [build log](evidence/rom-0.0.2/studio-consumer/build.log) are copied without changes.

## Review and release integration

Independent review found that `spawnSync` timeout could leave test descendants alive. The verifier now awaits the existing `runChild` process-group supervisor.

Each command has a 180-second limit and an 8 MiB output bound. Logs record exit code, timeout and spawn failure. The final browser run used this corrected supervisor. Its existing two process-control tests also passed.

The standalone WebKit preparation command uses the same supervisor. Its result is recorded in [webkit-runtime.log](evidence/rom-0.0.2/studio-consumer/webkit-runtime.log).

A second review required identity checks for the copied author files. The verifier now checks them before commands and after acceptance. Generated package manifests remain outside that author input inventory.

A fault-injection run changed only the extracted `AuthorCode.svelte` after its asset build. All four browser cases passed, but final admission returned exit 1 for copied source drift. This is an intentional negative control, not a product defect. Its [raw log](evidence/rom-0.0.2/studio-consumer/copied-source-drift-control.log) and scratch directory `/var/tmp/rom-studio-author-drift-STFkNB` are retained.

`demo/verify-studio` now retains an immutable copy of its freshly built native binary. After the normal host browser gate, it runs the external author verifier against that copy. It checks the binary hash afterwards.

This preserves the fixed release gate command and manifest gate count. `bash -n` passed for the integration script. The complete integration command was not run here because its native build remains subject to the capacity gate.

The report does not claim an npm registry distribution. This is a source archive workflow. The author package uses the maintained Studio dependency set rather than a newly minimized dependency manifest.

This is agent-executed acceptance. It does not certify human usability. Production Studio asset notices and release gates remain separate requirements.

No native build ran for this task. Seven retained scratch directories used 1,594,933,248 allocated bytes in total. Final root free space was 103,171,870,720 bytes.
