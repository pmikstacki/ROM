// Development-only OAuth server. No token or introspection response rewriting.
import { generateKeyPairSync } from 'node:crypto';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { Provider, errors } from 'oidc-provider';

export const resource = 'https://rom.fixture.invalid/api';
const responseLimit = 16 * 1024;

export async function startProvider({ serviceSecret, introspectionSecret, port = 0 }) {
  for (const secret of [serviceSecret, introspectionSecret]) {
    if (typeof secret !== 'string' || !secret.length || Buffer.byteLength(secret) > 4096)
      throw new Error('invalid fixture configuration');
  }
  if (!Number.isInteger(port) || port < 0 || port > 65535)
    throw new Error('invalid fixture configuration');
  const server = createServer();
  let closing;
  const close = () => closing ??= new Promise((resolve, reject) => {
    server.closeAllConnections();
    server.close(error => error && error.code !== 'ERR_SERVER_NOT_RUNNING' ? reject(error) : resolve());
  });
  try {
    server.listen(port, '127.0.0.1');
    await once(server, 'listening');
    const actualPort = server.address().port;
    const issuer = `http://127.0.0.1:${actualPort}`;
    const { privateKey } = generateKeyPairSync('rsa', { modulusLength: 2048 });
    const client = (id, secret, grants) => ({
      client_id: id, client_secret: secret, grant_types: grants,
      response_types: [], redirect_uris: [], token_endpoint_auth_method: 'client_secret_basic',
    });
    const provider = new Provider(issuer, {
      scopes: ['rom'],
      ttl: { ClientCredentials: 60 },
      jwks: { keys: [{ ...privateKey.export({ format: 'jwk' }), kid: 'fixture', use: 'sig', alg: 'RS256' }] },
      clients: [
        { ...client('rom-service', serviceSecret, ['client_credentials']), scope: 'rom' },
        client('rom-introspector', introspectionSecret, []),
      ],
      features: {
        devInteractions: { enabled: false },
        clientCredentials: { enabled: true },
        introspection: {
          enabled: true,
          allowedPolicy: (_ctx, caller, token) => caller.clientId === 'rom-introspector' && token.clientId === 'rom-service',
        },
        resourceIndicators: {
          enabled: true,
          getResourceServerInfo: (_ctx, requested, caller) => {
            if (requested !== resource || caller.clientId !== 'rom-service') throw new errors.InvalidTarget();
            return { scope: 'rom', audience: 'rom-api', accessTokenTTL: 60, accessTokenFormat: 'opaque' };
          },
        },
      },
      extraTokenClaims: (_ctx, token) => {
        if (token.kind !== 'ClientCredentials' || token.clientId !== 'rom-service') throw new errors.InvalidGrant();
        return { sub: token.clientId, principal_kind: 'service' };
      },
    });
    // Bind the actual ephemeral port before constructing the issuer. No release/rebind gap.
    server.on('request', provider.callback());
    return { issuer, port: actualPort, resource, close };
  } catch {
    await close().catch(() => {});
    throw new Error('provider startup rejected');
  }
}

async function post(issuer, route, clientId, secret, fields) {
  const parsed = new URL(issuer);
  if (parsed.protocol !== 'http:' || parsed.hostname !== '127.0.0.1' || parsed.username || parsed.password || parsed.pathname !== '/' || parsed.search || parsed.hash)
    throw new Error('fixture requires numeric loopback');
  const response = await fetch(`${parsed.origin}${route}`, {
    method: 'POST', redirect: 'error', signal: AbortSignal.timeout(3000),
    // The fixture deliberately restarts the issuer on the same port. Do not reuse a dead connection.
    headers: { connection: 'close', authorization: `Basic ${Buffer.from(`${clientId}:${secret}`).toString('base64')}` },
    body: new URLSearchParams(fields),
  });
  if (response.status !== 200) {
    await response.body?.cancel();
    throw new Error('provider request rejected');
  }
  const chunks = [];
  let bytes = 0;
  for await (const chunk of response.body) {
    bytes += chunk.length;
    if (bytes > responseLimit) throw new Error('provider response too large');
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}

export async function issueToken(issuer, serviceSecret) {
  const value = await post(issuer, '/token', 'rom-service', serviceSecret, {
    grant_type: 'client_credentials', scope: 'rom', resource,
  });
  if (value.token_type !== 'Bearer' || typeof value.access_token !== 'string' || !value.access_token.length || Buffer.byteLength(value.access_token) > 4096)
    throw new Error('provider token rejected');
  return value.access_token;
}

export function introspect(issuer, introspectionSecret, token) {
  return post(issuer, '/token/introspection', 'rom-introspector', introspectionSecret, {
    token, token_type_hint: 'access_token',
  });
}
