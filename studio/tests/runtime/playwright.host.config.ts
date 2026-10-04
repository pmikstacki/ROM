import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: ".",
  testMatch: "host.spec.ts",
  reporter: "list",
  workers: 1,
  timeout: 120000,
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
  ],
});
