# Independent public-control consumer

The fixture imports controls from `rom-studio/controls` and CSS from `rom-studio/styles`.
It uses source-package exports without `$lib` or Studio source aliases.
The root facade retains its existing names. Alias-free root App consumption is outside this fixture.

## Source and dependency contract

The exported stylesheet owns its imported `shadcn-svelte` and `tailwindcss` packages as runtime dependencies.
A consumer compiles source CSS with the supported Tailwind/Vite pipeline. Animation CSS remains a runtime dependency.
The fixture declares ROM, Svelte, and selected build/typecheck/browser tools. It does not copy the producer's complete development dependency graph.
Its checked-in package-lock records the independent consumer graph, including the installed local ROM package.
Ordinary verification runs `npm ci --install-links`. It checks lock identity, installed versions, and installed source hashes.
Cached or registry retrieval may supply missing package bytes. No node_modules copy or package symlink is accepted.

When source dependency metadata or version changes, generate a new candidate consumer lock:

```sh
node studio/tests/public-controls/verify.mjs /var/tmp/new-controls-bootstrap --bootstrap-lock
```

Review the candidate `consumer/package-lock.json` before updating the fixture lock.
Bootstrap evidence is labeled separately. It cannot satisfy release admission.

## Authoring verification

Use a new evidence directory:

```sh
node studio/tests/public-controls/verify.mjs /var/tmp/new-controls-authoring
```

The verifier archives the current source and labels this mode `authoring_checkout`.
A reused output path fails before modifying historical evidence.
Run the fast admission tests with `node --test studio/tests/public-controls/verification.test.mjs`.

## Release verification

Supply a candidate archive with the fixed `rom-studio-source/` prefix.
Its package root must contain `package.json`, `package-lock.json`, `src`, `LICENSE`, and `THIRD_PARTY_NOTICES.md`:

```sh
ROM_PUBLIC_CONTROLS_BROWSER=1 ROM_WEBKIT_EXECUTABLE=/path/to/pw_run.sh node studio/tests/public-controls/verify.mjs /var/tmp/new-controls-release --source-archive /path/to/studio-source.tar.gz --admit
```

For an already verified, extracted framework artifact, use `--source-dir /path/to/extracted/ROM/studio` instead.
The provided archive or directory must come from the release artifact being checked.
The verifier compares every installed source file with its supplied source and records the archive and lock hashes.
Release admission requires locked installation, matching installed source, and passed Chromium and WebKit outcomes.
It rejects authoring mode, bootstrap mode, missing engines, skipped browser work, and failed tests.

Set `ROM_WEBKIT_EXECUTABLE` from `./scripts/studio-browser-runtime`.
Chromium uses `ROM_CHROMIUM_PATH` or the repository default.
Coordinate port 43281 before browser execution. Each child command has a 180000ms timeout and an 8MiB output limit.
The fixture uses its own production build with one browser worker. It does not use the shared component server.
Authoring runs can omit browser work; their completed result does not establish release admission.

## Existing prop boundaries

| Control | Bindable state | Ref target | Class target |
| --- | --- | --- | --- |
| Input | value; files for file input | input | input |
| Textarea | value | textarea | textarea |
| NativeSelect | value | select | wrapper; inner select uses `[data-slot="native-select"]` |
| NativeSelectOption | native option attributes | option | option |
| NativeSelectOptGroup | native group attributes | optgroup | optgroup |
| Checkbox | checked; indeterminate | bits-ui button host | host |
| Slider | existing single or multiple representation | bits-ui root host | root host |
| Button | native button/anchor props | button or anchor | button or anchor |
| Label | label props | label | label |

Textarea retains content sizing. The fixture applies fixed, bounded composer sizing.
Browser assertions exercise package CSS, wrapper/inner-select styling, refs, value bindings, keyboard access, and two viewports.
The selected cases do not establish every file/date/number, indeterminate, multiple-slider, or anchor variant.
No font file is referenced by the inspected stylesheet. SSR and new font assets require separate acceptance.

The coordinator integrates this fixture into local and release gates. Notice admission and complete artifact checks remain separate requirements.

Candidate source archives use the fixed `rom-studio-source/` prefix and include ROM's license and source notices.
Before extraction, admission bounds compressed/expanded bytes, member counts, and logical file sizes. It rejects links and unsafe member paths.
A candidate archive and successful fixture are evidence for review. They are not a completed release distribution.
