import { probeBrowserTrust } from './tls-trust-probe.mjs';
try { await probeBrowserTrust(); }
catch { process.exitCode = 1; console.log(JSON.stringify({ status: 'tls-trust-prerequisite-failed', provider_acceptance: false })); }
