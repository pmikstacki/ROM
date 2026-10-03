import { issueToken } from './provider.mjs';
import { readPrivate, writePrivate } from './files.mjs';

try {
  const [readyFile, secretFile, outputFile, ...extra] = process.argv.slice(2);
  if (!readyFile || !secretFile || !outputFile || extra.length) throw new Error('invalid arguments');
  const ready = JSON.parse(await readPrivate(readyFile, 4096));
  const secret = await readPrivate(secretFile, 4096);
  const token = await issueToken(ready.issuer, secret);
  await writePrivate(outputFile, `Bearer ${token}`);
  console.log('Private authentication file created.');
} catch {
  console.error('Token acquisition failed; credential details withheld.');
  process.exitCode = 1;
}
