// Trusted protected-preview profile. Demo accounts only; never production authentication.
import { startHumanServer } from './human-server.mjs';
function httpsUrl(value) {
  if (typeof value !== 'string' || value.length > 2048) throw Error('invalid preview URL');
  const url = new URL(value);
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash || url.href !== value)
    throw Error('preview requires an exact HTTPS URL');
  return url;
}
export async function startHttpsProvider({ issuer, redirectUri, ...options }) {
  const publicIssuer = httpsUrl(issuer);
  const redirect = httpsUrl(redirectUri);
  if (!/^\/[a-zA-Z0-9_-]+$/.test(publicIssuer.pathname)
      || !/^\/[a-zA-Z0-9_-]+\/auth\/callback\/[a-zA-Z0-9_-]+$/.test(redirect.pathname)
      || redirect.origin !== publicIssuer.origin)
    throw Error('invalid preview issuer or callback mount');
  return startHumanServer({ ...options, redirectUri: redirect.href, publicIssuer: publicIssuer.href });
}
