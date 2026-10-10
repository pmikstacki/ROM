// Generated execution trees remain outside artifacts. Retain only bounded regular evidence.
import { mkdirSync, lstatSync, realpathSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { hash, safePath } from '../skills/files.mjs';
export function retainRegularEvidence(source, target, selected) {
  if(realpathSync(source)!==source||resolve(target)!==target||realpathSync(dirname(target))!==dirname(target)||!Array.isArray(selected)||!selected.length||new Set(selected).size!==selected.length)throw Error('invalid regular evidence roots');
  mkdirSync(target);let count=0,bytes=0;const files={};
  function copy(path,depth=0){
    if(++count>100000||depth>32)throw Error('regular evidence inventory exceeds bound');
    const input=safePath(source,path),stat=lstatSync(input),output=join(target,path);
    if(stat.isDirectory()){mkdirSync(dirname(output),{recursive:true});mkdirSync(output);for(const name of readdirSync(input).sort())copy(`${path}/${name}`,depth+1);}
    else{
      bytes+=stat.size;if(!stat.isFile()||stat.nlink!==1||stat.size>64*1024*1024||bytes>512*1024*1024)throw Error('invalid regular evidence file');
      mkdirSync(dirname(output),{recursive:true});const data=readFileSync(input);writeFileSync(output,data,{flag:'wx',mode:stat.mode&0o777});
      if(hash(input)!==hash(output))throw Error('regular evidence changed during retention');files[path]=hash(output);
    }
  }
  for(const path of selected)copy(path);return files;
}
export const consumerRetentionPaths=Object.freeze(['result.json','browser-results.json','studio-source.tar.gz','consumer/package-lock.json','consumer/node_modules/rom-studio/package.json','consumer/node_modules/rom-studio/src','consumer/node_modules/rom-studio/LICENSE','consumer/node_modules/rom-studio/THIRD_PARTY_NOTICES.md']);
export const recoveryRetentionPaths=Object.freeze(['http-results.json','native-provenance.json','consumer/src','consumer/tests','consumer/package.json','consumer/tsconfig.json','consumer/vite.config.ts','consumer/index.html','consumer/dist','consumer/http-dist','runtime']);
