import { fileURLToPath } from 'node:url';
import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: '.', testMatch: '*.spec.ts', workers: 1, timeout: 30000,
  use: { baseURL: `http://127.0.0.1:${process.env.ROM_RECOVERY_PORT}/` },
  projects: [
    { name:'chromium', use:{ browserName:'chromium', launchOptions:{executablePath:process.env.ROM_CHROMIUM_PATH ?? '/root/.nix-profile/bin/chromium'} } },
    ...(process.env.ROM_WEBKIT_EXECUTABLE ? [{name:'webkit', use:{browserName:'webkit' as const,launchOptions:{executablePath:process.env.ROM_WEBKIT_EXECUTABLE}}}] : []),
  ],
  webServer: { cwd:fileURLToPath(new URL('../',import.meta.url)), command:'node ../runtime/server.mjs', url:`http://127.0.0.1:${process.env.ROM_RECOVERY_PORT}/`, reuseExistingServer:false, timeout:30000 },
});
