// Anonymous public registry metadata only. Never persist registry bearer responses.
import { createHash } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const digestPattern = /^sha256:[a-f0-9]{64}$/;
const registries = {
  authentik: { origin: 'https://ghcr.io', repository: 'goauthentik/server', realm: 'https://ghcr.io/token', service: 'ghcr.io' },
  postgres: { origin: 'https://registry-1.docker.io', repository: 'library/postgres', realm: 'https://auth.docker.io/token', service: 'registry.docker.io' },
};
const accept = 'application/vnd.oci.image.index.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.v2+json';

export function verifyDigest(bytes, digest) {
  if (!Buffer.isBuffer(bytes) || bytes.length > 16 * 1024 * 1024 || !digestPattern.test(digest) || `sha256:${createHash('sha256').update(bytes).digest('hex')}` !== digest) throw Error('image digest mismatch');
}
export function selectPlatform(index, platform) {
  if (!Array.isArray(index.manifests) || index.manifests.length > 32) throw Error('image platform index invalid');
  const matches = index.manifests.filter(entry => `${entry.platform?.os}/${entry.platform?.architecture}` === platform);
  if (matches.length !== 1 || !digestPattern.test(matches[0].digest)) throw Error('image platform missing or ambiguous');
  return matches[0].digest;
}
export function requireImageManifest(manifest) {
  if (!digestPattern.test(manifest.config?.digest) || !Array.isArray(manifest.layers) || !manifest.layers.length || manifest.layers.length > 128) throw Error('image manifest invalid');
  let bytes = 0;
  for (const layer of manifest.layers) {
    if (!digestPattern.test(layer.digest) || !Number.isSafeInteger(layer.size) || layer.size < 0 || layer.urls !== undefined) throw Error('image manifest layer invalid');
    bytes += layer.size;
  }
  if (bytes > 6 * 1024 ** 3) throw Error('image manifest acquisition limit');
  return bytes;
}

async function body(response) {
  if (!response.ok) throw Error(`public metadata HTTP ${response.status}`);
  const chunks = []; let count = 0;
  for await (const chunk of response.body) {
    count += chunk.length; if (count > 16 * 1024 * 1024) throw Error('public metadata limit'); chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}
const request = (url, headers = {}) => fetch(url, { headers, redirect: 'error', signal: AbortSignal.timeout(20_000) });

const blobOrigins = { authentik: 'https://pkg-containers.githubusercontent.com', postgres: 'https://production.cloudfront.docker.com' };
export async function fetchPublicBlob(role, url, headers, send = fetch) {
  const response = await send(url, { headers, redirect: 'manual', signal: AbortSignal.timeout(20_000) });
  if (![301, 302, 303, 307, 308].includes(response.status)) return body(response);
  let target;
  try { target = new URL(response.headers.get('location')); } catch { throw Error('public blob redirect invalid'); }
  if (target.origin !== blobOrigins[role] || target.username || target.password || target.hash) throw Error('public blob redirect unapproved');
  // A signed CDN URL is ephemeral. Do not persist it or forward registry credentials.
  await response.body?.cancel();
  const redirected = await send(target, { headers: {}, redirect: 'manual', signal: AbortSignal.timeout(20_000) });
  if ([301, 302, 303, 307, 308].includes(redirected.status)) throw Error('public blob redirect repeated');
  return body(redirected);
}

export async function resolvePublicImage(role, requestedDigest, output) {
  const registry = registries[role];
  if (!registry || !digestPattern.test(requestedDigest)) throw Error('unapproved public image');
  const base = `${registry.origin}/v2/${registry.repository}`; let bearer;
  async function get(path) {
    const blob = path.startsWith('blobs/');
    const send = headers => blob ? fetchPublicBlob(role, `${base}/${path}`, headers) : request(`${base}/${path}`, headers);
    // Manifests establish anonymous registry authorization before blob acquisition.
    if (blob) return send({ ...(bearer ? { Authorization: `Bearer ${bearer}` } : {}) });
    let response = await send({ Accept: accept, ...(bearer ? { Authorization: `Bearer ${bearer}` } : {}) });
    if (response.status === 401 && !bearer) {
      const challenge = response.headers.get('www-authenticate') ?? '';
      const realm = /realm="([^"]+)"/.exec(challenge)?.[1];
      if (realm !== registry.realm) throw Error('unapproved registry authentication source');
      const url = new URL(registry.realm); url.searchParams.set('service', registry.service); url.searchParams.set('scope', `repository:${registry.repository}:pull`);
      const privateResponse = JSON.parse((await body(await request(url))).toString());
      bearer = privateResponse.token ?? privateResponse.access_token;
      if (typeof bearer !== 'string' || !bearer.length || bearer.length > 16_384) throw Error('invalid anonymous registry response');
      response = await request(`${base}/${path}`, { Accept: accept, Authorization: `Bearer ${bearer}` });
    }
    return body(response);
  }
  const indexBytes = await get(`manifests/${requestedDigest}`); verifyDigest(indexBytes, requestedDigest);
  const index = JSON.parse(indexBytes); const platform = 'linux/amd64';
  const platformDigest = Array.isArray(index.manifests) ? selectPlatform(index, platform) : requestedDigest;
  const bytes = platformDigest === requestedDigest ? indexBytes : await get(`manifests/${platformDigest}`); verifyDigest(bytes, platformDigest);
  const manifest = JSON.parse(bytes); const compressedBytes = requireImageManifest(manifest);
  const configBytes = await get(`blobs/${manifest.config.digest}`); verifyDigest(configBytes, manifest.config.digest);
  const config = JSON.parse(configBytes);
  if (`${config.os}/${config.architecture}` !== platform) throw Error('image config platform mismatch');
  const labels = config.config?.Labels ?? {};
  const publicLabels = Object.fromEntries(['org.opencontainers.image.source', 'org.opencontainers.image.revision', 'org.opencontainers.image.version', 'org.opencontainers.image.licenses'].filter(key => typeof labels[key] === 'string').map(key => [key, labels[key]]));
  const result = { role, requested_digest: requestedDigest, platform, platform_digest: platformDigest, config_digest: manifest.config.digest,
    compressed_bytes: compressedBytes, labels: publicLabels, registry: registry.origin, repository: registry.repository, layers: manifest.layers.map(({ digest, size }) => ({ digest, size })) };
  // Config Env is neither copied nor printed. Only public manifest bytes and selected labels are retained.
  writeFileSync(join(output, `${role}-index.json`), indexBytes, { flag: 'wx', mode: 0o600 });
  writeFileSync(join(output, `${role}-manifest.json`), bytes, { flag: 'wx', mode: 0o600 });
  writeFileSync(join(output, `${role}-identity.json`), JSON.stringify(result, null, 2)+'\n', { flag: 'wx', mode: 0o600 });
  return result;
}
