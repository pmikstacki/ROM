import { probePrivateNss } from './nss-private-trust.mjs';
try { await probePrivateNss(); }
catch { process.exitCode = 1; console.log(JSON.stringify({ status: 'private-nss-prerequisite-failed', tls_admission: false })); }
