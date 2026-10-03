// Complete opt-in deployment acceptance against actual local provider processes.
import { mkdtemp, mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { AcceptanceFailure, HostJourney, ProviderProcess, requireThat } from './host.mjs';
import { resourceJourney, identityJourney } from './journey.mjs';

let stage = 'arguments';
let root;
const hosts = [];
const providers = [];
try {
  const [demo, cli, ...extra] = process.argv.slice(2);
  requireThat(Boolean(demo && cli && !extra.length), 'binary arguments missing');
  root = await mkdtemp(join(tmpdir(), 'rom-provider-acceptance-'));
  const expiry = [];
  for (const backend of ['sqlite', 'redb']) {
    stage = `${backend}: preparation`;
    const directory = join(root, backend);
    const providerDirectory = join(directory, 'provider');
    const hostDirectory = join(directory, 'host');
    await mkdir(providerDirectory, { recursive: true, mode: 0o700 });
    await mkdir(hostDirectory, { mode: 0o700 });
    const secrets = Object.fromEntries(['service', 'introspectionV1', 'introspectionV2'].map(key => [key, randomBytes(32).toString('hex')]));
    const provider = new ProviderProcess(providerDirectory, secrets);
    providers.push(provider);
    const metadata = await provider.initialize();
    const host = new HostJourney(hostDirectory, backend, resolve(demo), resolve(cli), metadata, secrets);
    hosts.push(host);
    stage = `${backend}: resource operations`;
    const original = await resourceJourney(host, provider);
    stage = `${backend}: identity and rotation`;
    await identityJourney(host, provider, original);
    stage = `${backend}: expiry preparation`;
    const token = await host.auth(provider.readyFile, provider.serviceFile);
    expiry.push({ host, provider, token, deadline: Date.now() + 62000 });
    console.log(`Provider acceptance (${backend}): operations, receipt replay, ROM reopen, access revocation, audience binding and credential rotation passed.`);
  }
  // Real provider expiration uses wall time; no response rewriting or synthetic clock.
  for (const { host, provider, token, deadline } of expiry) {
    stage = `${host.backend}: actual token expiry`;
    await delay(Math.max(0, deadline - Date.now()));
    await host.start();
    await host.request(token, ['read', 'tasks', 'provider-task'], 3);
    const fresh = await host.auth(provider.readyFile, provider.serviceFile);
    await host.request(fresh, ['read', 'tasks', 'provider-task']);
    await host.stopHost();
    host.assertRedacted();
    provider.assertRedacted(host.privateValues);
    await host.scanDatabase();
    console.log(`Provider acceptance (${host.backend}): expired token denied, fresh token accepted and output/database marker scans passed.`);
  }
} catch (error) {
  const reason = error instanceof AcceptanceFailure ? error.message : 'operation failed';
  console.error(`Provider acceptance failed at ${stage}: ${reason}; captured credential data withheld.`);
  process.exitCode = 1;
} finally {
  let cleanupFailed = false;
  for (const host of hosts) await host.finish().catch(() => { cleanupFailed = true; });
  for (const provider of providers) await provider.finish().catch(() => { cleanupFailed = true; });
  if (root) await rm(root, { recursive: true, force: true }).catch(() => { cleanupFailed = true; });
  if (cleanupFailed) {
    console.error('Provider acceptance cleanup failed; details withheld.');
    process.exitCode = 1;
  }
}
