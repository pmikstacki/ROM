// Private local inputs for the development fixture. No raw errors leave command wrappers.
import { constants } from 'node:fs';
import { open } from 'node:fs/promises';

export async function readPrivate(path, limit) {
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
  try {
    const metadata = await file.stat();
    if (!metadata.isFile() || (metadata.mode & 0o077) || metadata.size > limit)
      throw new Error('private input rejected');
    const buffer = Buffer.alloc(limit + 1);
    let total = 0;
    while (total < buffer.length) {
      const { bytesRead } = await file.read(buffer, total, buffer.length - total, null);
      if (!bytesRead) break;
      total += bytesRead;
    }
    if (!total || total > limit) throw new Error('private input rejected');
    return new TextDecoder('utf-8', { fatal: true }).decode(buffer.subarray(0, total));
  } finally {
    await file.close();
  }
}

export async function writePrivate(path, contents) {
  const file = await open(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL, 0o600);
  try {
    await file.writeFile(contents);
  } finally {
    await file.close();
  }
}
