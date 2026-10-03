import { startHumanProvider } from '../../../demo/provider-fixture/human-provider.mjs';
const provider = await startHumanProvider({ clientId: 'studio', clientSecret: 'controlled-host-test-secret', redirectUri: process.argv[2] });
process.stdout.write(JSON.stringify({issuer: provider.issuer}) + '\n');
process.stdin.resume();
process.stdin.once('data', async () => { await provider.close(); process.exit(0); });
