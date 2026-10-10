// Admission requires retained executed evidence; this module never infers behavior from metadata.
import { lstatSync, readFileSync, realpathSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { hash, safePath } from '../skills/files.mjs';
import { requireReleaseRequirements } from './requirements.mjs';
import { STUDIO_PROFILE, PUBLIC_CONSUMERS_PROFILE } from './commands.mjs';
export function productionProfile(version) {
  const match=/^(\d+)\.(\d+)\.(\d+)(?:[-+][A-Za-z0-9.-]+)?$/.exec(version);
  if(!match)throw Error('invalid production version');
  return Number(match[1])>0||Number(match[2])>=1?PUBLIC_CONSUMERS_PROFILE:STUDIO_PROFILE;
}
export function admitProductionRequirements(input, sourceIdentity) {
  if(!input||Object.keys(input).sort().join(',')!=='directory,sha256'||typeof input.directory!=='string'||resolve(input.directory)!==input.directory||realpathSync(input.directory)!==input.directory||!/^[a-f0-9]{64}$/.test(input.sha256??''))throw Error('release requirement evidence required');
  const path=safePath(input.directory,'record.json'),stat=lstatSync(path);
  if(!stat.isFile()||stat.nlink!==1||stat.size>1024*1024||hash(path)!==input.sha256)throw Error('release requirement record mismatch');
  const record=JSON.parse(readFileSync(path,'utf8'));
  const summary=requireReleaseRequirements(record,sourceIdentity,input.directory,{requireComplete:true});
  return Object.freeze({directory:input.directory,sha256:input.sha256,sourceIdentity,record:structuredClone(record),summary});
}
export function retainProductionRequirements(admitted, stage) {
  const current=admitProductionRequirements({directory:admitted.directory,sha256:admitted.sha256},admitted.sourceIdentity);
  const target=join(stage,'evidence/requirements');mkdirSync(target);
  const refs=current.record.requirements.flatMap(entry=>[...entry.evidence,entry.review]),seen=new Set();let bytes=0;
  for(const ref of refs){
    if(seen.has(ref.path))continue;seen.add(ref.path);
    const input=safePath(current.directory,ref.path),data=readFileSync(input);bytes+=data.length;
    if(bytes>256*1024*1024||data.length!==ref.bytes||hash(input)!==ref.sha256)throw Error('release requirement evidence changed');
    const output=join(target,ref.path);mkdirSync(join(output,'..'),{recursive:true});writeFileSync(output,data,{flag:'wx'});
  }
  writeFileSync(join(target,'record.json'),JSON.stringify(current.record,null,2)+'\n',{flag:'wx'});
  requireReleaseRequirements(current.record,current.sourceIdentity,target,{requireComplete:true});
  return {path:'evidence/requirements/record.json',record:current.record};
}

export function requireManifestProfile(manifest) {
  if (productionProfile(manifest.source?.package_version) === PUBLIC_CONSUMERS_PROFILE &&
      (manifest.manifest_version !== 3 || manifest.verification_profile !== PUBLIC_CONSUMERS_PROFILE))
    throw Error('production release requires executed public consumer profile');
  return true;
}
