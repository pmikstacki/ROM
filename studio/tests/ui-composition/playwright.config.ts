import { defineConfig } from "@playwright/test";
const port = Number(process.env.ROM_UI_PORT ?? 43301);
if (!process.env.ROM_WEBKIT_EXECUTABLE)
  throw Error("actual WebKit executable required");
export default defineConfig({
  testDir: "./tests",
  testMatch: "*.spec.ts",
  workers: 1,
  retries: 0,
  timeout: 20000,
  use: {
    baseURL: `http://127.0.0.1:${port}/`,
    viewport: { width: 1280, height: 900 },
  },
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
    {
      name: "webkit",
      use: {
        browserName: "webkit",
        launchOptions: { executablePath: process.env.ROM_WEBKIT_EXECUTABLE },
      },
    },
  ],
  webServer: {
    cwd: "..",
    command: "node http.mjs",
    url: `http://127.0.0.1:${port}/`,
    reuseExistingServer: false,
    timeout: 20000,
  },
});
