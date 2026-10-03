import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: ".",
  testMatch: "*.spec.ts",
  reporter: "list",
  use: { baseURL: "http://127.0.0.1:43173/rom-studio/" },
  projects: [
    {
      name: "chromium",
      use: {
        browserName: "chromium",
        launchOptions: {
          executablePath:
            process.env.ROM_CHROMIUM_PATH ?? "/root/.nix-profile/bin/chromium",
        },
      },
    },
  ],
  webServer: {
    command: "npm run preview:components",
    url: "http://127.0.0.1:43173/rom-studio/",
    reuseExistingServer: false,
    timeout: 20000,
  },
});
