import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
import { runtimeNotices } from "./build/runtime-notices.mjs";
export default defineConfig(({ mode }) => ({
  base: "/rom-studio/",
  plugins: [
    svelte(),
    tailwindcss(),
    runtimeNotices(),
    ...(mode === "components"
      ? [
          {
            name: "explicit-component-test-bootstrap",
            transformIndexHtml(html: string) {
              const store = {
                maxBytes: 1052672,
                maxSlots: 128,
                timeoutMs: 10000,
              };
              const profile = {
                version: 1,
                authority: "component-fixture",
                recovery: {
                  namespace: "component-fixture",
                  retryEpoch: "0",
                  maxBytes: 1048576,
                  intentStore: { ...store, name: "component-intents" },
                  editorStore: { ...store, name: "component-editors" },
                },
              };
              return html.replace(
                "</head>",
                `<script type="application/json" id="rom-studio-auth-profile">${JSON.stringify(profile)}</script></head>`,
              );
            },
          },
        ]
      : []),
    ...(mode === "studio-demo"
      ? [
          {
            name: "explicit-demo-entry",
            transformIndexHtml: {
              order: "pre",
              handler(html: string) {
                return html.replace("/src/main.ts", "../demo/studio/main.ts");
              },
            },
          },
        ]
      : []),
  ],
  resolve: {
    alias: { $lib: fileURLToPath(new URL("./src/lib", import.meta.url)) },
  },
  build:
    mode === "components"
      ? {
          rollupOptions: {
            input: {
              main: "index.html",
              sortableFields: "tests/components/sortable-fields.html",
              reference: "tests/components/reference.html",
              semanticFields: "tests/components/semantic-fields.html",
              components: "tests/components/harness.html",
              details: "tests/components/details.html",
              creation: "tests/components/creation.html",
              detailsComposition: "tests/components/details-composition.html",
              referenceComposition: "tests/components/reference-composition.html",
              historyComposition: "tests/components/history-composition.html",
              conversationLayout: "tests/components/conversation-layout.html",
              selectionLayoutComposition: "tests/components/selection-layout-composition.html",
              rendererRegressions: "tests/components/renderer-regressions.html",
              directForm: "tests/components/direct-form.html",
              sharedPrimitives: "tests/components/shared-primitive-identity.html",
            },
          },
        }
      : {},
}));
