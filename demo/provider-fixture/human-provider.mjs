// Explicit loopback-only human OIDC fixture. This is not a production identity server.
import { generateKeyPairSync, randomBytes } from 'node:crypto';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { Provider } from 'oidc-provider';
import { interaction } from './human-interaction.mjs';

export async function startHumanProvider({ clientId, clientSecret, redirectUri, port = 0, accounts = ['alice', 'bob'] }) {
  for (const value of [clientId, clientSecret]) {
    if (typeof value !== 'string' || !value.length || Buffer.byteLength(value) > 4096)
      throw Error('invalid human fixture configuration');
  }
  const redirect = new URL(redirectUri);
  if (redirect.protocol !== 'http:' || redirect.hostname !== '127.0.0.1' || !redirect.port || redirect.username || redirect.password || redirect.search || redirect.hash)
    throw Error('human fixture requires an exact numeric-loopback callback');
  if (!Number.isInteger(port) || port < 0 || port > 65535 || !Array.isArray(accounts) || !accounts.length || accounts.length > 8 || new Set(accounts).size !== accounts.length || accounts.some(value => !/^[a-z][a-z0-9_-]{1,31}$/.test(value)))
    throw Error('invalid human fixture configuration');
  const server = createServer();
  server.requestTimeout = 5000;
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
    const provider = new Provider(issuer, {
      clients: [{
        client_id: clientId, client_secret: clientSecret, grant_types: ['authorization_code'],
        response_types: ['code'], redirect_uris: [redirect.href], token_endpoint_auth_method: 'client_secret_basic',
        id_token_signed_response_alg: 'RS256',
      }],
      jwks: { keys: [{ ...privateKey.export({ format: 'jwk' }), kid: 'human-fixture', use: 'sig', alg: 'RS256' }] },
      cookies: { keys: [randomBytes(32).toString('hex'), randomBytes(32).toString('hex')] },
      scopes: ['openid', 'profile'],
      ttl: { IdToken: 300, AuthorizationCode: 60, Interaction: 120, Session: 300, Grant: 300 },
      pkce: { required: () => true },
      features: { devInteractions: { enabled: false } },
      interactions: { url: (_ctx, details) => `${issuer}/interaction/${details.uid}` },
      findAccount: async (_ctx, accountId) => accounts.includes(accountId) ? {
        accountId, claims: async () => ({ sub: accountId, name: `Fixture ${accountId}` }),
      } : undefined,
    });
    server.on('request', (req, res) => {
      if (req.url.startsWith('/interaction/')) {
        interaction(provider, issuer, accounts, req, res).catch(() => {
          if (!res.headersSent) res.writeHead(400, { 'content-type': 'text/plain', 'cache-control': 'no-store' });
          res.end('Fixture interaction rejected');
        });
      } else provider.callback()(req, res);
    });
    return { issuer, port: actualPort, close };
  } catch {
    await close().catch(() => {});
    throw Error('human provider startup rejected');
  }
}
