// Fresh provider cgroup only. Never alter a parent controller or move an unowned process.
import{mkdirSync,readFileSync,writeFileSync,realpathSync,statSync}from'node:fs';import{randomBytes}from'node:crypto';import{readProcessIdentity,sameProcessIdentity}from'./launch.mjs';
export function requireProviderLimits(value){if(value?.cpu!=='400000 100000'||value.memory!=='4294967296'||value.pids!=='512')throw Error('provider cgroup limits mismatch');}
export function requireMembership(raw,expected){if(raw.trim()!==`0::${expected}`)throw Error('provider cgroup membership mismatch');}
export function requireDrainedCgroup(raw) {
 if(typeof raw!=='string')throw Error('cgroup drain evidence malformed');
 const fields=raw.trim().split('\n').map(line=>line.trim().split(/\s+/));
 if(fields.length!==2||new Set(fields.map(([key])=>key)).size!==2||fields.some(([key,value,...rest])=>!['populated','frozen'].includes(key)||!['0','1'].includes(value)||rest.length)||!fields.some(([key,value])=>key==='populated'&&value==='0'))throw Error('cgroup drain requires no live descendants');
}
const controls={cpu:'cpu.max',memory:'memory.max',pids:'pids.max'};
export function createProviderCgroup(){const parent='/sys/fs/cgroup';const available=new Set(readFileSync(`${parent}/cgroup.subtree_control`,'utf8').trim().split(/\s+/));if(!['cpu','memory','pids'].every(x=>available.has(x)))throw Error('provider cgroup controllers unavailable');
 const name=`rom-identity-${randomBytes(16).toString('hex')}`,path=`${parent}/${name}`;mkdirSync(path);const inode=statSync(path).ino;for(const[key,file]of Object.entries(controls))writeFileSync(`${path}/${file}`,{cpu:'400000 100000',memory:'4294967296',pids:'512'}[key]);const group={path,membership:`/${name}`,inode};snapshotProviderCgroup(group);return Object.freeze(group);}
export function snapshotProviderCgroup(group){if(!/^\/sys\/fs\/cgroup\/rom-identity-[a-f0-9]{32}$/.test(group.path)||realpathSync(group.path)!==group.path||statSync(group.path).ino!==group.inode)throw Error('provider cgroup identity mismatch');const limits=Object.fromEntries(Object.entries(controls).map(([key,file])=>[key,readFileSync(`${group.path}/${file}`,'utf8').trim()]));requireProviderLimits(limits);return{...group,limits,memory_events:readFileSync(`${group.path}/memory.events`,'utf8'),pids_events:readFileSync(`${group.path}/pids.events`,'utf8'),cgroup_events:readFileSync(`${group.path}/cgroup.events`,'utf8')};}
export function prepareLauncherCgroup(group) {
 snapshotProviderCgroup(group);
 if(readFileSync(`${group.path}/cgroup.procs`,'utf8').trim())throw Error('provider cgroup has internal processes');
 writeFileSync(`${group.path}/cgroup.subtree_control`,'+cpu +memory +pids');
 const path=`${group.path}/launcher`;mkdirSync(path,{mode:0o700});
 return Object.freeze({...group,process_path:path,process_inode:statSync(path).ino,process_membership:`${group.membership}/launcher`});
}
export function admitOwnedWrapper(group,identity){snapshotProviderCgroup(group);const path=group.process_path??group.path,membership=group.process_membership??group.membership;if(group.process_path&&(path!==`${group.path}/launcher`||realpathSync(path)!==path||statSync(path).ino!==group.process_inode))throw Error('provider launcher cgroup identity mismatch');if(identity.pid!==identity.group||identity.pid!==identity.session||!sameProcessIdentity(identity,readProcessIdentity(identity.pid)))throw Error('provider cgroup process ownership mismatch');writeFileSync(`${path}/cgroup.procs`,String(identity.pid));if(!sameProcessIdentity(identity,readProcessIdentity(identity.pid)))throw Error('provider cgroup process identity changed');requireMembership(readFileSync(`/proc/${identity.pid}/cgroup`,'utf8'),membership);return{identity,cgroup:membership};}
