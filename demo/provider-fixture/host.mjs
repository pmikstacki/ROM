// Actual-process acceptance driver. Captured data stays private; failures use static labels.
import { readFile, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { bounded, startProcess, startNode, ready, stop, stopAll } from './processes.mjs';
import { writePrivate } from './files.mjs';

export class AcceptanceFailure extends Error {}

export function requireThat(condition, label) {
  if (!condition) throw new AcceptanceFailure(label);
}

export class HostJourney {
  constructor(root, backend, demo, cli, provider, secrets) {
    Object.assign(this, { root, backend, demo, cli, provider, secrets });
    this.children = [];
    this.privateValues = [...Object.values(secrets)];
    this.privatePaths = [root, join(root, 'introspection-v1'), join(root, 'introspection-v2')];
    this.sequence = 0;
    this.database = join(root, `${backend}.db`);
    this.configFile = join(root, 'host.json');
    this.config = {
      version: 1,
      provider: { authority: 'fixture-provider', issuer: provider.issuer, audience: 'rom-api',
        endpoint: provider.endpoint, introspection_client: provider.introspection_client, credential_ref: 'secret-v1' },
      user: { id: 'service-user', display_name: 'Fixture service' },
      service_subject: 'rom-service',
      secrets: { 'secret-v1': 'introspection-v1', 'secret-v2': 'introspection-v2' },
      endpoint_policy: 'loopback-test-only', auth: { jobs: 4, response_timeout_ms: 2000 },
    };
  }
  async initialize() {
    await writePrivate(join(this.root, 'introspection-v1'), this.secrets.introspectionV1);
    await writePrivate(join(this.root, 'introspection-v2'), this.secrets.introspectionV2);
    await writePrivate(this.configFile, JSON.stringify(this.config));
  }
  async input(value) {
    const path = join(this.root, `input-${++this.sequence}.json`);
    await writePrivate(path, JSON.stringify(value));
    return path;
  }
  launch(executable, args) {
    const child = startProcess(executable, args);
    this.children.push(child);
    return child;
  }
  async command(executable, args, expected = 0) {
    const child = this.launch(executable, args);
    const [code, signal] = await bounded(child.done, 7000);
    requireThat(code === expected && signal === null && !child.exceeded(),
      `unexpected process result (expected ${expected}, received ${code}, signal ${signal})`);
    if (expected !== 0) return child;
    let value;
    try { value = JSON.parse(child.stdout()); } catch { throw new Error('invalid command JSON'); }
    return value;
  }
  async provision(config = this.configFile, expected = 0) {
    return this.command(this.demo, ['provider-provision', this.backend, this.database, config], expected);
  }
  async start() {
    requireThat(!this.host, 'host already running');
    this.host = this.launch(this.demo, ['provider-serve', this.backend, this.database, this.configFile, '0']);
    for (let attempt = 0; attempt < 500; attempt++) {
      const line = this.host.stdout().split('\n')[0];
      try {
        const value = JSON.parse(line);
        const endpoint = new URL(value.endpoint);
        requireThat(endpoint.protocol === 'http:' && endpoint.hostname === '127.0.0.1', 'non-loopback host readiness');
        this.endpoint = value.endpoint;
        return;
      } catch { /* Await complete readiness JSON. */ }
      requireThat(this.host.child.exitCode === null && this.host.child.signalCode === null, 'host exited before readiness');
      await delay(10);
    }
    throw new Error('host readiness deadline');
  }
  async stopHost() {
    if (!this.host) return;
    // ROM's normal stop path listens for Ctrl-C. A successful test must exit gracefully.
    this.host.child.kill('SIGINT');
    const [code, signal] = await bounded(this.host.done, 7000);
    requireThat(code === 0 && signal === null, 'host graceful shutdown failed');
    this.host = undefined;
  }
  async auth(readyFile, serviceFile) {
    const path = join(this.root, `auth-${++this.sequence}`);
    this.privatePaths.push(path);
    const child = startNode('issue.mjs', [readyFile, serviceFile, path]);
    this.children.push(child);
    const [code, signal] = await bounded(child.done);
    requireThat(code === 0 && signal === null, 'token acquisition failed');
    const header = await readFile(path, 'utf8');
    this.privateValues.push(header, header.slice(7));
    return path;
  }
  async request(auth, args, expected = 0) {
    requireThat(Boolean(this.host), 'host not running');
    return this.command(this.cli, ['--endpoint', this.endpoint, '--auth-file', auth, '--output', 'json', ...args], expected);
  }
  async maintain(kind, id, expected, patch) {
    requireThat(!this.host, 'offline maintenance requires stopped host');
    const input = await this.input({ kind, id, expected, idempotency: `maintenance-${this.sequence}`,
      operation: { type: 'patch', input: Object.fromEntries(Object.entries(patch).map(([key, value]) => [key, { op: 'set', value }])) } });
    const result = await this.command(this.demo, ['provider-maintain', this.backend, this.database, this.configFile, input]);
    requireThat(result.revision === expected + 1, 'maintenance revision mismatch');
    return result.revision;
  }
  assertRedacted() {
    for (const child of this.children)
      for (const secret of [...this.privateValues, ...this.privatePaths])
        requireThat(!child.output().includes(secret), 'credential leaked into child output');
  }
  async finish() {
    // Teardown also owns children after a failed readiness or acceptance assertion.
    await stopAll(this.children);
  }
  async scanDatabase() {
    const bytes = await readFile(this.database);
    for (const secret of [...this.privateValues, ...this.privatePaths])
      requireThat(!bytes.includes(Buffer.from(secret)), 'credential leaked into native database bytes');
  }
}

export class ProviderProcess {
  constructor(root, secrets) {
    Object.assign(this, { root, secrets });
    this.readyFile = join(root, 'provider-ready.json');
    this.configFile = join(root, 'provider.json');
    this.serviceFile = join(root, 'service-secret');
    this.secretFile = join(root, 'provider-introspection');
    this.children = [];
  }
  async initialize() {
    await writePrivate(this.serviceFile, this.secrets.service);
    await writePrivate(this.secretFile, this.secrets.introspectionV1);
    await writePrivate(this.configFile, JSON.stringify({ service_secret_file: this.serviceFile, introspection_secret_file: this.secretFile }));
    return this.start();
  }
  async start() {
    this.process = startNode('server.mjs', [this.configFile, this.readyFile]);
    this.children.push(this.process);
    this.metadata = await ready(this.readyFile, this.process);
    return this.metadata;
  }
  async rotate() {
    await stop(this.process);
    await rm(this.readyFile);
    await writeFile(this.secretFile, this.secrets.introspectionV2);
    await writeFile(this.configFile, JSON.stringify({ service_secret_file: this.serviceFile,
      introspection_secret_file: this.secretFile, port: Number(new URL(this.metadata.issuer).port) }));
    return this.start();
  }
  async finish() {
    await stopAll(this.children);
  }
  assertRedacted(tokens) {
    for (const child of this.children)
      for (const secret of [...Object.values(this.secrets), ...tokens, this.root, this.serviceFile, this.secretFile])
        requireThat(!child.output().includes(secret), 'provider leaked credentials');
  }
}
