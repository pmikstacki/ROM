import {projectCryptoWrapper} from './crypto-wrapper.mjs';
import {probeBrowserTrust} from './tls-trust-probe.mjs';
const record='/var/tmp/rom-010-authentik-20261007/run/volume/evidence/crypto-tools-e08d62bfc46542ea409ca49c/result.json';
await probeBrowserTrust({cryptoWrapper:projectCryptoWrapper(record)});
