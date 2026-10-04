import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
export default defineConfig({
  base: "/rom-studio/",
  plugins: [svelte(), tailwindcss()],
  resolve: {
    dedupe: ["svelte"],
    alias: [
      { find: "rom-studio/styles", replacement: fileURLToPath(new URL("../studio/src/app.css", import.meta.url)) },
      { find: "rom-studio", replacement: fileURLToPath(new URL("../studio/src/index.ts", import.meta.url)) },
      { find: "$lib", replacement: fileURLToPath(new URL("../studio/src/lib", import.meta.url)) },
    ],
  },
});
