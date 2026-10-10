// Resolution may prune/add local entries, but starts with the exact immutable registry selections.
import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {hash} from '../../../scripts/skills/files.mjs';
export function seedProducerLock(producer,app,expectedHash){
  const original=join(producer,'Cargo.lock');
  if(!/^[a-f0-9]{64}$/.test(expectedHash??'')||hash(original)!==expectedHash)throw Error('producer lock identity changed');
  writeFileSync(join(app,'Cargo.lock'),readFileSync(original),{flag:'wx'});
  if(hash(join(app,'Cargo.lock'))!==expectedHash)throw Error('producer lock seed copy changed');
  return expectedHash;
}
