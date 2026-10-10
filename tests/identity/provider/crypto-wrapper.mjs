import {readFileSync,writeFileSync,realpathSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {readProcessIdentity,sameProcessIdentity} from './launch.mjs';
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
export function requireOwnedBrowserMembership(raw){if(typeof raw!=='string'||!/^0::\/rom-identity-[a-f0-9]{32}(?:\/launcher)?$/.test(raw))throw Error('owned browser cgroup required');}
export function renderCryptoWrapper(original,assetDirectory,libraryPath){
 if(![assetDirectory,libraryPath].every(p=>typeof p==='string'&&/^\/[A-Za-z0-9_./:+-]+$/.test(p))||original.split('MYDIR="$(dirname $(readlink -f $0))"').length!==2||original.split('export LD_LIBRARY_PATH="').length!==2||!original.endsWith('exec "${MYDIR}/bin/MiniBrowser" "$@"\n'))throw Error('fixed crypto wrapper structure required');
 return original.replace('MYDIR="$(dirname $(readlink -f $0))"',`MYDIR="${assetDirectory}"`).replace('export LD_LIBRARY_PATH="',`export LD_LIBRARY_PATH="${libraryPath}:`);
}
export function admitLoadedCrypto(paths,expected){
 if(!expected.every(p=>paths.includes(p))||paths.some(p=>/\/lib(?:gnutls|tasn1)\.so\./.test(p)&&!expected.includes(p)))throw Error('actual patched crypto mapping required');
}
export function projectCryptoWrapper(recordPath){
 const record=JSON.parse(readFileSync(recordPath,'utf8'));
 if(record.status!=='acquired-patched-crypto-static-loader-compatible'||!/^\/var\/tmp\/rom-010-authentik-20261007\/overlay\/acquisition\/crypto-tools-[a-f0-9]{24}$/.test(record.directory))throw Error('admitted crypto supplement required');
 for(const d of record.dependencies)if(hash(readFileSync(d.path))!==d.sha256)throw Error('crypto dependency drift');
 const original='/var/tmp/rom-studio-webkit-2359/minibrowser-wpe/MiniBrowser',assetDirectory=original.slice(0,original.lastIndexOf('/'));
 const body=readFileSync(original,'utf8'),wrapper=`${record.directory}/webkit-patched-crypto.sh`,bytes=Buffer.from(renderCryptoWrapper(body,assetDirectory,record.loader.library_path));
 writeFileSync(wrapper,bytes,{flag:'wx',mode:0o700});
 const projection={schema:'rom-private-webkit-crypto-supplement-v1',record_path:recordPath,record_sha256:hash(readFileSync(recordPath)),path:wrapper,sha256:hash(bytes),original:{path:original,realpath:realpathSync(original),sha256:hash(Buffer.from(body))},binary:{path:`${assetDirectory}/bin/MiniBrowser`,sha256:hash(readFileSync(`${assetDirectory}/bin/MiniBrowser`))},libraries:record.packages.map(p=>({path:p.library_path,sha256:p.library_sha256})),production_dependency_admission:false};
 writeFileSync(`${record.directory}/wrapper-identity.json`,JSON.stringify(projection,null,2)+'\n',{flag:'wx',mode:0o600});return projection;
}
export function snapshotLoadedCrypto(expected,{requireMatch=true}={}){
 const membership=readFileSync('/proc/self/cgroup','utf8').trim();requireOwnedBrowserMembership(membership);
 const pids=readFileSync(`/sys/fs/cgroup${membership.slice(3)}/cgroup.procs`,'utf8').trim().split('\n');if(pids.length>512)throw Error('browser process envelope');
 const processes=[],paths=new Set();
 for(const pid of pids){if(!/^[0-9]+$/.test(pid))continue;const before=readProcessIdentity(Number(pid));if(!before)continue;try{if(readFileSync(`/proc/${pid}/cgroup`,'utf8').trim()!==membership)throw Error('browser membership changed');const maps=readFileSync(`/proc/${pid}/maps`,'utf8');if(!sameProcessIdentity(before,readProcessIdentity(Number(pid))))throw Error('browser process birth changed');const selected=[...new Set(maps.split('\n').map(line=>/\s(\/[^\n]+)$/.exec(line)?.[1]).filter(p=>p&&/\/lib(?:gnutls|tasn1)\.so\./.test(p)))];for(const p of selected)paths.add(p);if(selected.length)processes.push({identity:before,paths:selected});}catch(error){if(error.code!=='ENOENT'&&error.code!=='ESRCH')throw error;}}
 const snapshot={membership,processes,paths:[...paths]};if(requireMatch)admitLoadedCrypto(snapshot.paths,expected);return snapshot;
}
