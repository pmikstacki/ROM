// Checkout HEAD is diagnostic metadata, not proof of archived or compiled source.
import { execFileSync } from 'node:child_process';
import { realpathSync } from 'node:fs';

export function sourceCheckoutHead(root, mode) {
  if (mode === 'release_artifact' || mode === 'extracted_release_source') return null;
  if (mode !== 'authoring_checkout') throw Error('invalid recovery source mode');
  const git = args => execFileSync('git', args, {
    cwd: root, encoding: 'utf8', timeout: 5000, maxBuffer: 65536,
  }).trim();
  if (realpathSync(git(['rev-parse', '--show-toplevel'])) !== realpathSync(root)) {
    throw Error('recovery source is not the checkout root');
  }
  const head = git(['rev-parse', 'HEAD']);
  if (!/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(head)) throw Error('invalid checkout revision');
  return head;
}
