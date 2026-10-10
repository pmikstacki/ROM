import { runHostBrowser } from './host-browser-journey.mjs';
try { await runHostBrowser(process.argv[2], process.argv[3]); }
catch { process.exitCode = 1; console.log(JSON.stringify({ status: 'fixture-failed', artifact_admission: false })); }
