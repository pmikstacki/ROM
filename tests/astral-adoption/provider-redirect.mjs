// Isolated consumer fixture only. This does not configure a production provider.
import { isDeepStrictEqual } from 'node:util';

const providerPath = '/api/v3/providers/oauth2/1/';
const allocatedCallbacks = new Set([
  'http://127.0.0.1:43902/auth/callback/astral',
  'https://rom-astral.test:43904/auth/callback/astral',
]);

function redirects(response) {
  const value = response?.redirect_uris;
  if (!Array.isArray(value) || value.length > 32 || value.some(item =>
    !item || !['matching_mode,url', 'matching_mode,redirect_uri_type,url'].includes(Object.keys(item).sort().join(',')) ||
    ('redirect_uri_type' in item && !['authorization', 'logout'].includes(item.redirect_uri_type)) ||
    !['strict', 'regex'].includes(item.matching_mode) ||
    typeof item.url !== 'string' || item.url.length === 0 || item.url.length > 2048
  )) throw Error('bounded synthetic provider redirects required');
  return structuredClone(value);
}

export async function withAstralRedirect(api, callback, body, evidence) {
  if (!allocatedCallbacks.has(callback)) throw Error('allocated Astral callback required');
  if (typeof api?.request !== 'function' || typeof body !== 'function' ||
      !Array.isArray(evidence) || evidence.length > 8) {
    throw Error('bounded synthetic redirect scope required');
  }
  const original = redirects(await api.request(providerPath));
  if (original.some(item => item.matching_mode === 'strict' && item.url === callback &&
      (item.redirect_uri_type === undefined || item.redirect_uri_type === 'authorization'))) {
    evidence.push({ stage: 'redirect_existing', count: original.length });
    return body();
  }
  if (original.length === 32) throw Error('synthetic redirect capacity exhausted');
  const entry = { matching_mode: 'strict', url: callback };
  if (original.some(item => item.redirect_uri_type !== undefined)) entry.redirect_uri_type = 'authorization';
  const expected = [...original, entry];
  let attempted = false, failure;
  try {
    // An unknown PATCH outcome still requires restoration before the scope exits.
    attempted = true;
    const changed = redirects(await api.request(providerPath, 'PATCH', { redirect_uris: expected }));
    if (!isDeepStrictEqual(changed, expected) ||
        !isDeepStrictEqual(redirects(await api.request(providerPath)), expected)) {
      throw Error('synthetic callback not confirmed');
    }
    evidence.push({ stage: 'redirect_active', count: expected.length });
    return await body();
  } catch (error) {
    failure = error;
    throw error;
  } finally {
    if (attempted) {
      let restored = false;
      try {
        const changed = redirects(await api.request(providerPath, 'PATCH', { redirect_uris: original }));
        restored = isDeepStrictEqual(changed, original) &&
          isDeepStrictEqual(redirects(await api.request(providerPath)), original);
      } catch {
        restored = false;
      }
      evidence.push({ stage: 'redirect_restore', restored });
      if (!restored) {
        const restoration = Error('synthetic provider redirect restoration failed');
        if (failure) throw new AggregateError([failure, restoration], 'consumer scope and restoration failed');
        throw restoration;
      }
    }
  }
}
