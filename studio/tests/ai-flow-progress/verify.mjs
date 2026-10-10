// Installed Studio-source composition with a separately compiled, source-fenced real FlowClient host.
import {cpSync,mkdirSync,readFileSync,writeFileSync,lstatSync,realpathSync,readdirSync,existsSync} from 'node:fs';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual} from 'node:util';
import {randomBytes} from 'node:crypto';
import {reserveOutput,sourceManifest,hashBytes,assertLockedGraph,browserOutcomes} from '../public-controls/verification.mjs';
import {command} from '../../../scripts/packages/astral-owned-command.mjs';
import {fenceProducer,fenceInputs,requirePackageProvenance} from '../../../tests/ai-flows-installed/provenance.mjs';
import {fixtureSources,copiedSources,requireFixtureFreeze,requireCopiedInputs,runtimeFiles} from './fixture-fence.mjs';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../../..');
export function nativeSources(root) {
  const files={};for(const subtree of ['studio/tests/ai-flow-progress/host','examples/ai-flows/src']){
    const visit=relative=>{const info=lstatSync(join(root,relative));if(info.isDirectory()){for(const name of readdirSync(join(root,relative)).sort()){if(name==='target')continue;visit(join(relative,name));}}else if(info.isFile())files[relative]=hashBytes(readFileSync(join(root,relative)));else throw Error('native source contains nonregular entry');};visit(subtree);
  }
  files['examples/ai-flows/Cargo.toml']=hashBytes(readFileSync(join(root,'examples/ai-flows/Cargo.toml')));return files;
}
export function requireNativeProvenance(value,binary,root) {
  if(value?.format!=='ai-flow-progress-native-v1'||value.compile_exit_code!==0||value.binary_sha256!==hashBytes(readFileSync(binary))||typeof value.compiler!=='string'||!value.compiler.startsWith('rustc ')||!/^[0-9a-f]{64}$/.test(value.compile_lock_sha256??'')||value.root_lock_sha256!==hashBytes(readFileSync(join(root,'Cargo.lock')))||!isDeepStrictEqual(value.source_files,nativeSources(root)))throw Error('native binary lacks matching current source/compiler/lock provenance');
  if (!value.producer_root || !value.source_archive || !Array.isArray(value.crate_archives) || !value.crate_archives.length || !Array.isArray(value.packages) || value.packages.length !== value.crate_archives.length) throw Error('missing immutable native package provenance');
  fenceProducer(value.producer_root,value.producer);
  if(value.producer_lock_seed_sha256!==value.producer.lock_sha256)throw Error('native compile lacks exact producer lock seed');
  if(hashBytes(readFileSync(value.source_archive))!==value.source_archive_sha256)throw Error('native source archive changed');
  for(const archive of value.crate_archives)if(hashBytes(readFileSync(archive.path))!==archive.sha256)throw Error('native crate archive changed');
  for(const pkg of value.packages)fenceInputs(pkg.directory,pkg.files);
  requirePackageProvenance(value.packages,value.producer_root);
}
async function main(){
  const args=process.argv.slice(2),options={};let requested,diagnose=false;
  for(let index=0;index<args.length;index++){const flag=args[index];if(['--adapter','--source-dir','--host-binary','--host-provenance','--fixture-freeze','--port'].includes(flag)){if(!args[index+1]||args[index+1].startsWith('--'))throw Error(`${flag} requires value`);options[flag.slice(2)]=args[++index];}else if(flag==='--diagnose-startup')diagnose=true;else if(flag.startsWith('--')||requested)throw Error('invalid arguments');else requested=flag;}
  if(!requested||!['sqlite','redb'].includes(options.adapter)||!options['source-dir']||!options['host-binary']||!options['host-provenance']||!options['fixture-freeze'])throw Error('exclusive output, adapter, extracted Studio source, real host, provenance and independent fixture freeze required');
  const binary=resolve(options['host-binary']),source=resolve(options['source-dir']),provenancePath=resolve(options['host-provenance']),port=Number(options.port??43286);
  if(!lstatSync(binary).isFile()||lstatSync(binary).size>134217728||!Number.isInteger(port)||port<1024||port>65535)throw Error('bounded native binary and port required');
  if(!process.env.ROM_CHROMIUM_PATH||!process.env.ROM_WEBKIT_EXECUTABLE)throw Error('actual Chromium and WebKit executable paths required');
  const maintainedRoot=join(root,'studio/tests/ai-flow-progress'),freezePath=resolve(options['fixture-freeze']);
  if(freezePath===maintainedRoot||freezePath.startsWith(maintainedRoot+'/')||realpathSync(freezePath)!==freezePath||!lstatSync(freezePath).isFile())throw Error('fixture freeze must be an independent regular file outside fixture inputs');
  const freezeHash=hashBytes(readFileSync(freezePath)),maintainedInputs=requireFixtureFreeze(JSON.parse(readFileSync(freezePath,'utf8')),fixtureSources(maintainedRoot));
  const provenance=JSON.parse(readFileSync(provenancePath,'utf8'));requireNativeProvenance(provenance,binary,root);const sourceFiles=sourceManifest(source),binaryHash=hashBytes(readFileSync(binary)),nativeBefore=nativeSources(root);
  const output=reserveOutput(requested),fixture=join(output,'consumer'),extracted=join(output,'extracted'),runtime=join(output,'runtime'),commands=[];
  let copiedInputs;
  function fenceFixture(){if(hashBytes(readFileSync(freezePath))!==freezeHash)throw Error('selected fixture freeze changed');requireFixtureFreeze(JSON.parse(readFileSync(freezePath,'utf8')),fixtureSources(maintainedRoot));requireCopiedInputs(maintainedInputs,copiedSources(output));}
  const record={app:fixture,target:join(output,'unused-native-target'),evidence:output};
  async function run(program,args,label,seconds){const stdout=await command(record,args,label,seconds,8388608,program);commands.push({program,args,label,seconds,terminal:true});return stdout;}
  try{
    mkdirSync(extracted);for(const file of ['package.json','package-lock.json','src','LICENSE','THIRD_PARTY_NOTICES.md'])cpSync(join(source,file),join(extracted,file),{recursive:true});cpSync(join(root,'studio/tests/ai-flow-progress/consumer'),fixture,{recursive:true});mkdirSync(runtime);
    for(const file of runtimeFiles)cpSync(join(root,'studio/tests/ai-flow-progress',file),join(runtime,file));
    copiedInputs=copiedSources(output);requireCopiedInputs(maintainedInputs,copiedInputs);fenceFixture();
    const lockPath=join(fixture,'package-lock.json'),lock=JSON.parse(readFileSync(lockPath,'utf8')),producer=JSON.parse(readFileSync(join(extracted,'package.json'),'utf8')),local=lock.packages['node_modules/rom-studio'];
    if(local.version!==producer.version||!isDeepStrictEqual(local.dependencies,producer.dependencies))throw Error('consumer lock metadata differs from supplied Studio package');
    await run('npm',['ci','--prefer-offline','--install-links','--no-audit','--no-fund'],'install',180);assertLockedGraph(lock,JSON.parse(readFileSync(lockPath,'utf8')));
    const installed=join(fixture,'node_modules/rom-studio');if(lstatSync(installed).isSymbolicLink()||!realpathSync(installed).startsWith(realpathSync(fixture)+'/')||!isDeepStrictEqual(sourceFiles,sourceManifest(installed)))throw Error('installed Studio must be matching physical consumer-local source');
    for(const [path,entry]of Object.entries(lock.packages)){if(!path||!entry.version)continue;const file=join(fixture,path,'package.json');if(!existsSync(file)&&entry.optional)continue;if(!existsSync(file)||JSON.parse(readFileSync(file,'utf8')).version!==entry.version)throw Error(`dependency drift ${path}`);}
    fenceFixture();await run('npm',['run','check'],'type-check',120);fenceFixture();await run('npm',['run','build'],'build',120);fenceFixture();
    const environment={ROM_AI_BROWSER_HOST:binary,ROM_AI_BROWSER_ADAPTER:options.adapter,ROM_AI_BROWSER_EVIDENCE:output,ROM_AI_BROWSER_PORT:String(port),ROM_AI_BROWSER_CONTROL:randomBytes(32).toString('hex'),ROM_AI_BROWSER_DIAGNOSE:diagnose?'1':'0'};
    for(const [name,value]of Object.entries(environment))process.env[name]=value;
    fenceFixture();const browser=await run('npm',['exec','--offline','--','playwright','test','--config','tests/playwright.config.ts','--reporter=json',...(diagnose?['--project','chromium']:[])],'browser',diagnose?60:600);const report=JSON.parse(browser),outcomes=browserOutcomes(report);
    writeFileSync(join(output,'browser-results.json'),JSON.stringify(report,null,2)+'\n');
    if(diagnose){fenceFixture();requireNativeProvenance(provenance,binary,root);if(outcomes.chromium?.length!==1||outcomes.chromium[0]!=='passed'||outcomes.webkit)throw Error('missing or failed single Chromium startup observation');writeFileSync(join(output,'diagnostic-result.json'),JSON.stringify({acceptance_complete:false,diagnostic:true,adapter:options.adapter,browser_outcomes:outcomes,fixture_freeze_sha256:freezeHash,maintained_fixture_inputs:maintainedInputs,copied_fixture_inputs:copiedInputs,commands},null,2)+'\n');console.log(JSON.stringify({acceptance_complete:false,diagnostic:true,output}));return;}
    for(const engine of ['chromium','webkit'])if(outcomes[engine]?.length!==12||outcomes[engine].some(value=>value!=='passed'))throw Error(`missing or failed ${engine} domain cases`);
    fenceFixture();
    if(binaryHash!==hashBytes(readFileSync(binary))||!isDeepStrictEqual(nativeBefore,nativeSources(root))||!isDeepStrictEqual(sourceFiles,sourceManifest(source))||!isDeepStrictEqual(sourceFiles,sourceManifest(installed)))throw Error('source/binary changed during browser acceptance');requireNativeProvenance(provenance,binary,root);
    writeFileSync(join(output,'native-provenance.json'),JSON.stringify(provenance,null,2)+'\n');writeFileSync(join(output,'result.json'),JSON.stringify({completed:true,admitted:false,adapter:options.adapter,source_mode:'supplied_extracted_studio_source',source_files:sourceFiles,fixture_freeze_sha256:freezeHash,maintained_fixture_inputs:maintainedInputs,copied_fixture_inputs:copiedInputs,installed_source_matches:true,browser_outcomes:outcomes,native_binary_sha256:binaryHash,native_provenance_sha256:hashBytes(readFileSync(provenancePath)),commands,limits:['Application-owned disposable transport, not production authentication or external-provider integration.','Process restart and IndexedDB transaction completion do not certify power-loss durability.','Inherited process-group helper is finite for observed child groups; escaped descendants holding stdio are not adversarially contained.']},null,2)+'\n');console.log(JSON.stringify({completed:true,admitted:false,output}));
  }catch(error){writeFileSync(join(output,'failure.json'),JSON.stringify({completed:false,error:error.message,commands},null,2)+'\n');throw error;}
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(error=>{console.error(error.message);process.exitCode=1;});
