import { acquireNss } from './nss-acquisition.mjs';
try { await acquireNss(); }
catch { process.exitCode = 1; console.log(JSON.stringify({ status: 'tool-prerequisite-failed', tls_admission: false })); }
