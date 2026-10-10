import { runHostAuthoring } from './host-authoring.mjs';
try { await runHostAuthoring(process.argv[2], process.argv[3]); }
catch { process.exitCode = 1; console.log(JSON.stringify({ status: 'fixture-failed', artifact_admission: false })); }
