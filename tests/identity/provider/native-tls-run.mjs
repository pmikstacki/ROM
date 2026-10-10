import { runNativeTlsCase } from './native-tls-orchestrator.mjs';
try { const result = await runNativeTlsCase(process.argv[2], process.argv[3]); console.log(JSON.stringify({ status: result.status, release_admission: false })); }
catch { process.exitCode = 1; console.error('native TLS finite case failed'); }
