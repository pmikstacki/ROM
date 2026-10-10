import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: ".",
  testMatch: "*.spec.ts",
  reporter: "list",
  workers: 2,
  use: { baseURL: "http://127.0.0.1:43173/rom-studio/" },
  projects: [
    ...(process.env.ROM_WEBKIT_EXECUTABLE
      ? [
          {
            name: "webkit",
            use: {
              browserName: "webkit" as const,
              launchOptions: {
                executablePath: process.env.ROM_WEBKIT_EXECUTABLE,
              },
            },
          },
        ]
      : []),
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
    command: "corepack pnpm run preview:components",
    url: "http://127.0.0.1:43173/rom-studio/",
    reuseExistingServer: false,
    timeout: 20000,
  },
});
