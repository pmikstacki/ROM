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
              rendererRegressions: "tests/components/renderer-regressions.html",
              directForm: "tests/components/direct-form.html",
            },
          },
        }
      : {},
}));
