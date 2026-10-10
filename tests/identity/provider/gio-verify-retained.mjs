// Resume verification of the retained completed copy. Never rewrite its failed acquisition evidence.
import { readFileSync, writeFileSync, mkdirSync, realpathSync, lstatSync } from 'node:fs';
import { randomBytes, createHash } from 'node:crypto';
import { OwnedLauncher } from './launch.mjs';
import { createProviderCgroup, admitOwnedWrapper, snapshotProviderCgroup, requireDrainedCgroup } from './cgroup.mjs';
import { verifyGioClosure } from './gio-verification.mjs';
const run='/var/tmp/rom-010-authentik-20261007/run/volume';
const acquisition='/var/tmp/rom-010-authentik-20261007/overlay/acquisition';
const oldPath=`${run}/evidence/gio-tools-f6b340431389545cefafb753/result.json`;
const old=JSON.parse(readFileSync(oldPath,'utf8')),metadata=readFileSync(old.metadata);
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
if(old.status!=='gio-prerequisite-failed'||old.failure_stage!=='exact-closure-verification'||hash(metadata)!==old.metadata_sha256||hash(readFileSync(old.nix.path))!==old.nix.sha256||old.directory!==`${acquisition}/gio-tools-f6b340431389545cefafb753`)throw Error('retained isolated copy identity');
const nonce=randomBytes(12).toString('hex'),evidence=`${run}/evidence/gio-verified-${nonce}`,projection=`${old.directory}/modules-${nonce}`;mkdirSync(evidence,{mode:0o700});mkdirSync(projection,{mode:0o700});
const group=createProviderCgroup(),launcher=new OwnedLauncher();
const record={schema:'rom-retained-historical-gio-verification-v1',status:'starting',prior:oldPath,prior_sha256:hash(readFileSync(oldPath)),directory:old.directory,processes:[],production_dependency_admission:false};
const env={HOME:`${old.directory}/home`,XDG_CACHE_HOME:`${old.directory}/cache`,TMPDIR:`${old.directory}/tmp`,NIX_CONF_DIR:`${old.directory}/configuration`,NIX_USER_CONF_FILES:'',NIX_SSL_CERT_FILE:'/etc/ssl/certs/ca-certificates.crt',PATH:'/run/current-system/sw/bin:/usr/bin:/bin',LANG:'C',TZ:'UTC',NIX_CONFIG:`experimental-features = nix-command\ntrusted-public-keys = ${old.official_key}\nrequire-sigs = true\n`};
async function command(executable,args){const child=launcher.launch(executable,args,{timeoutMs:60000,graceMs:1000,drainMs:1000,outputBytes:8*1024**2,env,admitIdentity:identity=>admitOwnedWrapper(group,identity)}),stdout=[],stderr=[];child.child.stdout.on('data',bytes=>stdout.push(bytes));child.child.stderr.on('data',bytes=>stderr.push(bytes));const target=await child.targetStarted,result=await child.closed;await child.physicalClose;const i=record.processes.length;writeFileSync(`${evidence}/process-${i}-stdout.log`,Buffer.concat(stdout),{flag:'wx',mode:0o600});writeFileSync(`${evidence}/process-${i}-stderr.log`,Buffer.concat(stderr),{flag:'wx',mode:0o600});record.processes.push({executable,args,wrapper:child.identity,target,result});if(!target||result.exit_code!==0||!result.drained||result.signal!==null)throw Error('retained verification process');return Buffer.concat(stdout);}
try{
 const modulePath='/nix/store/pna9r6204grpyb4qsdfmdr9qjxsr6yhr-glib-networking-2.80.1';
 await command(old.nix.path,['store','verify','--store',old.target_store,'--recursive','--sigs-needed','1',modulePath]);
 const actual=JSON.parse((await command(old.nix.path,['path-info','--store',old.target_store,'--recursive','--json',modulePath])).toString());record.verified_closure=verifyGioClosure(JSON.parse(metadata),actual);
 const source=`${old.directory}/store-root${modulePath}/lib/gio/modules/libgiognutls.so`;if(!lstatSync(source).isFile()||realpathSync(source)!==source)throw Error('regular module');const path=`${projection}/libgiognutls.so`,bytes=readFileSync(source);writeFileSync(path,bytes,{flag:'wx',mode:0o600});record.module={source,path,sha256:hash(bytes),env_module_directory:projection,private_ca_environment:'NIX_SSL_CERT_FILE'};
 const loader='/nix/store/5m9amsvvh2z8sl7jrnc87hzy21glw6k1-glibc-2.40-66/lib/ld-linux-x86-64.so.2';const resolved=(await command(loader,['--list',path])).toString();const allowed=record.verified_closure.map(v=>v.path+'/');record.dependencies=[];
 for(const line of resolved.trim().split('\n')){if(line.trim().startsWith('linux-vdso.so.1 '))continue;const dependency=/(?:=>\s+)?(\/[^\s]+)\s+\(/.exec(line)?.[1];if(!dependency||!allowed.some(prefix=>dependency.startsWith(prefix)))throw Error('undeclared module dependency');record.dependencies.push({path:dependency,realpath:realpathSync(dependency),sha256:hash(readFileSync(dependency))});}
 record.status='signed-historical-gio-verified';
}catch{record.status='gio-verification-prerequisite-failed';process.exitCode=1;}
finally{record.drain=await launcher.drain();record.group=snapshotProviderCgroup(group);requireDrainedCgroup(record.group.cgroup_events);writeFileSync(`${evidence}/result.json`,JSON.stringify(record,null,2)+'\n',{flag:'wx',mode:0o600});console.log(JSON.stringify({status:record.status,evidence,production_dependency_admission:false}));}
