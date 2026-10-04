import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
export default defineConfig(({ mode }) => ({
  base: "/rom-studio/",
  plugins: [
    svelte(),
    tailwindcss(),
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
              components: "tests/components/harness.html",
              details: "tests/components/details.html",
            },
          },
        }
      : {},
}));
