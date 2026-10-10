// Reuse already admitted immutable packages; never compile against checkout ROM dependencies.
import {readFileSync,writeFileSync,cpSync,mkdirSync,existsSync} from 'node:fs';
import {resolve,dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual} from 'node:util';
import {inventory} from '../../../scripts/packages/astral-adoption.mjs';
import {command} from '../../../scripts/packages/astral-owned-command.mjs';
import {hash} from '../../../scripts/skills/files.mjs';
import {fenceInputs,fenceProducer,requirePackageProvenance,auditAiGraph,auditRegistryLock} from '../../../tests/ai-flows-installed/provenance.mjs';
import {rewriteAiManifest,requireAiCompiler} from '../../../tests/ai-flows-installed/contracts.mjs';
import {nativeSources} from './verify.mjs';
import {seedProducerLock} from './lock-seed.mjs';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../../..');
const args=process.argv.slice(2);if(args.length!==5)throw Error('usage: prepare-host.mjs OPTIONS_JSON INSTALLED_ACCEPTANCE_JSON PRODUCER_ROOT EXCLUSIVE_OUTPUT EXCLUSIVE_TARGET');
const [optionsPath,acceptancePath,producer,output,target]=args.map(value=>resolve(value)),options=JSON.parse(readFileSync(optionsPath,'utf8')),accepted=JSON.parse(readFileSync(acceptancePath,'utf8'));
if(!accepted.completed||!accepted.offline||!isDeepStrictEqual(accepted.source,options.source)||!isDeepStrictEqual(accepted.crate_archives,options.crateArchives)||existsSync(output)||existsSync(target))throw Error('exact admitted input identity and exclusive output/target required');
const before=nativeSources(root),packages=accepted.packages;
function fence(){fenceProducer(producer,options.source);for(const item of options.crateArchives)if(hash(item.path)!==item.sha256)throw Error('archive changed');if(hash(options.sourceArchive)!==options.sourceArchiveSha256)throw Error('source archive changed');for(const pkg of packages)fenceInputs(pkg.directory,pkg.files);requirePackageProvenance(packages,producer);if(!isDeepStrictEqual(before,nativeSources(root)))throw Error('authoring fixture changed');}
fence();for(const file of ['Cargo.toml',...Object.keys(inventory(join(root,'examples/ai-flows/src'))).map(path=>'src/'+path)])if(hash(join(root,'examples/ai-flows',file))!==hash(join(producer,'examples/ai-flows',file)))throw Error('public domain source differs from selected immutable producer');
mkdirSync(output,{mode:0o700});mkdirSync(target,{mode:0o700});const app=join(output,'host'),domain=join(output,'domain'),evidence=join(output,'evidence');mkdirSync(evidence);mkdirSync(domain);
cpSync(join(root,'studio/tests/ai-flow-progress/host'),app,{recursive:true});cpSync(join(producer,'examples/ai-flows/src'),join(domain,'src'),{recursive:true});writeFileSync(join(domain,'Cargo.toml'),rewriteAiManifest(readFileSync(join(producer,'examples/ai-flows/Cargo.toml'),'utf8'),packages));
let manifest=readFileSync(join(app,'Cargo.toml'),'utf8');for(const name of ['rom','rom-ai','rom-sqlite','rom-redb']){const pkg=packages.find(item=>item.name===name);if(!pkg)throw Error('missing extracted host dependency');const pattern=new RegExp(`^${name} = \\{ version = "=0\\.0\\.3", path = "\\.\\./\\.\\./\\.\\./\\.\\./crates/${name}" \\}$`,'m');if(!pattern.test(manifest))throw Error('unreviewed host dependency syntax');manifest=manifest.replace(pattern,`${name} = { version = "=${pkg.version}" }`);}
manifest=manifest.replace('rom-ai-flows-consumer = { path = "../../../../examples/ai-flows" }','rom-ai-flows-consumer = { path = "../domain" }');writeFileSync(join(app,'Cargo.toml'),manifest);mkdirSync(join(app,'.cargo'));const config=join(app,'.cargo/config.toml');writeFileSync(config,'[patch.crates-io]\n'+packages.map(pkg=>`${JSON.stringify(pkg.name)} = { path = ${JSON.stringify(pkg.directory)} }`).join('\n')+'\n');
const producerLockSeed=seedProducerLock(producer,app,options.source.lock_sha256);
const sourceInputs=inventory(app),domainInputs=inventory(domain),resolvedPackages=[...packages,{name:'rom-ai-flows-consumer',version:'0.0.0',directory:domain}];
const admitted=[readFileSync(join(producer,'Cargo.lock'),'utf8'),...packages.filter(pkg=>existsSync(join(pkg.directory,'Cargo.lock'))).map(pkg=>readFileSync(join(pkg.directory,'Cargo.lock'),'utf8'))];
async function run(program,argv,label,seconds){fence();fenceInputs(domain,domainInputs);return command({app,target,evidence},argv,label,seconds,8388608,program);}
try{
  const compiler=requireAiCompiler(await run('rustc',['-Vv'],'compiler',30),options.compiler);
  const base=['--config',config];const metadata=JSON.parse(await run('cargo',[...base,'metadata','--offline','--format-version','1'],'resolve',60));
  const after=inventory(app),expected={...sourceInputs};delete after['Cargo.lock'];delete expected['Cargo.lock'];if(!isDeepStrictEqual(after,expected))throw Error('resolution changed host source');auditRegistryLock(readFileSync(join(app,'Cargo.lock'),'utf8'),admitted);sourceInputs['Cargo.lock']=hash(join(app,'Cargo.lock'));auditAiGraph(metadata,resolvedPackages,app,'rom-ai-progress-host');
  fenceInputs(app,sourceInputs);await run('cargo',[...base,'build','--offline','--locked','--bin','rom-ai-progress-host'],'compile',600);fenceInputs(app,sourceInputs);auditAiGraph(JSON.parse(await run('cargo',[...base,'metadata','--offline','--locked','--format-version','1'],'final-metadata',60)),resolvedPackages,app,'rom-ai-progress-host');fence();
  const binary=join(target,'debug/rom-ai-progress-host');const provenance={format:'ai-flow-progress-native-v1',compile_exit_code:0,binary_sha256:hash(binary),compiler:compiler.verbose,compile_lock_sha256:hash(join(app,'Cargo.lock')),producer_lock_seed_sha256:producerLockSeed,root_lock_sha256:hash(join(root,'Cargo.lock')),source_files:before,producer:options.source,producer_root:producer,source_archive:options.sourceArchive,source_archive_sha256:options.sourceArchiveSha256,crate_archives:options.crateArchives,packages:packages.map(pkg=>({name:pkg.name,version:pkg.version,directory:pkg.directory,files:pkg.files})),domain_inputs:domainInputs,compiled_host_inputs:sourceInputs};writeFileSync(join(output,'source.json'),JSON.stringify(provenance,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({compiled:true,binary,provenance:join(output,'source.json')}));
}catch(error){writeFileSync(join(output,'failure.json'),JSON.stringify({compiled:false,error:error.message},null,2)+'\n');throw error;}
