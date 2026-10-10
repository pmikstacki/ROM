import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  optimizeDeps: { exclude: ["rom-studio"] },
  resolve: { dedupe: ["svelte"] },
});
