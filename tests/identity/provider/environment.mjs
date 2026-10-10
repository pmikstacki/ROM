// Isolated Podman CLI identity. No default store, daemon connection or user auth file.
import { mkdirSync, writeFileSync, realpathSync } from 'node:fs';
import { join, resolve } from 'node:path';

export function isolatedPodman(root, { directory: name = 'engine', driver = 'vfs' } = {}) {
  if (typeof root !== 'string' || resolve(root) !== root || root === '/' || realpathSync(root) !== root) throw Error('invalid isolated engine root');
  if (!['engine', 'bounded'].includes(name)) throw Error('invalid isolated engine directory');
  if (!['vfs', 'overlay'].includes(driver)) throw Error('invalid isolated engine driver');
  const directory = join(root, name); mkdirSync(directory, { mode: 0o700 });
  for (const name of ['root', 'runroot', 'volumes', 'networks', 'tmp', 'hooks', 'download']) mkdirSync(join(directory, name), { mode: 0o700 });
  const auth = join(directory, 'auth.json'); writeFileSync(auth, '{}\n', { flag: 'wx', mode: 0o600 });
  const storage = join(directory, 'storage.conf');
  writeFileSync(storage, `[storage]\ndriver=${JSON.stringify(driver)}\ngraphroot=${JSON.stringify(join(directory, 'root'))}\nrunroot=${JSON.stringify(join(directory, 'runroot'))}\n[storage.options]\nadditionalimagestores=[]\n`, { flag: 'wx', mode: 0o600 });
  const args = ['--remote=false', `--storage-driver=${driver}`, '--storage-opt=', '--events-backend=file', '--cgroup-manager=cgroupfs',
    ...Object.entries({ root: 'root', runroot: 'runroot', volumepath: 'volumes', 'network-config-dir': 'networks', tmpdir: 'tmp', 'hooks-dir': 'hooks' }).map(([flag, path]) => `--${flag}=${join(directory, path)}`)];
  return Object.freeze({ root, directory, driver, executable: '/run/current-system/sw/bin/podman', args,
    env: { PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC', GOMAXPROCS: '2',
      TMPDIR: join(directory, 'download'), REGISTRY_AUTH_FILE: auth, CONTAINERS_STORAGE_CONF: storage } });
}

export function requireIsolatedInfo(engine, info) {
  if (info?.host?.serviceIsRemote !== false || info?.store?.graphRoot !== join(engine.directory, 'root') ||
      info.store.runRoot !== join(engine.directory, 'runroot') || info.store.graphDriverName !== engine.driver ||
      info.store.containerStore?.number !== 0 || info.store.imageStore?.number !== 0) throw Error('isolated engine identity mismatch');
  return info;
}
