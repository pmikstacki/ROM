// Controlled human interactions for the actual upstream provider fixture.
async function fields(req) {
  if (req.headers['content-type'] !== 'application/x-www-form-urlencoded') throw Error('unsupported form');
  const chunks = [];
  let size = 0;
  for await (const chunk of req) {
    size += chunk.length;
    if (size > 4096) throw Error('form limit');
    chunks.push(chunk);
  }
  const result = new URLSearchParams(Buffer.concat(chunks).toString('utf8'));
  for (const key of result.keys()) if (result.getAll(key).length !== 1) throw Error('duplicate form field');
  return result;
}

function render(res, action, prompt, accounts) {
  const control = prompt === 'login'
    ? `<label for="account">Fixture account</label><select id="account" name="account">${accounts.map(account => `<option value="${account}">${account}</option>`).join('')}</select><button type="submit">Sign in</button>`
    : '<p>Allow ROM Studio to identify this fixture account?</p><button name="consent" value="accept" type="submit">Allow</button>';
  res.writeHead(200, {
    'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store',
    'content-security-policy': "default-src 'none'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
  });
  res.end(`<!doctype html><html lang="en"><head><meta charset="utf-8"><title>ROM fixture login</title></head><body><h1>ROM human identity fixture</h1><p>Local test accounts only. This is not production authentication.</p><form method="post" action="${action}">${control}</form></body></html>`);
}

export async function interaction(provider, issuer, accounts, req, res) {
  const path = new URL(req.url, issuer).pathname;
  const details = await provider.interactionDetails(req, res);
  if (!/^[a-zA-Z0-9_-]+$/.test(details.uid)) throw Error('invalid interaction');
  const { prompt: { name, details: missing }, session, params, grantId } = details;
  if (!['login', 'consent'].includes(name)) throw Error('unsupported prompt');
  const expected = `/interaction/${details.uid}`;
  if (req.method === 'GET' && path === expected) return render(res, `${expected}/${name}`, name, accounts);
  if (req.method !== 'POST' || path !== `${expected}/${name}` || (req.headers.origin && req.headers.origin !== issuer)) throw Error('invalid interaction route');
  const body = await fields(req);
  if (name === 'login') {
    if ([...body.keys()].some(key => key !== 'account') || !accounts.includes(body.get('account'))) {
      res.writeHead(403, { 'content-type': 'text/plain', 'cache-control': 'no-store' });
      res.end('Undeclared fixture account');
      return;
    }
    await provider.interactionFinished(req, res, { login: { accountId: body.get('account') } }, { mergeWithLastSubmission: false });
    return;
  }
  if (body.get('consent') !== 'accept' || [...body.keys()].some(key => key !== 'consent')) throw Error('invalid consent');
  const grant = grantId
    ? await provider.Grant.find(grantId)
    : new provider.Grant({ accountId: session.accountId, clientId: params.client_id });
  if (!grant) throw Error('missing grant');
  if (missing.missingOIDCScope) grant.addOIDCScope(missing.missingOIDCScope.join(' '));
  if (missing.missingOIDCClaims) grant.addOIDCClaims(missing.missingOIDCClaims);
  for (const [indicator, scopes] of Object.entries(missing.missingResourceScopes ?? {})) grant.addResourceScope(indicator, scopes.join(' '));
  await provider.interactionFinished(req, res, { consent: { grantId: await grant.save() } }, { mergeWithLastSubmission: true });
}
