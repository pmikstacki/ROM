// Owned TLS endpoints. Original provider bodies pass through unchanged.
import { createServer } from 'node:https';
import { request as upstreamRequest } from 'node:http';
import { existsSync, writeFileSync } from 'node:fs';
import { readNativeTlsFile, readNativeTlsJson, nativeTlsPrivatePath } from './native-tls-io.mjs';
import { forwardedHeaders, admitProxyRequest } from './proxy-contract.mjs';
import { NativeTlsCallbackHold } from './native-tls-callback-hold.mjs';
import { NativeTlsHandoff } from './native-tls-handoff.mjs';
import { selectNativeTlsCase, nativeTlsCutoverRequired } from './native-tls-profile.mjs';

export async function serveNativeTlsProxy(path) {
  nativeTlsPrivatePath(path);
  const config = readNativeTlsJson(path, 8192);
  if (Object.keys(config).sort().join(',') !== 'case,control_directory,negative_leaf,result,stop_file,trusted_leaf') throw Error('closed native TLS proxy configuration required');
  for (const name of ['control_directory', 'result', 'stop_file']) nativeTlsPrivatePath(config[name]);
  const selected = selectNativeTlsCase(config.case), handoff = new NativeTlsHandoff(selected.route), callbackHold = new NativeTlsCallbackHold();
  const materials = leaf => ({ key: readNativeTlsFile(leaf.key, 65536, true), cert: readNativeTlsFile(leaf.certificate, 65536) });
  const trusted = materials(config.trusted_leaf), negative = materials(config.negative_leaf);
  const sockets = new Set(), upstreams = new Set(), servers = [];
  const counts = { native_tcp_connections: 0, native_tls_handshakes: 0, tls_errors: 0, token_http_requests: 0, jwks_http_requests: 0, token_forwarded: 0, target_route: selected.route };
  let armed = false, failed = false, requests = 0, inflight = 0, responseBytes = 0, provider;
  const rotate = () => { if (nativeTlsCutoverRequired(selected)) provider.setSecureContext(negative); };
  const route = lane => (request, response) => {
    try {
      admitProxyRequest(lane, request.method, request.url);
      if (++requests > 128 || inflight >= 16) throw Error('proxy request bound');
      if (lane === 'host') {
        const decision = callbackHold.accept(request.method, request.url, request.headers.cookie, armed);
        if (decision.kind === 'hold') {
          writeFileSync(`${config.control_directory}/callback-held.json`, JSON.stringify(decision.handoff), { flag: 'wx', mode: 0o600 });
          response.writeHead(204, { 'Cache-Control': 'no-store' }); response.end(); return;
        }
      }
      if (lane === 'provider' && armed) {
        admitProxyRequest('private', request.method, request.url);
        if (request.url === '/application/o/token/') counts.token_http_requests++;
        else { if (selected.route === 'jwks' && selected.certificate !== 'trusted') handoff.beforeJwks(); counts.jwks_http_requests++; }
      }
    } catch { failed = true; response.writeHead(400); response.end(); return; }
    inflight++;
    const upstream = upstreamRequest({ host: '127.0.0.1', port: lane === 'host' ? 44391 : 44390, method: request.method, path: request.url, headers: forwardedHeaders(request.headers, lane), timeout: 5000 });
    upstreams.add(upstream); let done = false, input = 0, output = 0;
    const finish = () => { if (done) return; done = true; inflight--; upstreams.delete(upstream); };
    const fail = () => { failed = true; upstream.destroy(); if (!response.headersSent) response.writeHead(502); response.end(); finish(); };
    upstream.on('timeout', fail); upstream.on('error', fail); request.on('aborted', fail); response.on('close', () => { upstream.destroy(); finish(); });
    request.on('data', bytes => { input += bytes.length; if (input > 65536) fail(); });
    upstream.on('response', incoming => {
      const token = armed && lane === 'provider' && request.url === '/application/o/token/' && incoming.statusCode === 200;
      const headers = { ...incoming.headers }; delete headers.connection; delete headers['transfer-encoding'];
      if (token && selected.route === 'jwks' && selected.certificate !== 'trusted') headers.connection = 'close';
      response.writeHead(incoming.statusCode ?? 502, headers);
      incoming.on('data', bytes => { output += bytes.length; responseBytes += bytes.length; if (output > 4 * 1024 ** 2 || responseBytes > 32 * 1024 ** 2) fail(); });
      incoming.on('error', fail);
      incoming.on('end', () => {
        if (token) {
          counts.token_forwarded++;
          if (selected.route === 'jwks' && selected.certificate !== 'trusted') {
            try {
              handoff.tokenForwarded(200); rotate();
              // Connection: close forces a new handshake before JWKS acquisition.
              request.socket.once('close', () => { try { handoff.nativeConnectionClosed(); } catch { failed = true; } });
            } catch { failed = true; }
          }
        }
        finish();
      });
      incoming.pipe(response);
    }); request.pipe(upstream);
  };
  try {
    for (const [lane, port] of [['host', 44389], ['provider', 44392]]) {
      const server = createServer({ ...trusted, minVersion: 'TLSv1.2' }, route(lane));
      if (lane === 'provider') provider = server;
      server.requestTimeout = 10000; server.headersTimeout = 5000; server.maxHeadersCount = 64;
      server.on('connection', socket => { sockets.add(socket); socket.once('close', () => sockets.delete(socket)); if (lane === 'provider' && armed && ++counts.native_tcp_connections > 16) { failed = true; socket.destroy(); } });
      server.on('secureConnection', () => { if (lane === 'provider' && armed) counts.native_tls_handshakes++; });
      server.on('tlsClientError', () => { if (lane === 'provider' && armed) counts.tls_errors++; });
      servers.push(server); await new Promise((resolve, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', resolve); });
    }
    const deadline = Date.now() + 120000;
    while (Date.now() < deadline && !existsSync(config.stop_file) && !failed) {
      const arm = `${config.control_directory}/arm.json`;
      if (!armed && existsSync(arm)) {
        const value = readNativeTlsJson(arm, 4096);
        if (Object.keys(value).sort().join(',') !== 'authorization_code_held,browser_drained' || value.authorization_code_held !== true || value.browser_drained !== true) throw Error('physical handoff evidence required');
        if (!callbackHold.captured) throw Error('original callback must be held before arm');
        handoff.authorizationHeld(); handoff.browserDrained(); handoff.arm();
        if (selected.route === 'token') rotate();
        armed = true;
        writeFileSync(`${config.control_directory}/armed.json`, JSON.stringify({ armed: true, route: selected.route }), { flag: 'wx', mode: 0o600 });
      }
      await new Promise(resolve => setTimeout(resolve, 25));
    }
  } finally {
    for (const upstream of upstreams) upstream.destroy();
    for (const socket of sockets) socket.destroy();
    for (const server of servers) { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
    writeFileSync(config.result, JSON.stringify({ ...counts, failed, armed, callback_held: callbackHold.captured, requests, response_bytes: responseBytes }), { flag: 'wx', mode: 0o600 });
  }
  if (failed) throw Error('native TLS proxy failed');
}
