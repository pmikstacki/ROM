import{providerLifetimeSeconds}from'./window-profile.mjs';
// Authoring-only container lifecycle. Keep secret-bearing inspect bodies in memory.
import{readFileSync,readlinkSync,realpathSync,statSync,writeFileSync}from'node:fs';
import{randomBytes}from'node:crypto';
import{OwnedLauncher,readProcessIdentity,sameProcessIdentity}from'./launch.mjs';
import{admitOwnedWrapper,snapshotProviderCgroup}from'./cgroup.mjs';
import{admitStoppedProvider,admitProviderRebirth}from'./restart-identity.mjs';
import{safeContainerProjection,requireContainerIdentity}from'./container-identity.mjs';

export class ProviderEngine {
 #engine;#group;#run;#launcher;#owned=new Map();#commands=[];#session=randomBytes(16).toString('hex');
 constructor(engine,group,runRoot){this.#engine=engine;this.#group=group;this.#run=runRoot;this.#launcher=new OwnedLauncher();snapshotProviderCgroup(group);if(!group.process_path)throw Error('provider launcher hierarchy required');}
 async command(args){
  const child=this.#launcher.launch(this.#engine.executable,[...this.#engine.args,...args],{timeoutMs:15000,outputBytes:1024*1024,cwd:this.#run,env:this.#engine.env,admitIdentity:identity=>admitOwnedWrapper(this.#group,identity)});
  let stdout='',stderr='',stderrBytes=0,count=0;child.child.stdout.on('data',b=>{count+=b.length;if(count<=1024*1024)stdout+=b;});child.child.stderr.on('data',b=>{stderrBytes+=b.length;if(stderrBytes<=1024*1024)stderr+=b;});
  const target=await child.targetStarted,result=await child.closed;await child.physicalClose;const stderrPath=`${this.#run}/private/engine-${this.#session}-${this.#commands.length}.stderr`;
  writeFileSync(stderrPath,stderr,{flag:'wx',mode:0o600});this.#commands.push({args,wrapper:child.identity,target,result,stderr_bytes:stderrBytes,private_stderr_path:stderrPath});
  if(result.exit_code!==0||!result.drained||count>1024*1024)throw Error('isolated provider engine command failed');return stdout;
 }
 async launch(role,{image,envFile,volumes,networkContainer,lifetimeSeconds=600}){
  lifetimeSeconds=providerLifetimeSeconds(lifetimeSeconds);
  if(!['postgres','server','worker'].includes(role)||this.#owned.size>=3||![image,envFile].every(x=>typeof x==='string'))throw Error('invalid provider launch');
  const approvedImage=role==='postgres'?'docker.io/library/postgres@sha256:1a66d744c1b459e13b05a8fca341da84cb63383e99ce262210efee5a319d4551':'ghcr.io/goauthentik/server@sha256:09782fe56675bc616a0324468f1698e2d9d83c978bc5e426686fb7563517a442';
  if(image!==approvedImage)throw Error('unadmitted provider image');
  if(realpathSync(envFile)!==envFile||!envFile.startsWith(`${this.#run}/private/`)||!statSync(envFile).isFile()||(statSync(envFile).mode&0o777)!==0o600)throw Error('private provider environment required');
  if(networkContainer!==undefined&&!this.#owned.has(networkContainer))throw Error('unowned provider namespace');
  const nonce=randomBytes(16).toString('hex'),memory=role==='postgres'?805306368:1610612736,quota=role==='postgres'?75000:150000;
  const args=['run','--detach','--pull=never',`--name=rom-identity-${role}-${nonce}`,`--label=org.rom.identity-fixture=${nonce}`,
   `--cidfile=${this.#run}/private/${role}-${nonce}.cid`,`--conmon-pidfile=${this.#run}/private/${role}-${nonce}.conmon`,
   `--cgroup-parent=${this.#group.membership}`,'--cgroups=no-conmon',`--memory=${memory}`,'--cpu-period=100000',`--cpu-quota=${quota}`,'--pids-limit=128',
   `--timeout=${lifetimeSeconds}`,'--stop-timeout=5','--restart=no','--image-volume=ignore','--log-driver=k8s-file','--log-opt=max-size=1mb','--http-proxy=false',
   `--network=${networkContainer?`container:${networkContainer}`:'none'}`,`--env-file=${envFile}`];
  for(const [source,destination]of volumes){if(realpathSync(source)!==source||!source.startsWith(`${this.#run}/private/volumes/`)||!statSync(source).isDirectory()||!['/data','/templates','/var/lib/postgresql/data'].includes(destination))throw Error('unowned provider volume');args.push('--mount',`type=bind,src=${source},dst=${destination}`);}
  args.push(image);if(role!=='postgres')args.push(role);
  const id=(await this.command(args)).trim();if(!/^[a-f0-9]{64}$/.test(id))throw Error('invalid created container identity');
  const expected={id,image:role==='postgres'?'81bd698b4594e751a3269e4dcd3e03a4a0ec0daf7b72e7aa1abd43cce9887542':'d4d6843a3c120bb1b809048ce8429e0e617f68a3991aaa0779d6f5bd802b34d9',nonce,cgroup:this.#group.membership,...(networkContainer?{network_container:networkContainer}:{})};
  this.#owned.set(id,{expected,conmonFile:`${this.#run}/private/${role}-${nonce}.conmon`});return this.observe(id);
 }
 async observe(id){
  const owned=this.#owned.get(id);if(!owned)throw Error('unowned provider container');const values=JSON.parse(await this.command(['inspect',id]));if(values.length!==1)throw Error('ambiguous provider container');const value=values[0],safe=safeContainerProjection(value);requireContainerIdentity(owned.expected,safe);
  const pid=readProcessIdentity(safe.pid);if(!pid)throw Error('provider init identity unavailable');const membership=readFileSync(`/proc/${pid.pid}/cgroup`,'utf8').trim();if(!membership.startsWith(`0::${this.#group.membership}/`))throw Error('provider init cgroup escaped');
  const conmon=readProcessIdentity(Number(readFileSync(owned.conmonFile,'utf8').trim()));if(!conmon)throw Error('provider monitor identity unavailable');const conmonGroup=readFileSync(`/proc/${conmon.pid}/cgroup`,'utf8').trim();if(!conmonGroup.startsWith(`0::${this.#group.membership}/`))throw Error('provider monitor cgroup escaped');
  if(owned.pid&&!sameProcessIdentity(owned.pid,pid))throw Error('provider init identity changed');owned.pid=pid;owned.conmon=conmon;
  return{...safe,birth:pid,conmon,proc_cgroup:membership,conmon_cgroup:conmonGroup,netns:readlinkSync(`/proc/${pid.pid}/ns/net`)};
 }
 async stop(id){const owned=this.#owned.get(id);if(!owned)throw Error('unowned provider stop');if(owned.paused){const values=JSON.parse(await this.command(['inspect',id]));if(values.length!==1)throw Error('ambiguous stopped provider');admitStoppedProvider(owned.expected,safeContainerProjection(values[0]));return;}await this.observe(id);if(!sameProcessIdentity(owned.pid,readProcessIdentity(owned.pid.pid)))throw Error('provider stop identity changed');await this.command(['stop','--time=5',id]);}
 async pause(id){const owned=this.#owned.get(id);if(!owned||owned.paused||(owned.restarts??0)>=2)throw Error('closed provider pause');const before=await this.observe(id);await this.stop(id);const values=JSON.parse(await this.command(['inspect',id]));if(values.length!==1)throw Error('ambiguous stopped provider');admitStoppedProvider(owned.expected,safeContainerProjection(values[0]));if(readProcessIdentity(before.birth.pid)&&sameProcessIdentity(before.birth,readProcessIdentity(before.birth.pid)))throw Error('stopped init remains live');owned.paused=before;return before;}
 async resume(id){const owned=this.#owned.get(id);if(!owned?.paused)throw Error('closed provider resume');const values=JSON.parse(await this.command(['inspect',id]));if(values.length!==1)throw Error('ambiguous stopped provider');admitStoppedProvider(owned.expected,safeContainerProjection(values[0]));const previous=owned.paused;await this.command(['start',id]);owned.history??=[];owned.history.push(previous);owned.pid=undefined;owned.conmon=undefined;const fresh=await this.observe(id);admitProviderRebirth(previous,fresh);owned.restarts=(owned.restarts??0)+1;owned.paused=null;return fresh;}
 records(){return{commands:this.#commands,containers:[...this.#owned.values()].map(x=>({expected:x.expected,pid:x.pid,conmon:x.conmon,history:x.history??[],paused:x.paused??null,restarts:x.restarts??0})),group:snapshotProviderCgroup(this.#group)};}
 async drain(){return this.#launcher.drain();}
}
