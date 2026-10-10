import { lstatSync, realpathSync, statfsSync } from 'node:fs';

export function admitBrowserStorage(root, paths) {
  if (typeof root !== 'string' || !root.startsWith('/') || realpathSync(root) !== root || !lstatSync(root).isDirectory()) throw Error('browser storage root is not an owned regular directory');
  const device = lstatSync(root).dev, directories = {};
  for (const name of ['home', 'temporary', 'profile']) {
    const path = paths[name];
    if (name === 'profile' && path === undefined) continue;
    if (typeof path !== 'string' || !path.startsWith(`${root}/`) || realpathSync(path) !== path) throw Error('browser storage escapes the owned run filesystem');
    const stat = lstatSync(path);
    if (!stat.isDirectory() || stat.isSymbolicLink() || stat.dev !== device) throw Error('browser storage requires a directory on the owned run filesystem');
    directories[name] = { realpath: path, device: stat.dev, inode: stat.ino };
  }
  const filesystem = statfsSync(root);
  return { root, device, filesystem_type: filesystem.type, block_size: filesystem.bsize, available_bytes: Number(filesystem.bavail) * Number(filesystem.bsize), directories };
}
