// Read only pinned provider source from a currently admitted synthetic server.
import{readFileSync,writeFileSync}from'node:fs';
import{createHash,randomBytes}from'node:crypto';
import{ProviderEngine}from'./engine.mjs';
import{createProviderCgroup,prepareLauncherCgroup,snapshotProviderCgroup,requireDrainedCgroup}from'./cgroup.mjs';
import{safeContainerProjection,requireContainerIdentity}from'./container-identity.mjs';
import{readProcessIdentity,sameProcessIdentity}from'./launch.mjs';
const sourceKind=process.argv[3]??'crypto',sourceFile={crypto:'/authentik/crypto/api.py',users:'/authentik/core/api/users.py'}[sourceKind];if(!sourceFile)throw Error('closed provider source selector');
const root='/var/tmp/rom-010-authentik-20261007',run=root+'/run/volume',path=process.argv[2];
if(!new RegExp(`^${run}/evidence/authentik-ready-[a-f0-9]{32}\\.json$`).test(path))throw Error('owned readiness required');
const ready=JSON.parse(readFileSync(path)),server=ready.containers.find(v=>v.role==='server').observation,expected=ready.records.containers.find(v=>v.expected.id===server.id).expected,group=prepareLauncherCgroup(createProviderCgroup()),engine=new ProviderEngine(JSON.parse(readFileSync(root+'/overlay/bounded-engine-config.json')),group,run),nonce=randomBytes(12).toString('hex'),record={status:'starting',ready:path,server_id:server.id,artifact_admission:false};
try{
 const values=JSON.parse(await engine.command(['inspect',server.id]));if(values.length!==1)throw Error('ambiguous exact server');requireContainerIdentity(expected,safeContainerProjection(values[0]));if(!sameProcessIdentity(server.birth,readProcessIdentity(server.birth.pid))||!sameProcessIdentity(server.conmon,readProcessIdentity(server.conmon.pid)))throw Error('server birth changed');
 const source=await engine.command(['exec',server.id,'python','-c',`from pathlib import Path; p=Path('${sourceFile}'); print(p.read_text())`]);const sourcePath=run+'/evidence/provider-'+sourceKind+'-source-'+nonce+'.txt';writeFileSync(sourcePath,source,{flag:'wx',mode:0o600});record.source={path:sourcePath,sha256:createHash('sha256').update(source).digest('hex'),bytes:Buffer.byteLength(source)};record.status='captured';
}catch(error){record.status='failed';record.error=error.message;process.exitCode=1;}
finally{
 await engine.drain();let drained=false;for(let attempt=0;attempt<20;attempt++){record.group=snapshotProviderCgroup(group);try{requireDrainedCgroup(record.group.cgroup_events);drained=true;break;}catch{await new Promise(resolve=>setTimeout(resolve,100));}}record.drained=drained;if(!drained)process.exitCode=1;record.engine_records=engine.records();const evidence=run+'/evidence/provider-source-inspect-'+nonce+'.json';writeFileSync(evidence,JSON.stringify(record,null,2),{flag:'wx',mode:0o600});console.log(JSON.stringify({status:record.status,evidence,source:record.source,drained}));
}
