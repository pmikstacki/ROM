// Allocated loopback endpoints only. Listener ownership comes from actual launch identities.
import { execFileSync } from 'node:child_process';
import { readProcessIdentity, sameProcessIdentity } from './launch.mjs';

export function requireListenerProjection(output, port, pid) {
  requirePort(port);
  if (typeof output !== 'string' || output.length > 16384) throw Error('invalid allocated listener');
  const lines = output.trim().split('\n');
  if (!lines.shift()?.startsWith('State ')) throw Error('listener query format mismatch');
  if (pid === null) {
    if (lines.length) throw Error('allocated endpoint occupied');
    return;
  }
  if (!Number.isSafeInteger(pid) || pid < 2 || lines.length !== 1) throw Error('owned listener missing or ambiguous');
  const line = lines[0], owners = [...line.matchAll(/pid=(\d+)/g)].map(match => Number(match[1]));
  if (!new RegExp(`\\s127\\.0\\.0\\.1:${port}\\s`).test(line) || owners.length !== 1 || owners[0] !== pid) throw Error('listener owner or binding mismatch');
}
function requirePort(port) {
  if (!Number.isInteger(port) || port < 44389 || port > 44393) throw Error('invalid allocated listener');
}
function inspect(port) {
  requirePort(port);
  return execFileSync('/run/current-system/sw/bin/ss', ['-ltnp', `( sport = :${port} )`], { encoding: 'utf8', timeout: 1000, maxBuffer: 16384 });
}
export function requireVacantListener(port) { requireListenerProjection(inspect(port), port, null); }
export function requireOwnedListener(port, identity) {
  if (!sameProcessIdentity(identity, readProcessIdentity(identity?.pid))) throw Error('listener process identity mismatch');
  requireListenerProjection(inspect(port), port, identity.pid);
  if (!sameProcessIdentity(identity, readProcessIdentity(identity.pid))) throw Error('listener process identity changed');
}
