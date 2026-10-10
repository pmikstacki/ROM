// Loopback acceptance proxy: real HTTP, finite host lifetime, no product endpoints added.
import { createServer } from 'node:http';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { readFileSync, appendFileSync } from 'node:fs';
import { resolve, join, extname, sep } from 'node:path';

export async function startProxy({ binary, adapter, database, port, budgetMs = 180000, dist, evidence }) {
  let child, address, generation = 0, revoked = false, armed = null, held = null;
  const trace = [], dropped = [];
  async function upstream(path, body, headers = {}) {
    return fetch(`http://${address}${path}`, { method: 'POST', headers: { 'content-type': 'application/json', ...headers }, body: typeof body === 'string' ? body : JSON.stringify(body), signal: AbortSignal.timeout(10000) });
  }
  async function launch() {
    child = spawn(binary, [adapter, database, '0', String(budgetMs)], { stdio: ['ignore', 'pipe', 'pipe'] });
    child.stderr.on('data', bytes => appendFileSync(join(evidence, 'host-stderr.log'), bytes));
    let buffered = '';
    await new Promise((yes, no) => {
      const timeout = setTimeout(() => { child.kill('SIGTERM'); no(Error('fixture host startup timeout')); }, 20000);
      const fail = error => { clearTimeout(timeout); no(error); };
      child.once('error', fail); child.once('exit', code => fail(Error(`fixture host exited ${code}`)));
      child.stdout.on('data', bytes => {
        appendFileSync(join(evidence, 'host-stdout.log'), bytes); buffered += bytes;
        if (!address && buffered.includes('\n')) {
          try { const ready = JSON.parse(buffered.split('\n')[0]); address = ready.address; clearTimeout(timeout); yes(); }
          catch (error) { fail(error); }
        }
      });
    });
    await upstream('/__fixture/control', { generation, revoked });
  }
  async function stopHost() {
    if (!child || !child.pid || child.exitCode !== null) return;
    const stopped = child, exited = once(stopped, 'exit'); stopped.kill('SIGTERM');
    const timeout = setTimeout(() => stopped.kill('SIGKILL'), 3000);
    try { await exited; } finally { clearTimeout(timeout); }
  }
  try { await launch(); } catch (error) { await stopHost(); throw error; }
  const server = createServer(async (request, response) => {
    try {
      const path = new URL(request.url, 'http://localhost').pathname;
      if (request.method === 'POST') {
        let bytes = 0, body = '';
        for await (const chunk of request) { bytes += chunk.length; if (bytes > 1048576) throw Error('proxy body limit'); body += chunk; }
        const value = body ? JSON.parse(body) : {};
        if (path.startsWith('/__test/')) {
          let output;
          if (path === '/__test/drop') { armed = { id: value.id, key: value.key }; output = { armed: true }; }
          else if (path === '/__test/hold') { let release; const promise=new Promise(resolve => { release=resolve; });held={id:value.id,promise,release};output={holding:true}; }
          else if (path === '/__test/release') { held?.release();held=null;output={released:true}; }
          else if (path === '/__test/resume') { armed = null; output = { resumed:true }; }
          else if (path === '/__test/restart') { await stopHost(); address = null; await launch(); output = { restarted: true }; }
          else if (path === '/__test/retire') { await stopHost(); const destination=database+'.retired'; const retired=spawnSync(binary,['retain',adapter,database,destination],{encoding:'utf8',timeout:30000,maxBuffer:1048576}); appendFileSync(join(evidence,'retention.log'),(retired.stdout??'')+(retired.stderr??'')); if(retired.status!==0||retired.error)throw Error('fixture retention failed'); database=destination; address=null; await launch(); output={retired:true}; }
          else if (path === '/__test/session') { generation++; await upstream('/__fixture/control', { generation }); output = { generation }; }
          else if (path === '/__test/session-state') output = { generation, revoked };
          else if (path === '/__test/revoke') { revoked = value.revoked; await upstream('/__fixture/control', { revoked }); output = { revoked }; }
          else if (path === '/__test/trace') output = { bodies: trace.filter(x => JSON.parse(x).id === value.id), dropped: dropped.filter(x => x.id === value.id) };
          else if (path === '/__test/inspect') { const wire = await (await upstream('/__fixture/inspect', value)).text(); output = { ...JSON.parse(wire), wire }; }
          else throw Error('unknown fixture control');
          response.writeHead(200, { 'content-type': 'application/json' }); response.end(JSON.stringify(output)); return;
        }
        if (!path.startsWith('/api/')) throw Error('unknown fixture route');
        const headers = {};
        for (const name of ['authorization', 'x-rom-csrf']) if (request.headers[name]) headers[name] = String(request.headers[name]);
        if (path === '/api/invoke') trace.push(body);
        const upstreamResponse = await upstream(path.slice(4), body, headers);
        const content = await upstreamResponse.text();
        if (path === '/api/invoke' && upstreamResponse.status === 200 && armed?.id === value.id && (!armed.key || armed.key === value.idempotency)) {
          const operation = value.operation.type === 'action' ? value.operation.input.name : value.operation.type;
          const wire = await (await upstream('/__fixture/inspect', { id: value.id, idempotency: value.idempotency, operation })).text();
          const committed = { ...JSON.parse(wire), wire };
          if (!committed.receipt || committed.receipt.identity !== committed.expected_identity || committed.row?.revision !== committed.receipt.row.revision) throw Error('drop requires confirmed durable receipt');
          dropped.push({ id: value.id, key: value.idempotency, committed });
          appendFileSync(join(evidence, 'confirmed-dropped-ack.jsonl'), JSON.stringify({ id: value.id, key: value.idempotency, committed }) + '\n');
          armed = { id:value.id, key:value.idempotency }; response.destroy(); return;
        }
        if (path === '/api/invoke' && upstreamResponse.status === 200 && held?.id === value.id) await held.promise;
        if (response.destroyed) return;
        response.writeHead(upstreamResponse.status, { 'content-type': upstreamResponse.headers.get('content-type') ?? 'application/json' }); response.end(content); return;
      }
      if (request.method !== 'GET' || !dist) throw Error('unsupported method');
      const staticPath = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
      const file = resolve(dist, '.' + (staticPath === '/' ? '/index.html' : staticPath));
      if (!file.startsWith(resolve(dist) + sep)) throw Error('static path escaped');
      const types = { '.html':'text/html', '.js':'text/javascript', '.css':'text/css', '.woff2':'font/woff2', '.svg':'image/svg+xml' };
      const content = readFileSync(file);
      response.writeHead(200, { 'content-type':types[extname(file)] ?? 'application/octet-stream' }); response.end(content);
    } catch (error) { if (!response.destroyed) { if (response.headersSent) response.destroy(); else { response.writeHead(error.code === 'ENOENT' ? 404 : 500, { 'content-type':'application/json' }); response.end(JSON.stringify({ error: error.message })); } } }
  });
  server.requestTimeout = 15000; server.headersTimeout = 15000;
  await new Promise((yes, no) => { server.once('error', no); server.listen(port, '127.0.0.1', yes); });
  const timer = setTimeout(() => { void close(); }, budgetMs);
  let closing;
  function close() { if (!closing) closing = (async () => { clearTimeout(timer); server.closeAllConnections(); await new Promise(yes => server.close(yes)); await stopHost(); })(); return closing; }
  return { base: `http://127.0.0.1:${server.address().port}`, close };
}
