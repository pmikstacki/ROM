// Installed public-source acceptance against a caller-supplied native fixture binary.
import { cpSync, mkdirSync, readFileSync, writeFileSync, existsSync, lstatSync, realpathSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { isDeepStrictEqual } from 'node:util';
import { reserveOutput, sourceManifest, hashBytes, assertLockedGraph, browserOutcomes } from '../public-controls/verification.mjs';
import { assertStudioArchiveFile, extractStudioArchive, sourcePrefix } from '../public-controls/archive-admission.mjs';
import { sourceCheckoutHead } from './source-metadata.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)),'../../..'), studio = join(root,'studio');
const verifierHash=hashBytes(readFileSync(fileURLToPath(import.meta.url)));
const args=process.argv.slice(2);let requestedOutput,adapter,binary,port=43282,sourceArchive,sourceDirectory,hostProvenance,admit=false,prepareOnly=false;
for(let i=0;i<args.length;i++) {
  const flag=args[i];
  if(['--adapter','--host-binary','--port','--source-archive','--source-dir','--host-provenance'].includes(flag)) {
    if(!args[i+1]||args[i+1].startsWith('--'))throw Error(`${flag} requires a value`);const value=args[++i];
    if(flag==='--adapter')adapter=value;if(flag==='--host-binary')binary=resolve(value);if(flag==='--port')port=Number(value);if(flag==='--source-archive')sourceArchive=resolve(value);if(flag==='--source-dir')sourceDirectory=resolve(value);if(flag==='--host-provenance')hostProvenance=resolve(value);
  } else if(flag==='--admit')admit=true;
  else if(flag==='--prepare-only')prepareOnly=true;
  else if(flag.startsWith('--')||requestedOutput)throw Error('invalid recovery verifier arguments');else requestedOutput=flag;
}
if(!['sqlite','redb'].includes(adapter)||(!prepareOnly&&(!binary||!lstatSync(binary).isFile())))throw Error('supported adapter and regular native host binary required');
if(!Number.isInteger(port)||port<1024||port>65535)throw Error('invalid recovery port');
if(sourceArchive&&sourceDirectory)throw Error('select one source input');
if(admit&&prepareOnly)throw Error('preparation cannot admit a release');
if(admit&&!sourceArchive&&!sourceDirectory)throw Error('admission requires supplied source');
hostProvenance ??= binary ? join(dirname(binary),'source.json') : undefined;
if(admit&&(!hostProvenance||!existsSync(hostProvenance)))throw Error('admission requires frozen native compile provenance');
const output=reserveOutput(requestedOutput),commands=[],fixture=join(output,'consumer'),archive=join(output,'studio-source.tar.gz');
const sourceMode=sourceArchive?'release_artifact':sourceDirectory?'extracted_release_source':'authoring_checkout';
function run(program,argv,cwd,label,env=process.env) {
  const result=spawnSync(program,argv,{cwd,env,encoding:'utf8',timeout:180000,maxBuffer:8*1024*1024});const record={program,argv,cwd,exit_code:result.status,signal:result.signal,error:result.error?.message};commands.push(record);
  writeFileSync(join(output,label+'.log'),JSON.stringify(record)+'\n'+(result.stdout??'')+(result.stderr??''),{flag:'wx'});
  if(result.status!==0||result.error)throw Error(`${label} failed; ${join(output,label+'.log')}`);return result;
}
try {
  if(sourceArchive){assertStudioArchiveFile(sourceArchive);cpSync(sourceArchive,archive);}
  else {
    const source=sourceDirectory??studio,stage=join(output,'source-stage'),candidate=join(stage,sourcePrefix);mkdirSync(candidate,{recursive:true});
    for(const file of ['package.json','package-lock.json','src'])cpSync(join(source,file),join(candidate,file),{recursive:true});
    for(const file of ['LICENSE','THIRD_PARTY_NOTICES.md'])cpSync(existsSync(join(source,file))?join(source,file):join(dirname(source),file),join(candidate,file));
    sourceManifest(candidate);run('tar',['-czf',archive,'-C',stage,sourcePrefix],stage,'archive');
  }
  const archiveHash=hashBytes(readFileSync(archive)),extractParent=join(output,'source-extraction');mkdirSync(extractParent);
  const extracted=join(output,'extracted');cpSync(extractStudioArchive(archive,extractParent),extracted,{recursive:true});const expected=sourceManifest(extracted);
  cpSync(join(studio,'tests/mutation-recovery/consumer'),fixture,{recursive:true});mkdirSync(join(output,'runtime'));for(const file of ['proxy.mjs','server.mjs'])cpSync(join(studio,'tests/mutation-recovery',file),join(output,'runtime',file));
  const lockPath=join(fixture,'package-lock.json'),frozen=JSON.parse(readFileSync(lockPath,'utf8')),producer=JSON.parse(readFileSync(join(extracted,'package.json'),'utf8')),local=frozen.packages['node_modules/rom-studio'];
  if(local.version!==producer.version||!isDeepStrictEqual(local.dependencies,producer.dependencies))throw Error('frozen dependency metadata differs from supplied source');
  run('npm',['ci','--prefer-offline','--install-links','--no-audit','--no-fund'],fixture,'install');assertLockedGraph(frozen,JSON.parse(readFileSync(lockPath,'utf8')));
  const installed=join(fixture,'node_modules/rom-studio');if(lstatSync(installed).isSymbolicLink()||!realpathSync(installed).startsWith(realpathSync(fixture)+'/'))throw Error('installed producer must be physical consumer-local source');
  if(!isDeepStrictEqual(expected,sourceManifest(installed)))throw Error('installed source differs');
  for(const [path,entry]of Object.entries(frozen.packages)){if(!path||!entry.version)continue;const file=join(fixture,path,'package.json');if(!existsSync(file)&&entry.optional)continue;if(!existsSync(file)||JSON.parse(readFileSync(file,'utf8')).version!==entry.version)throw Error(`realized dependency drift ${path}`);}
  run('npm',['run','check'],fixture,'type-check');run('npm',['run','build'],fixture,'build');run('npm',['run','build:http'],fixture,'build-http');
  if(prepareOnly){writeFileSync(join(output,'prepared.json'),JSON.stringify({completed:false,prepared:true,adapter,archive_sha256:archiveHash,source_files:expected,installed_source_matches:true,commands},null,2)+'\n');console.log(JSON.stringify({prepared:true,acceptance_complete:false,output}));process.exit(0);}
  const environment={...process.env,ROM_RECOVERY_HOST:binary,ROM_RECOVERY_ADAPTER:adapter,ROM_RECOVERY_EVIDENCE:output,ROM_RECOVERY_PORT:String(port)};
  run('node',['http-dist/http.js'],fixture,'http',{...environment,ROM_RECOVERY_DATABASE:join(output,adapter+'-http.db')});
  if(!process.env.ROM_WEBKIT_EXECUTABLE)throw Error('recovery acceptance requires an actual WebKit executable');
  const browsers=run('npm',['exec','--offline','--','playwright','test','--config','tests/playwright.config.ts','--reporter=json'],fixture,'browser',{...environment,ROM_RECOVERY_DATABASE:join(output,adapter+'-browser.db')});
  const report=JSON.parse(browsers.stdout),outcomes=browserOutcomes(report);writeFileSync(join(output,'browser-results.json'),JSON.stringify(report,null,2)+'\n');
  for(const engine of ['chromium','webkit'])if(outcomes[engine]?.length!==5||outcomes[engine].some(status=>status!=='passed'))throw Error(`missing or failed ${engine} recovery cases`);
  if(hashBytes(readFileSync(archive))!==archiveHash||!isDeepStrictEqual(expected,sourceManifest(installed)))throw Error('source changed during acceptance');
  let nativeCompile; if(hostProvenance&&existsSync(hostProvenance)){nativeCompile=JSON.parse(readFileSync(hostProvenance,'utf8'));if(nativeCompile.compile_exit_code!==0||nativeCompile.binary_sha256!==hashBytes(readFileSync(binary)))throw Error('native compile provenance does not match binary');cpSync(hostProvenance,join(output,'native-provenance.json'));}
  const fixtureFiles={};function recordFixture(base,path){const full=join(base,path);if(lstatSync(full).isDirectory()){for(const name of readdirSync(full))recordFixture(base,join(path,name));}else fixtureFiles[(base===fixture?'consumer/':'runtime/')+path]=hashBytes(readFileSync(full));}
  for(const path of ['src','tests','package.json','package-lock.json','tsconfig.json','vite.config.ts','index.html','dist','http-dist'])recordFixture(fixture,path);for(const file of ['proxy.mjs','server.mjs'])recordFixture(join(output,'runtime'),file);
  const nativeSources={};function recordNative(path){for(const name of readdirSync(path)){const file=join(path,name);if(lstatSync(file).isDirectory())recordNative(file);else nativeSources[file.slice(root.length+1)]=hashBytes(readFileSync(file));}}recordNative(join(root,'tests/recovery-host'));
  const result={head:sourceCheckoutHead(root,sourceMode),fixture_source_sha256:nativeSources,consumer_fixture_sha256:fixtureFiles,verifier_sha256:verifierHash,completed:true,admitted:admit,adapter,source_mode:sourceMode,source_input:sourceArchive??sourceDirectory??studio,archive_sha256:archiveHash,source_files:expected,installed_source_matches:true,installed_package:installed,consumer_lock_sha256:hashBytes(readFileSync(lockPath)),producer_lock_sha256:hashBytes(readFileSync(join(extracted,'package-lock.json'))),native_binary:binary,native_binary_sha256:hashBytes(readFileSync(binary)),native_compile_lock_sha256:nativeCompile?.files?.['Cargo.lock'],native_provenance_sha256:nativeCompile?hashBytes(readFileSync(hostProvenance)):null,current_root_lock_sha256:hashBytes(readFileSync(join(root,'Cargo.lock'))),browser_outcomes:outcomes,http:JSON.parse(readFileSync(join(output,'http-results.json'),'utf8')),commands,limits:['Disposable fixture authority, not production authentication/provider integration.','Process restart and strict IndexedDB transaction completion are not power-loss certification.','Before-dispatch storage refusal is tested; killing an accepted server mutation before commit is not certified.']};
  writeFileSync(join(output,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({completed:true,adapter,output}));
} catch(error) {writeFileSync(join(output,'failure.json'),JSON.stringify({completed:false,error:error.message,adapter,commands},null,2)+'\n');console.error(error.message);process.exitCode=1;}
