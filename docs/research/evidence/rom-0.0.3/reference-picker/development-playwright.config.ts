import config from "/root/ROM/studio/tests/components/playwright.config.ts";
export default {
  ...config,
  testDir: "/root/ROM/studio/tests/components",
  testMatch: "reference-picker.spec.ts",
  outputDir: "/tmp/rom-reference-dev-results",
  use: { ...config.use, baseURL: "http://127.0.0.1:43325/rom-studio/" },
  webServer: {
    command: "npx vite --host 127.0.0.1 --port 43325 --strictPort",
    cwd: "/root/ROM/studio",
    url: "http://127.0.0.1:43325/rom-studio/tests/components/reference.html",
    reuseExistingServer: false,
    timeout: 20000,
  },
};
