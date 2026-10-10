// Isolated adoption of extracted ROM packages by the real Astral application.
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, copyFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { command } from './astral-owned-command.mjs';
export { command } from './astral-owned-command.mjs';

const known = new Set(['rom','rom-ai','rom-auth','rom-backup','rom-blob','rom-blob-object-store','rom-cli','rom-config','rom-conformance','rom-derive','rom-fields','rom-http','rom-identity','rom-openrouter','rom-redb','rom-sqlite','rom-studio-host']);
const direct = ['rom', 'rom-sqlite', 'rom-identity', 'rom-studio-host'];
const digest = value => createHash('sha256').update(value).digest('hex');
const inside = (root, path) => resolve(path).startsWith(resolve(root) + sep);
function regular(path) {
  const stat = lstatSync(path);
  if (stat.isSymbolicLink() || (!stat.isFile() && !stat.isDirectory())) throw Error('nonregular or symbolic input');
  if (realpathSync(path) !== resolve(path)) throw Error('input has symbolic ancestor');
  return stat;
}
export function inventory(root) {
  const result = {};
  function visit(path) {
    const stat = regular(path);
    if (stat.isDirectory()) for (const name of readdirSync(path).sort()) {
      if (['target','node_modules','.git'].includes(name)) continue;
      visit(join(path,name));
    } else result[relative(root,path)] = digest(readFileSync(path));
  }
  visit(root);
  return result;
}
function copyTree(source, destination) {
  const stat = regular(source);
  if (stat.isDirectory()) {
    mkdirSync(destination,{recursive:true,mode:0o700});
    for (const name of readdirSync(source)) if (!['target','node_modules','.git'].includes(name)) copyTree(join(source,name),join(destination,name));
  } else {
    mkdirSync(dirname(destination),{recursive:true,mode:0o700});
    copyFileSync(source,destination);
  }
}
function identity(manifest) {
  const section = manifest.split(/^\[package\]\s*$/m)[1]?.split(/^\[/m)[0];
  const name = section?.match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1];
  const version = section?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!name || !version) throw Error('package requires normalized explicit identity');
  return {name,version};
}
export function inspectPackages(root) {
  regular(root);
  const result = [];
  for (const name of readdirSync(root).sort()) {
    const directory = join(root,name);
    if (!regular(directory).isDirectory()) throw Error('unexpected extracted package entry');
    const pkg = identity(readFileSync(join(directory,'Cargo.toml'),'utf8'));
    if (!known.has(pkg.name) || name !== `${pkg.name}-${pkg.version}` || result.some(other=>other.name===pkg.name)) throw Error('unknown, duplicated, or misnamed extracted package');
    result.push({...pkg,directory,files:inventory(directory)});
  }
  if (!direct.every(name=>result.some(pkg=>pkg.name===name))) throw Error('missing direct ROM package');
  return result;
}
export function rewriteManifest(text, packages) {
  let section = '', changed = new Set();
  const result = text.split('\n').map(line => {
    if (/^\[/.test(line.trim())) section = line.trim();
    const match = line.match(/^(rom(?:-[a-z-]+)?)\s*=\s*\{([^}]+)\}\s*$/);
    if (!match) return line;
    if (section !== '[dependencies]' || !direct.includes(match[1]) || changed.has(match[1])) throw Error('unsupported ROM dependency declaration');
    const original = match[2].trim();
    // Reject additional options rather than silently discard author intent.
    if (original !== `path = "vendor/rom/crates/${match[1]}"`) throw Error('unexpected original ROM dependency options');
    const pkg = packages.find(pkg=>pkg.name===match[1]);
    if (!pkg) throw Error('missing direct ROM package');
    changed.add(match[1]);
    return `${match[1]} = { version = "=${pkg.version}" }`;
  }).join('\n');
  if (changed.size !== direct.length || /^\s*\[patch[.\]]/m.test(result) || /\bpath\s*=\s*"[^"]*vendor\/rom\//.test(result)) throw Error('unsupported or retained ROM source override');
  return result;
}
function lockPackages(text) {
  return text.split('[[package]]').slice(1).map(section=>{
    const field = name=>section.match(new RegExp(`^${name} = "([^"]+)"`,'m'))?.[1] ?? null;
    return {name:field('name'),version:field('version'),source:field('source'),checksum:field('checksum')};
  });
}
export function auditGraph(metadata, packages, originalLock, candidateLock, resolvedLock) {
  const graph = new Set(metadata.resolve.nodes.map(node=>node.id));
  const expected = new Map(packages.map(pkg=>[pkg.name,pkg]));
  const original = lockPackages(originalLock), candidate = lockPackages(candidateLock), resolved = lockPackages(resolvedLock);
  const drift = [];
  for (const pkg of metadata.packages.filter(pkg=>graph.has(pkg.id))) {
    if (pkg.name === 'rom' || pkg.name.startsWith('rom-')) {
      const accepted = expected.get(pkg.name);
      if (!accepted || pkg.version !== accepted.version || pkg.source !== null || resolve(pkg.manifest_path) !== join(accepted.directory,'Cargo.toml')) throw Error('ROM graph escaped exact extracted package');
      regular(pkg.manifest_path);
      if (JSON.stringify(inventory(accepted.directory)) !== JSON.stringify(accepted.files)) throw Error('extracted package changed');
    } else if (pkg.source !== null) {
      const entry = resolved.find(item=>item.name===pkg.name && item.version===pkg.version && item.source===pkg.source);
      if (!entry) throw Error('resolved registry package missing from lock');
      const same = item=>JSON.stringify(item)===JSON.stringify(entry);
      if (!original.some(same)) {
        if (!candidate.some(same)) throw Error('unexplained registry dependency drift');
        drift.push({package:entry,explanation:'exact candidate ROM lock dependency'});
      }
    } else {
      const root=metadata.workspace_root;
      if(typeof root!=='string' || !(pkg.name==='astral-plane'
        ? resolve(pkg.manifest_path)===join(root,'Cargo.toml')
        : pkg.name.startsWith('astrorust_') && inside(join(root,'vendor/astrorust'),pkg.manifest_path))) {
        throw Error('unexpected local application dependency');
      }
      regular(pkg.manifest_path);
    }
  }
  if (!direct.every(name=>metadata.packages.some(pkg=>graph.has(pkg.id)&&pkg.name===name))) throw Error('missing resolved direct ROM dependency');
  return drift;
}
export function prepareAdoption(options) {
  const consumer = realpathSync(options.consumer), packageRoot = realpathSync(options.packageRoot);
  if (consumer !== resolve(options.consumer) || packageRoot !== resolve(options.packageRoot)) throw Error('symbolic root');
  const evidence = resolve(options.evidence), target = resolve(options.target);
  if (existsSync(evidence) || existsSync(target) || inside(consumer,evidence) || inside(consumer,target) || inside(packageRoot,evidence) || inside(packageRoot,target) || inside(evidence,target) || inside(target,evidence) || evidence===target) throw Error('unsafe or occupied adoption output');
  regular(dirname(evidence)); regular(dirname(target));
  const packages = inspectPackages(packageRoot);
  const originalManifest = readFileSync(join(consumer,'Cargo.toml'),'utf8');
  const manifest = rewriteManifest(originalManifest,packages);
  const sources = ['Cargo.toml','Cargo.lock','src','tests','knowledge','vendor/astrorust'];
  const originalInputs = Object.fromEntries(sources.map(name=>[name,inventory(join(consumer,name))]));
  mkdirSync(evidence,{mode:0o700}); mkdirSync(target,{mode:0o700});
  const app = join(evidence,'application'); mkdirSync(app,{mode:0o700});
  for (const name of sources) copyTree(join(consumer,name),join(app,name));
  writeFileSync(join(app,'Cargo.toml'),manifest);
  mkdirSync(join(app,'.cargo'));
  writeFileSync(join(app,'.cargo/config.toml'),'[patch.crates-io]\n'+packages.map(pkg=>`${JSON.stringify(pkg.name)} = { path = ${JSON.stringify(pkg.directory)} }`).join('\n')+'\n');
  const record = {consumer,packageRoot,evidence,target,app,packages,originalInputs,applicationInputs:inventory(app),options:{...options},completed:false};
  writeFileSync(join(evidence,'preparation.json'),JSON.stringify(record,null,2)+'\n');
  return record;
}
function finalInputAudit(record,frozenLock) {
  const observe=(path,before)=>{
    try {
      const after=inventory(path);
      return {unchanged:JSON.stringify(after)===JSON.stringify(before),after};
    } catch(error) {return {unchanged:false,observationError:error.code??error.message};}
  };
  const applicationExpected={...record.applicationInputs};
  if(frozenLock)applicationExpected['Cargo.lock']=frozenLock;
  return {
    originalSources:Object.fromEntries(Object.entries(record.originalInputs).map(([name,before])=>[name,observe(join(record.consumer,name),before)])),
    packages:record.packages.map(pkg=>({name:pkg.name,...observe(pkg.directory,pkg.files)})),
    application:observe(record.app,applicationExpected),
    expectedResolvedLockSha256:frozenLock??null,
  };
}
export async function runAdoption(options) {
  for(const key of ['metadataSeconds','testSeconds','clippySeconds','buildSeconds','outputBytes']) if(!Number.isSafeInteger(options[key])||options[key]<1)throw Error(`explicit finite budget required: ${key}`);
  for(const key of ['metadataSeconds','testSeconds','clippySeconds','buildSeconds']) if(options[key]>1800)throw Error('command deadline exceeds 30 minutes');
  if(options.outputBytes>33554432)throw Error('output budget exceeds 32 MiB');
  regular(options.candidateLock);
  const candidateLock=readFileSync(options.candidateLock,'utf8');
  const record=prepareAdoption(options);
  const originalLock=readFileSync(join(record.consumer,'Cargo.lock'),'utf8');
  let frozenLock,auditWritten=false;
  const check=metadata=>{
    if(digest(readFileSync(options.candidateLock))!==digest(candidateLock))throw Error('candidate lock changed');
    if(resolve(metadata.workspace_root)!==record.app)throw Error('metadata workspace escaped isolated application');
    const lock=readFileSync(join(record.app,'Cargo.lock'),'utf8');
    if(frozenLock && digest(lock)!==frozenLock)throw Error('frozen application lock changed');
    const expected={...record.applicationInputs}, actual=inventory(record.app);
    delete expected['Cargo.lock']; delete actual['Cargo.lock'];
    if(JSON.stringify(actual)!==JSON.stringify(expected))throw Error('isolated application inputs changed');
    return auditGraph(metadata,record.packages,originalLock,candidateLock,lock);
  };
  try {
    await command(record,['-Vv'],'compiler',options.metadataSeconds,options.outputBytes,'rustc');
    const initial=JSON.parse(await command(record,['metadata','--offline','--format-version','1'],'resolve',options.metadataSeconds,options.outputBytes));
    const drift=check(initial);
    frozenLock=digest(readFileSync(join(record.app,'Cargo.lock')));
    for(const [label,args,budget] of [
      ['frozen-metadata',['metadata','--offline','--locked','--format-version','1'],'metadataSeconds'],
      ['tests',['test','--offline','--locked','--all-targets'],'testSeconds'],
      ['clippy',['clippy','--offline','--locked','--all-targets','--','-D','warnings'],'clippySeconds'],
      ['host-build',['build','--offline','--locked','--bin','astral-plane'],'buildSeconds'],
    ]) {
      check(initial);
      await command(record,args,label,options[budget],options.outputBytes);
    }
    const final=JSON.parse(await command(record,['metadata','--offline','--locked','--format-version','1'],'final-metadata',options.metadataSeconds,options.outputBytes));
    check(final);
    for(const [name,before] of Object.entries(record.originalInputs))if(JSON.stringify(inventory(join(record.consumer,name)))!==JSON.stringify(before))throw Error('original consumer source changed');
    const binary=join(record.target,'debug/astral-plane'); regular(binary);
    const binaryBytes=readFileSync(binary);
    const finalAudit=finalInputAudit(record,frozenLock);
    writeFileSync(join(record.evidence,'final-input-audit.json'),JSON.stringify(finalAudit,null,2)+'\n');
    auditWritten=true;
    if(Object.values(finalAudit.originalSources).some(item=>!item.unchanged)||finalAudit.packages.some(item=>!item.unchanged)||!finalAudit.application.unchanged)throw Error('final input audit changed');
    record.acceptance={completed:true,registryDrift:drift,lockSha256:digest(readFileSync(join(record.app,'Cargo.lock'))),candidateLockSha256:digest(candidateLock),compilerLogSha256:digest(readFileSync(join(record.evidence,'compiler.stdout.log'))),binarySha256:digest(binaryBytes),binaryBytes:binaryBytes.length,binary,packageInventories:record.packages};
    writeFileSync(join(record.evidence,'acceptance.json'),JSON.stringify(record.acceptance,null,2)+'\n');
    return record.acceptance;
  }catch(error){writeFileSync(join(record.evidence,'failure.json'),JSON.stringify({completed:false,message:error.message},null,2)+'\n');throw error;}
  finally {
    // Preserve observed drift even when execution fails before its ordinary audits.
    // A secondary audit-write failure cannot replace the command's primary result.
    try {if(!auditWritten)writeFileSync(join(record.evidence,'final-input-audit.json'),JSON.stringify(finalInputAudit(record,frozenLock),null,2)+'\n');}
    catch(error) {process.stderr.write(`adoption final input audit unavailable: ${error.code??'write failure'}\n`);}
  }
}
if (process.argv[1] && resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const options=JSON.parse(readFileSync(process.argv[2],'utf8'));
  await runAdoption(options);
}
