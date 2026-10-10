import {readFileSync,realpathSync} from 'node:fs';
import {enterPrivateTrustView} from './trust-namespace.mjs';
const path=process.argv[2];
if(!path?.startsWith('/var/tmp/rom-010-authentik-20261007/run/volume/private/tls-')||realpathSync(path)!==path)throw Error('owned namespace configuration required');
await enterPrivateTrustView(JSON.parse(readFileSync(path,'utf8')));
await import('./tls-browser-worker.mjs');
