// Isolated rejection probe. The parent owns the deadline and can kill a blocked read.
import { readPrivate } from './files.mjs';

try {
  await readPrivate(process.argv[2], 4096);
  process.exitCode = 1;
} catch {
  process.exitCode = 0;
}
