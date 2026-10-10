// Consumers receive immutable source only after the complete existing artifact extraction checks.
import { openVerifiedExtraction } from './verification.mjs';
import { hash, safePath } from '../skills/files.mjs';

export async function withVerifiedExtraction(directory, source, artifacts, frontend, callback) {
  if (typeof callback !== 'function') throw Error('consumer callback required');
  if (!artifacts || !artifacts.source || !artifacts.skills) throw Error('missing consumer payload digests');
  for (const artifact of Object.values(artifacts)) {
    if (!artifact || typeof artifact.sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(artifact.sha256) ||
        hash(safePath(directory, artifact.path)) !== artifact.sha256) throw Error('consumer payload digest mismatch');
  }
  const lease = openVerifiedExtraction(directory, source, artifacts, frontend);
  try {
    const result = await callback(Object.freeze({ root: lease.root, witness: lease.witness, fence: lease.fence }));
    lease.fence();
    return result;
  } finally { lease.dispose(); }
}
