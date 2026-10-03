import { startProvider } from './provider.mjs';
import { readPrivate, writePrivate } from './files.mjs';

let provider;
let stopping;
async function stop() {
  if (!stopping) stopping = provider?.close();
  await stopping;
}
try {
  const [configFile, readyFile, ...extra] = process.argv.slice(2);
  if (!configFile || !readyFile || extra.length) throw new Error('invalid arguments');
  const config = JSON.parse(await readPrivate(configFile, 64 * 1024));
  const fields = new Set(['service_secret_file', 'introspection_secret_file', 'port']);
  if (!config || Array.isArray(config) || Object.keys(config).some(key => !fields.has(key)))
    throw new Error('invalid configuration');
  const serviceSecret = await readPrivate(config.service_secret_file, 4096);
  const introspectionSecret = await readPrivate(config.introspection_secret_file, 4096);
  provider = await startProvider({ serviceSecret, introspectionSecret, port: config.port ?? 0 });
  process.once('SIGTERM', () => { stop().catch(() => { process.exitCode = 1; }); });
  process.once('SIGINT', () => { stop().catch(() => { process.exitCode = 1; }); });
  await writePrivate(readyFile, JSON.stringify({
    issuer: provider.issuer, endpoint: `${provider.issuer}/token/introspection`,
    audience: 'rom-api', service_subject: 'rom-service', introspection_client: 'rom-introspector',
    provider: 'oidc-provider', version: '9.12.2', resource: provider.resource,
  }));
  console.log('Provider fixture ready.');
} catch {
  console.error('Provider fixture failed; credential details withheld.');
  process.exitCode = 1;
  await stop().catch(() => {});
}
