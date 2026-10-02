import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
const dir = await mkdtemp(join(tmpdir(), 'rom-resource-flow-'));
const binary = resolve(process.env.CARGO_TARGET_DIR ?? 'target', 'debug/resource-flow-prototype');
let server, base, sequence = 0;
const sleep = ms => new Promise(r => setTimeout(r, ms));
async function start(extra = {}) {
  server = spawn(binary, [], { env: { ...process.env, PORT: '0', PROTOTYPE_DB: join(dir, 'PROTOTYPE-wipe-me.sqlite'), PROTOTYPE_FAULTS: '1', ...extra }, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  server.stderr.on('data', c => process.stderr.write(c));
  base = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('server start timeout')), 10000);
    server.stdout.on('data', chunk => { output += chunk; const m = output.match(/LISTENING (http:\/\/127.0.0.1:\d+)/); if (m) { clearTimeout(timer); resolve(m[1]); } });
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`server exited ${code}: ${output}`)); });
  });
}
async function crashAfterReactionCommit() {
  const child = spawn(binary, [], { env: { ...process.env, PORT: '0', PROTOTYPE_DB: join(dir, 'PROTOTYPE-wipe-me.sqlite'), PROTOTYPE_FAULTS: '1', PROTOTYPE_REACTION_CRASH_AFTER_COMMIT: '1' }, stdio: ['ignore','ignore','pipe'] });
  child.stderr.on('data', c => process.stderr.write(c));
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => { child.kill('SIGKILL'); reject(new Error('reaction commit crash not reached')); }, 4000);
    child.once('exit', code => { clearTimeout(timer); if (code === 86) resolve(); else reject(new Error('unexpected fault process exit ' + code)); });
  });
}
async function stop() {
  if (!server || server.exitCode !== null || server.signalCode !== null) return;
  const done = new Promise(r => server.once('exit', r));
  server.kill('SIGKILL'); await done;
}
async function request(method, path, body, status = 200, principal = 'alice', headers = {}) {
  const response = await fetch(base + path, { method, headers: { 'content-type': 'application/json', 'x-principal': principal, ...headers }, body: body === undefined ? undefined : JSON.stringify(body) });
  const text = await response.text();
  assert.equal(response.status, status, `${method} ${path}: ${text}`);
  return text ? JSON.parse(text) : null;
}
const action = (input, expected_revision = 0, action_id = `probe-${++sequence}`) => ({ input, expected_revision, action_id });
async function stream(after = 0, principal = 'alice', header = false) {
  const controller = new AbortController();
  const response = await fetch(base + `/events${header ? '' : `?after=${after}`}`, { headers: { 'x-principal': principal, ...(header ? { 'last-event-id': String(after) } : {}) }, signal: controller.signal });
  assert.equal(response.status, 200);
  const reader = response.body.getReader(); const decoder = new TextDecoder(); let text = '';
  return { close: () => controller.abort(), async next() {
    return Promise.race([ (async () => { for (;;) {
      let end; while ((end = text.indexOf('\n\n')) >= 0) { const packet = text.slice(0, end); text = text.slice(end + 2); const data = packet.split('\n').find(l => l.startsWith('data:')); if (data) return JSON.parse(data.slice(5)); }
      const part = await reader.read(); if (part.done) throw new Error('SSE ended'); text += decoder.decode(part.value, { stream: true });
    } })(), sleep(4000).then(() => { throw new Error('SSE timeout'); }) ]);
  } };
}
let passed = 0;
async function probe(name, f) { await f(); passed++; console.log(`PASS ${name}`); }
try {
  await start({ PROTOTYPE_REACTIONS: '0' });
  let thermostat, task, sensor, tail = 0;
  await probe('one declaration registry, three unrelated kinds, generic HTTP CRUD', async () => {
    const descriptors = await request('GET', '/schema');
    assert.deepEqual(descriptors.map(d => d.kind).sort(), ['sensor', 'task', 'thermostat']);
    const fields = descriptors.find(d => d.kind === 'thermostat').fields;
    assert.equal(fields.find(f => f.name === 'note').nullable, true);
    assert.equal(fields.find(f => f.name === 'enabled').nullable, false);
    thermostat = await request('POST', '/resources/thermostat/room', action({ enabled: true, target: 21, note: 'keep' }));
    task = await request('POST', '/resources/task/manual', action({ title: 'Water plants', done: false, note: null }));
    sensor = await request('POST', '/resources/sensor/roof', action({ code: 'ROOF', reading: 12.5 }));
    for (const [kind, id, created] of [['thermostat','room',thermostat], ['task','manual',task], ['sensor','roof',sensor]]) {
      const read = await request('GET', `/resources/${kind}/${id}`); assert.deepEqual(read, created.resource);
      assert.equal((await request('GET', `/resources/${kind}`)).length, 1);
      assert.equal(created.resource.revision, 1); assert.ok(created.event_id > 0);
    }
    tail = sensor.event_id;
  });
  await probe('custom field extension rejects invalid value without per-kind handlers', async () => {
    await request('POST', '/resources/sensor/bad', action({ code: 'lower', reading: 3 }), 422);
    await request('POST', '/resources/sensor/unknown', action({ code: 'ABC', reading: 3, extra: true }), 422);
    assert.equal((await request('GET', '/resources/sensor')).length, 1);
  });
  await probe('false, zero, omitted and explicit null have distinct patch behavior', async () => {
    const changed = await request('PATCH', '/resources/thermostat/room', action({ enabled: false, target: 0 }, 1));
    assert.equal(changed.resource.data.target, 0); assert.equal(changed.resource.data.enabled, false); assert.equal(changed.resource.data.note, 'keep');
    const nulled = await request('PATCH', '/resources/thermostat/room', action({ note: null }, 2));
    assert.equal(nulled.resource.data.note, null); assert.equal(nulled.resource.data.enabled, false);
    await request('PATCH', '/resources/thermostat/room', action({ enabled: null }, 3), 422);
    thermostat = nulled; tail = nulled.event_id;
  });
  await probe('no-op records outcome without revision/event; same ID retries exactly', async () => {
    const body = action({ enabled: false }, 3, 'stable-noop');
    const first = await request('PATCH', '/resources/thermostat/room', body);
    assert.equal(first.changed, false); assert.equal(first.event_id, null); assert.equal(first.resource.revision, 3);
    assert.deepEqual(await request('PATCH', '/resources/thermostat/room', body), first);
    await request('PATCH', '/resources/thermostat/room', { ...body, input: { enabled: true } }, 409);
  });
  await probe('named declared action uses generic transaction and live SSE', async () => {
    const feed = await stream(tail);
    try {
      const up = await request('PATCH', '/resources/thermostat/room', action({ enabled: true }, 3));
      assert.equal((await feed.next()).seq, up.event_id);
      const body = action({}, 4, 'disable-once');
      const disabled = await request('POST', '/resources/thermostat/room/actions/disable', body);
      const event = await feed.next(); assert.equal(event.seq, disabled.event_id); assert.equal(event.resource.data.enabled, false);
      assert.deepEqual(await request('POST', '/resources/thermostat/room/actions/disable', body), disabled);
      tail = disabled.event_id; thermostat = disabled;
    } finally { feed.close(); }
  });
  await probe('concurrent same-revision updates produce one winner and one conflict', async () => {
    const bodies = [action({ reading: 15 }, 1), action({ reading: 16 }, 1)];
    const responses = await Promise.all(bodies.map(body => fetch(base + '/resources/sensor/roof', { method: 'PATCH', headers: { 'content-type': 'application/json', 'x-principal': 'alice' }, body: JSON.stringify(body) })));
    assert.deepEqual(responses.map(r => r.status).sort(), [200,409]);
    const winning = await responses.find(r => r.status === 200).json(); tail = winning.event_id;
    assert.equal((await request('GET','/resources/sensor/roof')).revision,2);
  });
  await probe('injected failure after state/event/receipt SQL rolls back all three', async () => {
    const body = action({ reading: 99 }, 2, 'rollback-retry');
    await request('PATCH', '/resources/sensor/roof', body, 500, 'alice', { 'x-prototype-fail': 'after-event' });
    const after = await request('GET','/resources/sensor/roof'); assert.equal(after.revision, 2);
    assert.deepEqual(await request('GET', `/journal?after=${tail}`), []);
    const retry = await request('PATCH', '/resources/sensor/roof', body); assert.equal(retry.resource.revision, 3); tail = retry.event_id;
  });
  await probe('principal ownership covers reads, mutation, receipts and event feed', async () => {
    await request('GET', '/resources/thermostat/room', undefined, 403, 'bob');
    await request('POST', '/resources/thermostat/room/actions/disable', action({}, 4, 'disable-once'), 403, 'bob');
    assert.deepEqual(await request('GET','/resources/thermostat',undefined,200,'bob'),[]);
    assert.deepEqual(await request('GET','/journal?after=0',undefined,200,'bob'),[]);
    await request('GET','/schema',undefined,401,'');
  });
  await probe('hard restart preserves data, durable SSE cursor and resumes reaction', async () => {
    await stop(); await crashAfterReactionCommit(); await start();
    assert.equal((await request('GET','/resources/thermostat/room')).revision,5);
    const replay = await stream(thermostat.event_id - 1, 'alice', true);
    try { assert.equal((await replay.next()).seq, thermostat.event_id); } finally { replay.close(); }
    const expected = `disabled-${thermostat.event_id}`;
    let items=[]; for(let i=0;i<80;i++) { items=await request('GET','/resources/task'); if(items.some(r=>r.id===expected)) break; await sleep(25); }
    const derived=items.find(r=>r.id===expected); assert.ok(derived,'reaction must catch up from durable journal');
    assert.equal(derived.data.done,false); assert.equal(derived.owner,'alice');
    const journal=await request('GET','/journal?after=0'); assert.ok(journal.some(e=>e.kind==='task'&&e.id===expected));
    const reactionEvents = journal.filter(e => e.kind === 'task' && e.id.startsWith('disabled-'));
    assert.equal(reactionEvents.length, 3); assert.equal(new Set(reactionEvents.map(e=>e.id)).size, 3);
    await stop(); await start(); await sleep(250);
    assert.equal((await request('GET','/resources/task')).filter(r=>r.id===expected).length,1);
    assert.equal((await request('GET','/journal?after=0')).filter(e=>e.kind==='task'&&e.id===expected).length,1);
  });
  await probe('declaration-only third kind also supports delete, tombstone event and retry', async () => {
    const body=action({},3,'delete-sensor'); const deleted=await request('DELETE','/resources/sensor/roof',body);
    assert.equal(deleted.resource.deleted,true); assert.equal(deleted.resource.revision,4);
    assert.deepEqual(await request('DELETE','/resources/sensor/roof',body),deleted);
    await request('GET','/resources/sensor/roof',undefined,404);
    assert.deepEqual(await request('GET','/resources/sensor'),[]);
  });
  await probe('SSE recovers from a committed mutation whose notification hint is suppressed', async () => {
    await stop(); await start({ PROTOTYPE_DROP_HINTS: '1' });
    const history = await request('GET','/journal?after=0');
    const feed = await stream(history.at(-1).seq);
    try {
      const created = await request('POST','/resources/task/silent',action({title:'No wake-up',done:false,note:null}));
      assert.equal((await feed.next()).seq, created.event_id);
    } finally { feed.close(); }
  });
  await probe('longest accepted source ID cannot stall later reactions', async () => {
    const longId = 'x'.repeat(128);
    await request('POST', '/resources/thermostat/' + longId, action({enabled:true,target:0,note:null}));
    const first = await request('PATCH', '/resources/thermostat/' + longId, action({enabled:false},1));
    await request('POST', '/resources/thermostat/later', action({enabled:true,target:0,note:null}));
    const second = await request('PATCH', '/resources/thermostat/later', action({enabled:false},1));
    let journal=[];
    for(let i=0;i<80;i++) {
      journal=await request('GET','/journal?after='+first.event_id);
      if(journal.filter(e=>e.kind==='task').length===2) break;
      await sleep(25);
    }
    const targets=journal.filter(e=>e.kind==='task');
    assert.equal(targets.length,2,'both longest-ID event and subsequent short-ID event must react');
    assert.ok(targets.every(e=>e.id.length<=128));
    assert.ok(targets.some(e=>e.id.endsWith('-'+first.event_id)));
    assert.ok(targets.some(e=>e.id.endsWith('-'+second.event_id)));
  });
  console.log(`VERDICT: ${passed} integration probes passed; SQLite is a scratch probe, not a database selection.`);
} finally { await stop(); await rm(dir,{recursive:true,force:true}); }
