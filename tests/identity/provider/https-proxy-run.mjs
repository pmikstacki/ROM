import { serveOwnedIdentityProxies } from './https-proxy.mjs';
try { await serveOwnedIdentityProxies(process.argv[2]); }
catch { process.exitCode = 1; console.log(JSON.stringify({ status: 'owned-identity-proxy-failed' })); }
