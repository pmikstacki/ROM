import {readFileSync} from 'node:fs';
import {enterPrivateTrustView} from './trust-namespace.mjs';
import { runHttpsIdentityBrowser } from './https-browser-journey.mjs';
try { const configuration=JSON.parse(readFileSync(process.argv[2],'utf8'));if(configuration.engine==='webkit')await enterPrivateTrustView(configuration);await runHttpsIdentityBrowser(process.argv[2]); }
catch { process.exitCode=1;console.log(JSON.stringify({status:'https-identity-browser-fixture-failed'})); }
