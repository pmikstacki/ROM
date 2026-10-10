// Locked installed-source admission for the actual Studio App composition.
import {cpSync,mkdirSync,readFileSync,writeFileSync,lstatSync,existsSync,realpathSync,readdirSync} from 'node:fs';
import {resolve,dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {isDeepStrictEqual} from 'node:util';
import {reserveOutput,sourceManifest,hashBytes,assertLockedGraph,browserOutcomes} from '../public-controls/verification.mjs';
import {assertStudioArchiveFile,extractStudioArchive} from '../public-controls/archive-admission.mjs';
const here=dirname(fileURLToPath(import.meta.url)),studio=resolve(here,'../..');
const args=process.argv.slice(2);let outputPath,sourceDirectory,sourceArchive,binary,adapter,hostProvenance,prepare=false,admit=false,scenario,port=43282,budgetMs=600000;
for(let i=0;i<args.length;i++) {
 const key=args[i];if(key==='--prepare-only')prepare=true;else if(key==='--admit')admit=true;
 else if(['--output','--source-dir','--source-archive','--host','--host-provenance','--adapter','--port','--budget-ms','--scenario'].includes(key)) {
  const value=args[++i];if(!value||value.startsWith('--'))throw Error('missing argument');
  if(key==='--output')outputPath=value;else if(key==='--source-dir')sourceDirectory=resolve(value);else if(key==='--source-archive')sourceArchive=resolve(value);else if(key==='--host')binary=resolve(value);else if(key==='--host-provenance')hostProvenance=resolve(value);else if(key==='--adapter')adapter=value;else if(key==='--port')port=Number(value);else if(key==='--scenario')scenario=value;else budgetMs=Number(value);
 } else throw Error('invalid App verifier argument');
}
if(admit)throw Error('App release admission remains disabled pending root-reviewed complete native source witness');
if(sourceArchive&&sourceDirectory)throw Error('select one supplied source');
if(![43282,43283].includes(port)||!Number.isSafeInteger(budgetMs)||budgetMs<1000||budgetMs>600000)throw Error('invalid fixture limits');
if(admit&&(prepare||(!sourceArchive&&!sourceDirectory)))throw Error('admission requires supplied release source and runtime');
if(!prepare&&(!binary||!['sqlite','redb'].includes(adapter)))throw Error('runtime requires fixture host and sqlite/redb adapter');
const output=reserveOutput(outputPath),consumer=join(output,'consumer'),extracted=join(output,'extracted'),commands=[];
const sourceMode=sourceArchive?'release_artifact':sourceDirectory?'extracted_release_source':'authoring_checkout';
function run(program,argv,label,env=process.env) {
 const result=spawnSync(program,argv,{cwd:consumer,env,encoding:'utf8',timeout:label==='browser'?budgetMs:180000,maxBuffer:8*1024*1024});
 const record={program,args:argv,exit_code:result.status,signal:result.signal,error:result.error?.message};commands.push(record);
 writeFileSync(join(output,label+'.log'),JSON.stringify(record)+'\n'+(result.stdout??'')+(result.stderr??''));
 if(result.status!==0||result.error)throw Error(label+' failed');return result;
}
try {
 if(sourceArchive){assertStudioArchiveFile(sourceArchive);mkdirSync(join(output,'source-extraction'));cpSync(extractStudioArchive(sourceArchive,join(output,'source-extraction')),extracted,{recursive:true});}
 else {mkdirSync(extracted);const source=sourceDirectory??studio;for(const name of ['src','package.json','package-lock.json'])cpSync(join(source,name),join(extracted,name),{recursive:true});for(const name of ['LICENSE','THIRD_PARTY_NOTICES.md'])cpSync(existsSync(join(source,name))?join(source,name):join(dirname(source),name),join(extracted,name));}
 const fixtureFiles={};function recordFixture(base,path){const full=join(base,path);if(lstatSync(full).isDirectory()){for(const name of readdirSync(full))recordFixture(base,join(path,name));}else fixtureFiles[path]=hashBytes(readFileSync(full));}
 for(const path of ['consumer','proxy.mjs','server.mjs','verify.mjs'])recordFixture(here,path);
 writeFileSync(join(output,'fixture-source.json'),JSON.stringify({files:fixtureFiles,dependency_proxy_sha256:hashBytes(readFileSync(join(here,'../mutation-recovery/proxy.mjs')))},null,2));
 const expected=sourceManifest(extracted);writeFileSync(join(output,'source.json'),JSON.stringify({source_mode:sourceMode,admitted:false,files:expected},null,2));
 cpSync(join(here,'consumer'),consumer,{recursive:true});
 const lockPath=join(consumer,'package-lock.json'),frozen=JSON.parse(readFileSync(lockPath)),producer=JSON.parse(readFileSync(join(extracted,'package.json')));
 if(!isDeepStrictEqual(frozen.packages['node_modules/rom-studio'].dependencies,producer.dependencies)||frozen.packages['node_modules/rom-studio'].version!==producer.version)throw Error('producer metadata differs from frozen graph');
 run('npm',['ci','--prefer-offline','--install-links','--no-audit','--no-fund'],'install');assertLockedGraph(frozen,JSON.parse(readFileSync(lockPath)));
 const installed=join(consumer,'node_modules/rom-studio');if(lstatSync(installed).isSymbolicLink()||!realpathSync(installed).startsWith(realpathSync(consumer)+'/'))throw Error('linked source forbidden');
 if(!isDeepStrictEqual(expected,sourceManifest(installed)))throw Error('installed source mismatch');
 for(const [path,entry]of Object.entries(frozen.packages)){if(!path||!entry.version)continue;const file=join(consumer,path,'package.json');if(!existsSync(file)&&entry.optional)continue;if(!existsSync(file)||JSON.parse(readFileSync(file)).version!==entry.version)throw Error('realized graph differs: '+path);}
 run('npm',['run','check'],'type-check');run('npm',['run','build'],'build');
 let outcomes,native;
 if(!prepare){
  if(!process.env.ROM_WEBKIT_EXECUTABLE)throw Error('actual locked WebKit executable required');
  if(hostProvenance){native=JSON.parse(readFileSync(hostProvenance));if(native.compile_exit_code!==0||native.binary_sha256!==hashBytes(readFileSync(binary)))throw Error('native binary provenance mismatch');cpSync(hostProvenance,join(output,'native-provenance.json'));}
  if(admit&&!native)throw Error('release runtime requires frozen native compile provenance');
  mkdirSync(join(output,'runtime/app'),{recursive:true});mkdirSync(join(output,'runtime/mutation-recovery'));
  for(const name of ['proxy.mjs','server.mjs'])cpSync(join(here,name),join(output,'runtime/app',name));cpSync(join(here,'../mutation-recovery/proxy.mjs'),join(output,'runtime/mutation-recovery/proxy.mjs'));
  const env={...process.env,ROM_RECOVERY_HOST:binary,ROM_RECOVERY_ADAPTER:adapter,ROM_RECOVERY_DATABASE:join(output,adapter+'.db'),ROM_RECOVERY_EVIDENCE:output,ROM_RECOVERY_PORT:String(port),ROM_APP_BUDGET_MS:String(budgetMs),ROM_APP_TEST_GREP:scenario??''};
  const browser=run('npm',['exec','--offline','--','playwright','test','--config','tests/playwright.config.ts','--reporter=json'],'browser',env);
  const report=JSON.parse(browser.stdout);writeFileSync(join(output,'browser-results.json'),JSON.stringify(report,null,2));outcomes=browserOutcomes(report);
  for(const engine of ['chromium','webkit'])if(outcomes[engine]?.length!==(scenario?1:10)||outcomes[engine].some(status=>status!=='passed'))throw Error('nine actual App cases plus maintained-startup diagnostic required: '+engine);
 }
 if(!isDeepStrictEqual(expected,sourceManifest(installed)))throw Error('installed source changed');
 const result={completed:!scenario,diagnostic_scenario:scenario,prepared:true,admitted:admit,source_mode:sourceMode,source_files:expected,fixture_source_sha256:fixtureFiles,installed_source_matches:true,consumer_lock_sha256:hashBytes(readFileSync(lockPath)),browser_executed:!prepare,browser_outcomes:outcomes,adapter,native_binary_sha256:binary?hashBytes(readFileSync(binary)):null,native_compile_lock_sha256:native?.files?.['Cargo.lock'],commands,limits:['Synthetic loopback session identity and real disposable database; not production provider identity admission.','Unit/type checks do not establish App draft behavior.','Browser persistence is not eviction or power-loss certification.']};
 writeFileSync(join(output,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({completed:!scenario,output,browser:!prepare,admitted:admit}));
}catch(error){writeFileSync(join(output,'failure.json'),JSON.stringify({completed:false,source_mode:sourceMode,error:error.message,commands},null,2));console.error(error.message);process.exitCode=1;}
