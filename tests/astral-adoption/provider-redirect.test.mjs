import test from 'node:test';
import assert from 'node:assert/strict';
import { withAstralRedirect } from './provider-redirect.mjs';

const callback = 'http://127.0.0.1:43902/auth/callback/astral';
const original = [{ matching_mode: 'strict', url: 'https://fixture.invalid/original' }];
test('current provider redirect types are preserved and logout is not an authorization callback', async () => {
  const initial = [{ matching_mode: 'strict', url: callback, redirect_uri_type: 'logout' }];
  let current = structuredClone(initial), entered = false;
  const api = { async request(path, method = 'GET', body) {
    if (method === 'PATCH') current = structuredClone(body.redirect_uris);
    return { redirect_uris: structuredClone(current) };
  } };
  const evidence = [];
  await withAstralRedirect(api, callback, async () => {
    entered = true;
    assert.deepEqual(current, [...initial, { matching_mode: 'strict', url: callback, redirect_uri_type: 'authorization' }]);
  }, evidence);
  assert.equal(entered, true);
  assert.deepEqual(current, initial);
  assert.equal(evidence.at(-1).restored, true);
});
test('unknown redirect types fail before mutation', async () => {
  let calls = 0;
  const api = { async request() { calls++; return { redirect_uris: [{ matching_mode: 'strict', url: callback, redirect_uri_type: 'unknown' }] }; } };
  await assert.rejects(withAstralRedirect(api, callback, async () => assert.fail('unexpected traffic'), []), /bounded/);
  assert.equal(calls, 1);
});
function fixture({ loseAcknowledgement = false, failRestore = false,
  activationReadbackMismatch = false, malformedActivation = false,
  restorationReadbackMismatch = false, loseRestorationAcknowledgement = false,
} = {}) {
  let current = structuredClone(original), writes = 0;
  const requests = [];
  return {
    requests,
    current: () => structuredClone(current),
    async request(path, method = 'GET', body) {
      requests.push({ path, method, body: structuredClone(body) });
      assert.equal(path, '/api/v3/providers/oauth2/1/');
      if (method === 'PATCH') {
        writes++;
        if (failRestore && writes === 2) throw Error('restore unavailable');
        current = structuredClone(body.redirect_uris);
        if (loseAcknowledgement && writes === 1) throw Error('lost acknowledgement');
        if (loseRestorationAcknowledgement && writes === 2) throw Error('lost restoration acknowledgement');
        if (malformedActivation && writes === 1) return { redirect_uris: null };
      }
      if (method === 'GET' && ((activationReadbackMismatch && writes === 1) ||
          (restorationReadbackMismatch && writes === 2))) return { redirect_uris: [] };
      return { redirect_uris: structuredClone(current), client_secret: 'private-must-not-enter-evidence' };
    },
  };
}
test('exact scoped callback is confirmed and original redirects are restored', async () => {
  const api = fixture(), evidence = [];
  const result = await withAstralRedirect(api, callback, async () => {
    assert.deepEqual(api.current(), [...original, { matching_mode: 'strict', url: callback }]);
    return 42;
  }, evidence);
  assert.equal(result, 42);
  assert.deepEqual(api.current(), original);
  assert.equal(api.requests.length, 5);
  assert.equal(JSON.stringify(evidence).includes('private-must-not-enter-evidence'), false);
  assert.deepEqual(evidence.at(-1), { stage: 'redirect_restore', restored: true });
});
test('lost patch acknowledgement restores state and does not start browser traffic', async () => {
  const api = fixture({ loseAcknowledgement: true }), evidence = [];
  let started = false;
  await assert.rejects(withAstralRedirect(api, callback, async () => { started = true; }, evidence), /lost acknowledgement/);
  assert.equal(started, false);
  assert.deepEqual(api.current(), original);
  assert.equal(evidence.at(-1).restored, true);
});
test('browser failure retains its cause and restores redirects', async () => {
  const api = fixture(), evidence = [];
  const failure = Error('actual browser failure');
  await assert.rejects(withAstralRedirect(api, callback, async () => { throw failure; }, evidence), error => error === failure);
  assert.deepEqual(api.current(), original);
});
test('restoration failure refuses success and preserves prior failure', async () => {
  const api = fixture({ failRestore: true }), evidence = [];
  const failure = Error('actual browser failure');
  await assert.rejects(withAstralRedirect(api, callback, async () => { throw failure; }, evidence), error => {
    assert.ok(error instanceof AggregateError);
    assert.equal(error.errors[0], failure);
    assert.match(error.errors[1].message, /restoration/);
    return true;
  });
  assert.equal(evidence.at(-1).restored, false);
});
test('unallocated callback is rejected before provider access', async () => {
  for (const value of ['https://example.com/auth/callback/astral', callback + '?extra=1', callback.replace('43902', '43903')]) {
    const api = fixture();
    await assert.rejects(withAstralRedirect(api, value, async () => {}, []), /allocated/);
    assert.equal(api.requests.length, 0);
  }
});
test('existing exact callback needs no provider mutation', async () => {
  const requests = [], redirects = [{ matching_mode: 'strict', url: callback }];
  const api = { async request(path, method = 'GET') { requests.push(method); return { redirect_uris: structuredClone(redirects) }; } };
  assert.equal(await withAstralRedirect(api, callback, async () => 7, []), 7);
  assert.deepEqual(requests, ['GET']);
});
for (const option of ['activationReadbackMismatch', 'malformedActivation']) {
  test(`${option} prevents traffic and restores original configuration`, async () => {
    const api = fixture({ [option]: true }), evidence = [];
    let started = false;
    await assert.rejects(withAstralRedirect(api, callback, async () => { started = true; }, evidence));
    assert.equal(started, false);
    assert.deepEqual(api.current(), original);
    assert.equal(evidence.at(-1).restored, true);
  });
}
for (const option of ['restorationReadbackMismatch', 'loseRestorationAcknowledgement']) {
  test(`${option} refuses successful browser result`, async () => {
    const api = fixture({ [option]: true }), evidence = [];
    let completed = false;
    await assert.rejects(withAstralRedirect(api, callback, async () => { completed = true; return 42; }, evidence), /restoration/);
    assert.equal(completed, true);
    assert.equal(evidence.at(-1).restored, false);
  });
}
test('malformed initial configuration and capacity reject before mutation', async () => {
  for (const initial of [null, [{ matching_mode: 'strict', url: 'https://fixture.invalid', extra: true }],
    Array.from({ length: 32 }, (_, index) => ({ matching_mode: 'strict', url: `https://fixture.invalid/${index}` }))]) {
    const requests = [];
    const api = { async request(path, method = 'GET') { requests.push(method); return { redirect_uris: initial }; } };
    await assert.rejects(withAstralRedirect(api, callback, async () => { throw Error('body must not run'); }, []), /bounded|capacity/);
    assert.deepEqual(requests, ['GET']);
  }
});
test('existing regex redirects are preserved without treating them as exact approval', async () => {
  const baseline = [{ matching_mode: 'regex', url: 'http://127[.]0[.]0[.]1:43902/.*' }];
  let current = structuredClone(baseline), writes = 0;
  const api = { async request(path, method = 'GET', body) {
    if (method === 'PATCH') { writes++; current = structuredClone(body.redirect_uris); }
    return { redirect_uris: structuredClone(current) };
  } };
  await withAstralRedirect(api, callback, async () => {
    assert.deepEqual(current, [...baseline, { matching_mode: 'strict', url: callback }]);
  }, []);
  assert.equal(writes, 2);
  assert.deepEqual(current, baseline);
});

