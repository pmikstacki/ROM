// Same-filesystem GNU no-copy/no-replace publication on the supported Linux host.
import { existsSync, lstatSync, mkdirSync, mkdtempSync, readdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { capture } from './source.mjs';

export function admitOutput(output) {
  if (process.platform !== 'linux') throw Error('release requires Linux');
  let present = false;
  try { lstatSync(output); present = true; }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  if (present) throw Error('release output exists');
  mkdirSync(dirname(output), { recursive: true });
  if (lstatSync(dirname(output)).isSymbolicLink()) throw Error('symbolic output parent');
}
export function publish(stage, output) {
  if (lstatSync(stage).dev !== lstatSync(dirname(output)).dev) throw Error('release stage must share output filesystem');
  capture('mv', ['-T', '--no-copy', '--update=none-fail', stage, output], dirname(output));
  if (existsSync(stage) || !existsSync(output)) throw Error('incomplete release publication');
}
export function capability(parent) {
  const filesystem = capture('findmnt', ['-T', parent, '-n', '-o', 'FSTYPE'], parent).trim();
  if (filesystem !== 'ext4') throw Error('release publication requires tested ext4 filesystem');
  const scratch = mkdtempSync(join(parent, '.rom-release-capability-'));
  try {
    const a = join(scratch, 'a'), b = join(scratch, 'b'), dest = join(scratch, 'dest');
    mkdirSync(a); mkdirSync(b);
    writeFileSync(join(a, 'complete'), 'original'); writeFileSync(join(b, 'complete'), 'collision');
    publish(a, dest);
    let rejected = false;
    try { publish(b, dest); } catch { rejected = true; }
    if (!rejected || !existsSync(b) || readFileSync(join(dest, 'complete'), 'utf8') !== 'original' || readdirSync(dest).length !== 1) throw Error('no-replace publication capability unavailable');
    return { platform: 'linux', filesystem, operation: 'mv -T --no-copy --update=none-fail', same_filesystem: true, existing_destination_rejected: true };
  } finally { rmSync(scratch, { recursive: true, force: true }); }
}
