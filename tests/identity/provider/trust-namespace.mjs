import {readFileSync,writeFileSync,readlinkSync,realpathSync,lstatSync,openSync,closeSync,fstatSync,fsyncSync,constants} from 'node:fs';
import {createHash,randomBytes} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {OwnedLauncher,readProcessIdentity,sameProcessIdentity} from './launch.mjs';
import {requireOwnedBrowserMembership} from './crypto-wrapper.mjs';
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
export const namespaceTools=Object.freeze({unshare:'/nix/store/1jasg83hbsd6m0gymnc6c740v0jl7mg8-util-linux-2.39.4-bin/bin/unshare',mount:'/nix/store/im20pazfdyswmha4j263xkvgqp6ivg3r-util-linux-2.39.4-mount/bin/mount'});
export function requirePrivateTrustNamespace(parent,current,mountinfo){
 if(!/^mnt:\[[0-9]+\]$/.test(parent)||!/^mnt:\[[0-9]+\]$/.test(current)||parent===current)throw Error('fresh private mount namespace required');
 const root=mountinfo.split('\n').filter(line=>line.split(' ')[4]==='/');if(root.length!==1||root[0].split(' - ')[0].split(' ').slice(6).some(field=>/^(?:shared|master|propagate_from):/.test(field)))throw Error('private root propagation required');
}
export function requireReadOnlyTrustMount(raw){
 const lines=raw.split('\n').filter(line=>line.split(' ')[4]==='/etc/ssl/certs');
 if(lines.length!==1||!lines[0].split(' ')[5].split(',').includes('ro')||lines[0].split(' - ')[0].split(' ').slice(6).some(field=>/^(?:shared|master|propagate_from):/.test(field)))throw Error('read-only private trust mount required');
}
export async function enterPrivateTrustView(configuration){
 const source=configuration.trust_view,root='/var/tmp/rom-010-authentik-20261007/run/volume';
 if(typeof source!=='string'||!source.startsWith(`${root}/private/tls-`)||!source.endsWith('/trust-view')||realpathSync(source)!==source)throw Error('owned trust directory required');
 const directory=lstatSync(source),file=lstatSync(`${source}/ca-certificates.crt`);if(!directory.isDirectory()||(directory.mode&0o777)!==0o700||!file.isFile()||file.nlink!==1||(file.mode&0o777)!==0o600||file.uid!==process.getuid()||directory.dev!==lstatSync(root).dev||file.dev!==directory.dev||file.size>65536)throw Error('owned trust bundle required');
 const birth=readProcessIdentity(process.pid),namespace=readlinkSync('/proc/self/ns/mnt'),membership=readFileSync('/proc/self/cgroup','utf8').trim();requireOwnedBrowserMembership(membership);
 requirePrivateTrustNamespace(configuration.parent_mount_namespace,namespace,readFileSync('/proc/self/mountinfo','utf8'));
 if(!birth||realpathSync('/etc/ssl/certs')!=='/etc/ssl/certs'||!lstatSync('/etc/ssl/certs').isDirectory())throw Error('real trust mountpoint required');
 const launcher=new OwnedLauncher(),record={schema:'rom-private-trust-mount-v1',birth,namespace,parent_namespace:configuration.parent_mount_namespace,membership,source,source_inode:directory.ino,source_device:directory.dev,bundle_inode:file.ino,bundle_sha256:hash(readFileSync(`${source}/ca-certificates.crt`)),processes:[],production_dependency_admission:false};
 async function mount(args){
  const nonce=randomBytes(12).toString('hex'),parent=source.slice(0,source.lastIndexOf('/')),path=`${parent}/mount-ack-${nonce}`,commandResult=`${parent}/mount-command-${nonce}.json`;writeFileSync(path,'',{flag:'wx',mode:0o600});const s=lstatSync(path),ack={path,device:s.dev,inode:s.ino,uid:s.uid};
  const child=launcher.launch(process.execPath,[fileURLToPath(new URL('./mount-command-run.mjs',import.meta.url)),JSON.stringify({ack,result:commandResult,args,namespace,membership})],{timeoutMs:10000,graceMs:1000,drainMs:1000,outputBytes:65536,admitIdentity:id=>{if(!sameProcessIdentity(birth,readProcessIdentity(process.pid))||readlinkSync(`/proc/${id.pid}/ns/mnt`)!==namespace||readFileSync(`/proc/${id.pid}/cgroup`,'utf8').trim()!==membership)throw Error('owned trust mount process changed');}});child.child.stdout.on('data',()=>{});child.child.stderr.on('data',()=>{});
  const target=await child.targetStarted;
  if(target){const fd=openSync(path,constants.O_WRONLY|constants.O_NOFOLLOW);try{const now=fstatSync(fd);if(now.ino!==ack.inode||now.dev!==ack.device||now.size!==0)throw Error('mount acknowledgement changed');writeFileSync(fd,'ready\n');fsyncSync(fd);}finally{closeSync(fd);}}
  const result=await child.closed;await child.physicalClose;const command=target?JSON.parse(readFileSync(commandResult,'utf8')):null;record.processes.push({wrapper:child.identity,target,result,args,command});if(!target||result.exit_code!==0||!result.drained||command.exit_code!==0)throw Error('private trust mount failed');
 }
 try{await mount(['--bind',source,'/etc/ssl/certs']);await mount(['--bind','-o','remount,ro','/etc/ssl/certs']);record.mountinfo=readFileSync('/proc/self/mountinfo','utf8');requireReadOnlyTrustMount(record.mountinfo);if(hash(readFileSync('/etc/ssl/certs/ca-certificates.crt'))!==record.bundle_sha256||lstatSync('/etc/ssl/certs').ino!==directory.ino||!sameProcessIdentity(birth,readProcessIdentity(process.pid)))throw Error('private trust projection changed');record.status='read-only-owned-trust-view';}
 catch(error){record.status='trust-namespace-prerequisite-failed';throw error;}
 finally{record.drain=await launcher.drain();writeFileSync(configuration.namespace_result,JSON.stringify(record,null,2)+'\n',{flag:'wx',mode:0o600});}
 return record;
}
