import test from 'node:test';
import assert from 'node:assert/strict';
import { nixSha256 } from './nix-hash.mjs';
test('Nix hash conversion matches actual admitted tool output without one process per closure path', () => {
  assert.equal(nixSha256('sha256-lLrPeXc1aPmkJ38bDMtz3ZZIirSXK6efme+MZNI/gvQ='), 'sha256:1x427z96937gk6gsfawpnj54i5nxfg5hq6vz4yjgjs1mfxwwzfll');
  assert.equal(nixSha256('sha256-sAXgsTcXASoX/80ZQtYq7cdTMggfvZNwBiZtvN9UGOE='), 'sha256:1q8qakgvqv960rq97g8z10r57izd5bb446fdzwbjl08p6yqy01dh');
});
test('conversion rejects other algorithms, malformed hashes and noncanonical base64', () => {
  for (const hash of ['', 'sha512-lLrPeXc1aPmkJ38bDMtz3ZZIirSXK6efme+MZNI/gvQ=', 'sha256-not-a-hash', 'sha256-'+Buffer.alloc(32).toString('base64').replace('A=', 'B=')]) assert.throws(() => nixSha256(hash));
});
