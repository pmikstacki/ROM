// Fixed private TLS supplement acquisition. No installation, scripts or global store writes.
import {readFileSync,writeFileSync,mkdirSync,readdirSync,lstatSync,realpathSync,statfsSync} from 'node:fs';
import {createHash,randomBytes} from 'node:crypto';
import {OwnedLauncher} from './launch.mjs';
import {createProviderCgroup,admitOwnedWrapper,snapshotProviderCgroup,requireDrainedCgroup} from './cgroup.mjs';
import {requirePackageDigest,debianDataMember,admitToolTar} from './debian.mjs';
import {cryptoPackages,selectCryptoLibrary} from './crypto-profile.mjs';
const acquisition='/var/tmp/rom-010-authentik-20261007/overlay/acquisition',run='/var/tmp/rom-010-authentik-20261007/run/volume',MIB=1024**2;
const xz='/nix/store/livin0fi0bzqnw9fqyx8acwbd4z4qrp9-xz-5.6.3-bin/bin/xz';
const loader='/nix/store/5m9amsvvh2z8sl7jrnc87hzy21glw6k1-glibc-2.40-66/lib/ld-linux-x86-64.so.2';
const prior=`${run}/evidence/gio-verified-ca8ffbb6ca5c63505ee4ac82/result.json`;
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
function stored(path,allowLinks=false){let bytes=0,entries=0;function visit(p){if(++entries>100000)throw Error('crypto entry cap');const s=lstatSync(p);if(s.isDirectory())for(const n of readdirSync(p))visit(`${p}/${n}`);else if(s.isFile()||(allowLinks&&s.isSymbolicLink()))bytes+=s.size;else throw Error('crypto storage type');}visit(path);return bytes;}
function totals(){let crypto=0,gio=0;for(const name of readdirSync(acquisition)){if(/^crypto-tools-[a-f0-9]{24}$/.test(name))crypto+=stored(`${acquisition}/${name}`);if(/^gio-tools-[a-f0-9]{24}$/.test(name))gio+=stored(`${acquisition}/${name}`,true);}return{crypto,gio};}
export async function acquirePrivateCrypto(){
 if(realpathSync(acquisition)!==acquisition||lstatSync(acquisition).dev===lstatSync('/var/tmp').dev)throw Error('hard crypto filesystem required');
 const gio=JSON.parse(readFileSync(prior,'utf8'));if(gio.status!=='signed-historical-gio-verified')throw Error('verified GIO prerequisite');
 for(const dependency of gio.dependencies)if(hash(readFileSync(dependency.path))!==dependency.sha256)throw Error('GIO dependency changed');
 const nonce=randomBytes(12).toString('hex'),directory=`${acquisition}/crypto-tools-${nonce}`,evidence=`${run}/evidence/crypto-tools-${nonce}`;
 mkdirSync(directory,{mode:0o700});mkdirSync(`${directory}/lib`,{mode:0o700});mkdirSync(evidence,{mode:0o700});
 const group=createProviderCgroup(),launcher=new OwnedLauncher(),deadline=Date.now()+600000;
 const record={schema:'rom-private-patched-crypto-supplement-v1',status:'starting',directory,evidence,prior,prior_sha256:hash(readFileSync(prior)),deadline_ms:600000,crypto_byte_cap:32*MIB,aggregate_gio_crypto_byte_cap:256*MIB,packages:[],metadata:[],processes:[],production_dependency_admission:false,remaining_advisory_review:['GLib2.82.1','browser runtimes','p11-kit0.25.5 RPC applicability']};
 let stage='metadata';
 function capacity(extra=0){const t=totals(),fs=statfsSync(acquisition);if(t.crypto+extra>32*MIB||t.crypto+t.gio+extra>256*MIB||Date.now()>deadline||Number(fs.bavail)*Number(fs.bsize)<1024*MIB)throw Error('crypto acquisition envelope');return t;}
 function save(path,bytes){capacity(bytes.length);writeFileSync(path,bytes,{flag:'wx',mode:0o600});}
 async function download(url,limit){const response=await fetch(url,{redirect:'error',signal:AbortSignal.timeout(Math.max(1,Math.min(30000,deadline-Date.now())))});if(!response.ok)throw Error('crypto official response rejected');let count=0;const chunks=[];for await(const bytes of response.body){count+=bytes.length;capacity(count);if(count>limit)throw Error('crypto response cap');chunks.push(Buffer.from(bytes));}return Buffer.concat(chunks);}
 async function command(executable,args,limit=16*MIB){capacity();const child=launcher.launch(executable,args,{timeoutMs:30000,graceMs:1000,drainMs:1000,outputBytes:limit,admitIdentity:id=>admitOwnedWrapper(group,id)});let count=0;const chunks=[];child.child.stdout.on('data',b=>{count+=b.length;if(count<=limit)chunks.push(b);});child.child.stderr.on('data',()=>{});const target=await child.targetStarted,result=await child.closed;await child.physicalClose;record.processes.push({executable,executable_sha256:hash(readFileSync(executable)),args,wrapper:child.identity,target,result,stdout_bytes:count});if(!target||result.exit_code!==0||!result.drained||count>limit)throw Error('crypto tool process failed');return Buffer.concat(chunks);}
 try{
  capacity(20*MIB);
  for(const p of cryptoPackages){
   const metadataRoot=`https://metadata.ftp-master.debian.org/changelogs/main/${p.directory}/${p.source}_${encodeURIComponent(p.version)}`;
   for(const [name,url]of[['copyright',`${metadataRoot}_copyright`],['changelog',`${metadataRoot}_changelog`],['source-descriptor',`https://deb.debian.org/debian/pool/main/${p.directory}/${p.source}_${p.version}.dsc`],['advisory',`https://security-tracker.debian.org/tracker/source-package/${p.source}`]]){
    const bytes=await download(url,256*1024);save(`${directory}/${p.name}-${name}.txt`,bytes);record.metadata.push({name,package:p.name,url,bytes:bytes.length,sha256:hash(bytes)});
   }
   stage=`download-${p.name}`;const url=`https://deb.debian.org/debian/pool/main/${p.directory}/${p.name}_${p.version}_amd64.deb`,bytes=await download(url,2*MIB);requirePackageDigest(bytes,p);save(`${directory}/${p.name}.deb`,bytes);
   stage=`archive-${p.name}`;const compressed=debianDataMember(bytes),path=`${directory}/${p.name}.data.tar.xz`;save(path,compressed);const tar=await command(xz,['--decompress','--stdout','--memlimit-decompress=256MiB',path]);const members=admitToolTar(tar);save(`${directory}/${p.name}.data.tar`,tar);const selected=selectCryptoLibrary(members,p);save(`${directory}/lib/${selected.filename}`,selected.bytes);record.packages.push({...p,url,source_member:selected.source,library_path:`${directory}/lib/${selected.filename}`,library_sha256:hash(selected.bytes),admitted_members:members.size});
  }
  stage='loader-compatibility';
  const directories=[`${directory}/lib`,...new Set(gio.dependencies.map(d=>d.path.slice(0,d.path.lastIndexOf('/'))))],libraryPath=directories.join(':');
  const bytes=await command(loader,['--library-path',libraryPath,'--list',gio.module.path],65536);save(`${directory}/loader-list.txt`,bytes);record.loader={path:loader,sha256:hash(readFileSync(loader)),library_path:libraryPath};record.dependencies=[];
  for(const line of bytes.toString('utf8').trim().split('\n')){if(line.trim().startsWith('linux-vdso.so.1'))continue;const path=/(?:=>\s+)?(\/[^\s]+)\s+\(/.exec(line)?.[1];if(!path||!directories.some(d=>path.startsWith(`${d}/`)))throw Error('unadmitted crypto loader dependency');record.dependencies.push({path,realpath:realpathSync(path),sha256:hash(readFileSync(path))});}
  for(const p of record.packages)if(!record.dependencies.some(d=>d.path===p.library_path))throw Error('patched crypto library not selected');
  record.status='acquired-patched-crypto-static-loader-compatible';
 }catch{record.status='crypto-prerequisite-failed';record.failure_stage=stage;process.exitCode=1;}
 finally{record.drain=await launcher.drain();record.group=snapshotProviderCgroup(group);requireDrainedCgroup(record.group.cgroup_events);record.storage=totals();record.finished=new Date().toISOString();writeFileSync(`${evidence}/result.json`,JSON.stringify(record,null,2)+'\n',{flag:'wx',mode:0o600});console.log(JSON.stringify({status:record.status,evidence,stored_bytes:record.storage.crypto,production_dependency_admission:false}));}
}
