import { test } from 'node:test';
import assert from 'node:assert/strict';
import { requireListenerProjection } from './network-admission.mjs';
const header = 'State Recv-Q Send-Q Local Address:Port Peer Address:PortProcess\n';
const row = 'LISTEN 0 128 127.0.0.1:44391 0.0.0.0:* users:(("owned-host",pid=1234,fd=6))\n';
test('listener admission requires exact allocated loopback port and owner PID', () => {
  assert.doesNotThrow(() => requireListenerProjection(header, 44391, null));
  assert.doesNotThrow(() => requireListenerProjection(header + row, 44391, 1234));
  for (const [output, port, pid] of [
    [header + row, 44391, null], [header + row, 44391, 9876],
    [header + row.replace('127.0.0.1:', '0.0.0.0:'), 44391, 1234],
    [header + row.replace('44391', '44392'), 44391, 1234],
    [header + row.replace('pid=1234', 'pid=1234,pid=9876'), 44391, 1234],
    [header, 44391, 1234], [header, 44388, null], ['unexpected', 44391, null],
  ]) assert.throws(() => requireListenerProjection(output, port, pid));
});