const tlsCallback = 'https://rom-astral.test:43904/auth/callback/astral';
test('allocated TLS callback is confirmed and restored without changing HTTP redirects', async () => {
  const api = fixture(), evidence = [];
  const result = await withAstralRedirect(api, tlsCallback, async () => {
    assert.deepEqual(api.current(), [...original, { matching_mode: 'strict', url: tlsCallback }]);
    return 43;
  }, evidence);
  assert.equal(result, 43);
  assert.deepEqual(api.current(), original);
  assert.deepEqual(evidence.at(-1), { stage: 'redirect_restore', restored: true });
});
test('TLS lost acknowledgement restores redirects before browser traffic', async () => {
  const api = fixture({ loseAcknowledgement: true }), evidence = [];
  let started = false;
  await assert.rejects(withAstralRedirect(api, tlsCallback, async () => { started = true; }, evidence), /lost acknowledgement/);
  assert.equal(started, false);
  assert.deepEqual(api.current(), original);
  assert.equal(evidence.at(-1).restored, true);
});
test('decorated and unallocated TLS callbacks fail before provider access', async () => {
  for (const value of [tlsCallback + '?extra=1', tlsCallback + '#fragment',
    tlsCallback.replace('43904', '43905'), tlsCallback.replace('https:', 'http:'),
    tlsCallback.replace('rom-astral.test', 'user@rom-astral.test'),
    tlsCallback.replace('/astral', '/other')]) {
    const api = fixture();
    await assert.rejects(withAstralRedirect(api, value, async () => assert.fail('unexpected traffic'), []), /allocated/);
    assert.equal(api.requests.length, 0);
  }
});
