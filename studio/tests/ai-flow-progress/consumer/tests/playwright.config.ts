import {fileURLToPath} from 'node:url';
import {defineConfig} from '@playwright/test';
if(!process.env.ROM_CHROMIUM_PATH||!process.env.ROM_WEBKIT_EXECUTABLE)throw Error('both actual browser engines required');
export default defineConfig({testDir:'.',testMatch:process.env.ROM_AI_BROWSER_DIAGNOSE==='1'?'startup.spec.ts':'progress.spec.ts',workers:1,retries:0,timeout:45000,
  use:{baseURL:`http://127.0.0.1:${process.env.ROM_AI_BROWSER_PORT}/`},
  projects:[
    {name:'chromium',use:{browserName:'chromium',launchOptions:{executablePath:process.env.ROM_CHROMIUM_PATH}}},
    {name:'webkit',use:{browserName:'webkit',launchOptions:{executablePath:process.env.ROM_WEBKIT_EXECUTABLE}}}
  ],webServer:{cwd:fileURLToPath(new URL('../',import.meta.url)),command:'node ../runtime/server.mjs',url:`http://127.0.0.1:${process.env.ROM_AI_BROWSER_PORT}/`,reuseExistingServer:false,timeout:15000}});
