import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
export default defineConfig(({ mode }) => ({
  base: "/rom-studio/",
  plugins: [svelte(), tailwindcss()],
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
