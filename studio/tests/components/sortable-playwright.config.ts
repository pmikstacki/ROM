import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: ".",
  testMatch: "sortable-fields.spec.ts",
  reporter: "list",
  workers: 1,
  use: { baseURL: "http://127.0.0.1:43324/rom-studio/" },
  projects: [
    {
      name: "chromium",
      use: {
        browserName: "chromium",
        launchOptions: { executablePath: "/root/.nix-profile/bin/chromium" },
      },
    },
  ],
  webServer: {
    cwd: fileURLToPath(new URL("../..", import.meta.url)),
    command: "npx vite --host 127.0.0.1 --port 43324 --strictPort",
    url: "http://127.0.0.1:43324/rom-studio/",
    reuseExistingServer: false,
  },
});
