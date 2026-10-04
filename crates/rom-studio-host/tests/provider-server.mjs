import { startHumanProvider } from '../../../demo/provider-fixture/human-provider.mjs';
import { createInterface } from 'node:readline';
let pauseToken = false;
let release;
const output = value => process.stdout.write(JSON.stringify(value) + '\n');
const provider = await startHumanProvider({
  clientId: 'studio', clientSecret: 'controlled-host-test-secret', redirectUri: process.argv[2],
  beforeRequest: async request => {
    if (request.url !== '/token' || !pauseToken) return;
    pauseToken = false;
    output({ event: 'token-started' });
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => { release = undefined; reject(Error('private barrier timeout')); }, 4000);
      release = () => { clearTimeout(timer); release = undefined; resolve(); };
    });
  },
});
output({ issuer: provider.issuer });
const input = createInterface({ input: process.stdin });
input.on('line', async line => {
  const command = JSON.parse(line);
  if (command === 'pause-token') { pauseToken = true; output({ event: 'armed' }); }
  else if (command === 'release') release?.();
  else if (command === 'close') { release?.(); await provider.close(); process.exit(0); }
});
input.on('close', async () => { release?.(); await provider.close(); process.exit(0); });
