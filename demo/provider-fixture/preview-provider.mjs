// Trusted service entry point for the demonstration provider. No HTTP configuration controls.
import { open } from 'node:fs/promises';
import { startHttpsProvider } from './https-provider.mjs';
async function boundedFile(path, bytes) {
  if (typeof path !== 'string' || !path.startsWith('/')) throw Error('absolute trusted file required');
  const file = await open(path, 'r');
  try {
    const info = await file.stat();
    if (!info.isFile() || info.size > bytes) throw Error('trusted file bound');
    const buffer = Buffer.alloc(bytes + 1);
    let count = 0;
    while (count < buffer.byteLength) {
      const result = await file.read(buffer, count, buffer.byteLength - count, null);
      if (!result.bytesRead) break;
      count += result.bytesRead;
    }
    if (count > bytes) throw Error('trusted file changed beyond bound');
    return buffer.subarray(0, count).toString('utf8');
  } finally { await file.close(); }
}
try {
  const profile = JSON.parse(await boundedFile(process.argv[2], 8192));
  const keys = ['issuer', 'redirect_uri', 'client_id', 'client_secret_file', 'port'];
  if (!profile || Array.isArray(profile) || Object.keys(profile).length !== keys.length || Object.keys(profile).some(k => !keys.includes(k)))
    throw Error('invalid trusted profile');
  const clientSecret = (await boundedFile(profile.client_secret_file, 4096)).trimEnd();
  const provider = await startHttpsProvider({ issuer: profile.issuer, redirectUri: profile.redirect_uri,
    clientId: profile.client_id, clientSecret, port: profile.port });
  process.stdout.write(JSON.stringify({ ready: true, issuer: provider.issuer, port: provider.port, demonstration: true }) + '\n');
  let closing = false;
  for (const signal of ['SIGTERM', 'SIGINT']) process.on(signal, () => {
    if (closing) return;
    closing = true;
    provider.close().then(() => { process.exitCode = 0; }, () => { process.exitCode = 1; });
  });
} catch {
  process.stderr.write('Demonstration provider rejected its trusted configuration\n');
  process.exitCode = 1;
}
