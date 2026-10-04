// Explicit loopback-only human OIDC fixture. This is not a production identity server.
import { startHumanServer } from './human-server.mjs';
export async function startHumanProvider(options) {
  const redirect = new URL(options.redirectUri);
  if (redirect.protocol !== 'http:' || redirect.hostname !== '127.0.0.1' || !redirect.port || redirect.username || redirect.password || redirect.search || redirect.hash)
    throw Error('human fixture requires an exact numeric-loopback callback');
  return startHumanServer({ ...options, publicIssuer: undefined });
}
