import { serveNativeTlsProxy } from './native-tls-proxy.mjs';
try { await serveNativeTlsProxy(process.argv[2]); }
catch { process.exitCode = 1; console.error('native TLS proxy fixture failed'); }
