import { holdNativeTlsAuthorization } from './native-tls-browser.mjs';
try { await holdNativeTlsAuthorization(process.argv[2]); }
catch { process.exitCode = 1; console.error('native TLS browser fixture failed'); }
