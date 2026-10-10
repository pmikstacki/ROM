import { runHttpsHostAuthoring } from './https-host-authoring.mjs';
try { await runHttpsHostAuthoring(process.argv[2],process.argv[3]); }
catch { process.exitCode=1;console.log(JSON.stringify({status:'https-host-fixture-prerequisite-failed',production_dependency_admission:false})); }
