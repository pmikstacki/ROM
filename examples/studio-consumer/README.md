# Independent Studio author example

This example adds a renderer through the public Studio source entry. It uses the existing generic application and client.

`src/main.ts` imports `App` and `registerRenderer` from `rom-studio`. `AuthorCode.svelte` imports the shared `Input` and `RendererProps` from that entry. Neither file imports implementation modules.

The renderer is selected by codec name and version. It does not inspect a Resource kind. The example's title makes its independently built assets visible in browser inspection.

This example registers the scalar editor with `layout: "inline"`.
The custom input appears directly in the shared Resource form. It does not require a separate edit dialog.
Saving uses the ordinary Resource mutation path and updates the displayed canonical value and revision.

## Verify the author workflow

From the ROM source root, run:

```sh
ROM_STUDIO_DEMO_BINARY=/absolute/path/to/accepted/rom-demo \
  node scripts/research/verify-studio-consumer.mjs
```

The verifier makes a source-bound Studio archive and extracts it outside the workspace. It copies this example beside that extraction. Public aliases resolve `rom-studio` to `studio/src/index.ts` and `rom-studio/styles` to `studio/src/app.css`.

The verifier creates an author package manifest from the copied pinned Studio dependency set. It changes the package name and build commands. It keeps all dependency versions and integrity records unchanged. Installation uses `npm ci --offline`.

The verifier runs type checks, builds assets and starts the existing native host with those assets. Chromium and WebKit complete actual OIDC login. They edit a custom codec field and an ordinary Resource through the same generic application.

The trusted browser tests use the copied provider and host fixtures. Those test fixtures are separate from the author's public imports. Generated packages, source archives, logs and native fixture databases are retained.

This example is a source distribution workflow. It does not establish an npm registry package. It is an agent-executed acceptance test, not a human usability certification.
