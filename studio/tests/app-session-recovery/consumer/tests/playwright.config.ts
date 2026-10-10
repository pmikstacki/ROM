import {fileURLToPath} from 'node:url';
import {defineConfig} from '@playwright/test';
if(!process.env.ROM_WEBKIT_EXECUTABLE)throw Error('Explicit actual WebKit 2359 path required');
export default defineConfig({testDir:'.',testMatch:'*.spec.ts',grep:process.env.ROM_APP_TEST_GREP?new RegExp(process.env.ROM_APP_TEST_GREP):undefined,workers:1,timeout:60000,
 use:{baseURL:`http://127.0.0.1:${process.env.ROM_RECOVERY_PORT}/rom-studio/`},
 projects:[{name:'chromium',use:{browserName:'chromium',launchOptions:{executablePath:process.env.ROM_CHROMIUM_PATH??'/root/.nix-profile/bin/chromium'}}},{name:'webkit',use:{browserName:'webkit',launchOptions:{executablePath:process.env.ROM_WEBKIT_EXECUTABLE}}}],
 webServer:{cwd:fileURLToPath(new URL('../',import.meta.url)),command:'node ../runtime/app/server.mjs',url:`http://127.0.0.1:${process.env.ROM_RECOVERY_PORT}/rom-studio/`,reuseExistingServer:false,timeout:30000}});
