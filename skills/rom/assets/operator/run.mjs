// A bounded transport fault demonstrates uncertainty without asserting native commit.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { writeFileSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

export async function run(context) {
  const project = await context.rust('operator');
  const generated = await context.command('cargo', ['run', '--offline', '--manifest-path', join(context.project, 'Cargo.toml')], { cwd: context.project });
  const request = JSON.parse(generated.stdout);
  const file = join(context.evidence, 'request.json');
  writeFileSync(file, JSON.stringify(request) + '\n', { mode: 0o600, flag: 'wx' });
  const digest = () => createHash('sha256').update(readFileSync(file)).digest('hex');
  const original = digest();
  let submissions = 0;
  let protocolError;
  const server = createServer((incoming, response) => {
    incoming.setTimeout(2000, () => incoming.destroy());
    const chunks = [];
    let bytes = 0;
    incoming.on('data', chunk => {
      bytes += chunk.length;
      if (bytes > 16_384) incoming.destroy(); else chunks.push(chunk);
    });
    incoming.on('end', () => {
      submissions++;
      try {
        assert.equal(incoming.method, 'POST');
        assert.equal(incoming.url, '/work/control');
        assert.deepEqual(JSON.parse(Buffer.concat(chunks)), request);
      } catch (error) { protocolError = error; }
      response.writeHead(200, { 'content-type': 'application/json', connection: 'close' });
      response.end('malformed submitted reply');
    });
  });
  try {
    await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
    const endpoint = `http://127.0.0.1:${server.address().port}`;
    const result = await context.command('cargo', ['run', '-p', 'rom-cli', '--locked', '--offline', '--bin', 'rom', '--', '--endpoint', endpoint, '--request-timeout', '2', 'work', 'retry', '--request-file', file], { expected: 5, timeout: 30000 });
    assert.equal(result.stdout, '');
    assert.match(result.stderr, /outcome unresolved/);
    assert.equal(submissions, 1);
    assert.equal(protocolError, undefined);
    assert.equal(digest(), original);
    const diagnosis = {
      outcome: 'unresolved', request_sha256: original, request,
      submissions, automatic_retry: false, native_commit_established: false,
      later_commands: [
        'rom --endpoint "$ROM_ENDPOINT" --auth-file "$ROM_AUTH_FILE" work capabilities',
        'rom --endpoint "$ROM_ENDPOINT" --auth-file "$ROM_AUTH_FILE" work list',
        `rom --endpoint "$ROM_ENDPOINT" --auth-file "$ROM_AUTH_FILE" work show ${request.handle}`,
        `rom --endpoint "$ROM_ENDPOINT" --auth-file "$ROM_AUTH_FILE" work retry --request-file "${file}"`,
      ],
      recovery_commands_executed: false,
    };
    writeFileSync(join(context.evidence, 'diagnosis.json'), JSON.stringify(diagnosis, null, 2) + '\n');
    return { ...project, ...diagnosis };
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
}
