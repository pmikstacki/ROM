import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: ".", testMatch: "*.spec.ts", reporter: "list", workers: 1,
  use: { baseURL: "http://127.0.0.1:43281/" },
  projects: [
    ...(process.env.ROM_WEBKIT_EXECUTABLE ? [{ name: "webkit", use: { browserName: "webkit" as const, launchOptions: { executablePath: process.env.ROM_WEBKIT_EXECUTABLE } } }] : []),
    { name: "chromium", use: { browserName: "chromium", launchOptions: { executablePath: process.env.ROM_CHROMIUM_PATH ?? "/root/.nix-profile/bin/chromium" } } },
  ],
  webServer: { cwd: fileURLToPath(new URL("../", import.meta.url)), command: "corepack pnpm@10.30.0 exec vite preview --host 127.0.0.1 --port 43281 --strictPort", url: "http://127.0.0.1:43281/", reuseExistingServer: false, timeout: 20000 },
});
