// Both native backends execute this same CLI journey; no response is fabricated.
import { join } from 'node:path';
import { writeFile } from 'node:fs/promises';
import { writePrivate } from './files.mjs';
import { requireThat } from './host.mjs';

function sameReceipt(actual, accepted) {
  requireThat(JSON.stringify(actual) === JSON.stringify(accepted), 'receipt replay mismatch');
}

function taskAt(row, revision, done) {
  requireThat(row?.key?.kind === 'tasks' && row.key.id === 'provider-task'
    && row.revision === revision && row.value?.done === done
    && row.value.title === 'Provider acceptance', 'task result mismatch');
}

async function assertHistory(host, auth) {
  const rows = await host.request(auth, ['query', 'tasks']);
  requireThat(Array.isArray(rows) && rows.length === 1, 'query result count mismatch');
  taskAt(rows[0], 2, true);
  const journal = await host.request(auth, ['journal', 'tasks']);
  requireThat(journal.events?.length === 2 && journal.cursor?.kind === 'tasks', 'journal event count mismatch');
  taskAt(journal.events[0].view, 1, false);
  taskAt(journal.events[1].view, 2, true);
  requireThat(journal.events[0].position < journal.events[1].position
    && journal.events[1].position <= journal.cursor.position, 'journal order mismatch');
}

export async function resourceJourney(host, provider) {
  await host.initialize();
  await host.provision();
  await host.provision();
  const changed = await host.input({ ...host.config, user: { ...host.config.user, display_name: 'Changed input' } });
  await host.provision(changed, 1);
  const auth = await host.auth(provider.readyFile, provider.serviceFile);
  await host.start();
  const discovery = await host.request(auth, ['discover']);
  requireThat(JSON.stringify(discovery).includes('tasks'), 'authorized task descriptor missing');
  const input = await host.input({ title: 'Provider acceptance', done: false });
  const create = ['create', 'tasks', 'provider-task', '--idempotency', 'provider-create', '--input-file', input];
  const created = await host.request(auth, create);
  taskAt(created, 1, false);
  const replay = await host.request(auth, create);
  sameReceipt(replay, created);
  const complete = await host.input(null);
  const completed = await host.request(auth, ['action', 'tasks', 'provider-task', 'complete', '--expected', '1', '--idempotency', 'provider-complete', '--input-file', complete]);
  taskAt(completed, 2, true);
  await assertHistory(host, auth);
  const capabilities = await host.request(auth, ['work', 'capabilities']);
  requireThat(capabilities.inspect === false && capabilities.retry === false
    && capabilities.reconcile === false, 'unexpected operator capability grant');
  await host.request(auth, ['work', 'list'], 3);
  const fake = join(host.root, 'synthetic-auth');
  await writePrivate(fake, 'Demo local');
  await host.request(fake, ['read', 'tasks', 'provider-task'], 3);
  await host.stopHost();
  await host.start();
  const reopened = await host.request(auth, ['read', 'tasks', 'provider-task']);
  taskAt(reopened, 2, true);
  sameReceipt(await host.request(auth, create), created);
  await assertHistory(host, auth);
  await host.stopHost();
  return { auth, create, created };
}

export async function identityJourney(host, provider, previous) {
  const providerId = host.config.provider.authority;
  const link = JSON.stringify([providerId, 'service', host.config.service_subject]);
  let linkRevision = await host.maintain('identity-links', link, 1, { enabled: false });
  await host.start();
  await host.request(previous.auth, ['read', 'tasks', 'provider-task'], 3);
  await host.request(previous.auth, previous.create, 5);
  await host.stopHost();
  linkRevision = await host.maintain('identity-links', link, linkRevision, { enabled: true });
  requireThat(linkRevision === 3, 'link revision mismatch');
  await host.start();
  sameReceipt(await host.request(previous.auth, previous.create), previous.created);
  await host.stopHost();

  let revision = await host.maintain('identity-providers', providerId, 1, { enabled: false });
  await host.start();
  await host.request(previous.auth, ['read', 'tasks', 'provider-task'], 3);
  await host.stopHost();
  revision = await host.maintain('identity-providers', providerId, revision, { enabled: true });
  let userRevision = await host.maintain('users', host.config.user.id, 1, { enabled: false });
  await host.start();
  await host.request(previous.auth, ['read', 'tasks', 'provider-task'], 3);
  await host.stopHost();
  await host.maintain('users', host.config.user.id, userRevision, { enabled: true });

  // The unchanged token is bound to rom-api; a host/Resource agreeing on another audience must still deny it.
  revision = await host.maintain('identity-providers', providerId, revision, { audience: 'other-api' });
  const originalAudience = host.config.provider.audience;
  host.config.provider.audience = 'other-api';
  await writeFile(host.configFile, JSON.stringify(host.config));
  await host.start();
  await host.request(previous.auth, ['read', 'tasks', 'provider-task'], 3);
  await host.stopHost();
  revision = await host.maintain('identity-providers', providerId, revision, { audience: originalAudience });
  host.config.provider.audience = originalAudience;
  await writeFile(host.configFile, JSON.stringify(host.config));

  const beforeRestart = await host.auth(provider.readyFile, provider.serviceFile);
  const restartDeadline = Date.now() + 50000;
  await host.start();
  taskAt(await host.request(beforeRestart, ['read', 'tasks', 'provider-task']), 2, true);
  await host.stopHost();
  const issuer = provider.metadata.issuer;
  await provider.rotate();
  requireThat(provider.metadata.issuer === issuer, 'provider restart issuer changed');
  const fresh = await host.auth(provider.readyFile, provider.serviceFile);
  await host.start();
  // Fresh token with the obsolete introspection credential must fail.
  await host.request(fresh, ['read', 'tasks', 'provider-task'], 3);
  await host.stopHost();
  revision = await host.maintain('identity-providers', providerId, revision, { credential_ref: 'secret-v2' });
  await host.start();
  await host.request(fresh, ['read', 'tasks', 'provider-task']);
  // The provider lost the old opaque token. ROM does not invent a replacement.
  await host.request(beforeRestart, ['read', 'tasks', 'provider-task'], 3);
  requireThat(Date.now() < restartDeadline, 'provider restart test exceeded token validity margin');
  await host.stopHost();
  revision = await host.maintain('identity-providers', providerId, revision, { credential_ref: 'unapproved-reference' });
  await host.start();
  await host.request(fresh, ['read', 'tasks', 'provider-task'], 3);
  await host.stopHost();
  await host.maintain('identity-providers', providerId, revision, { credential_ref: 'secret-v2' });
  await host.start();
  sameReceipt(await host.request(fresh, previous.create), previous.created);
  await assertHistory(host, fresh);
  await host.stopHost();
  host.assertRedacted();
  provider.assertRedacted(host.privateValues);
  await host.scanDatabase();
  return fresh;
}
