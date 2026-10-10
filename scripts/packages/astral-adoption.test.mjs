import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,symlinkSync,existsSync,chmodSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { inspectPackages,rewriteManifest,prepareAdoption,auditGraph,runAdoption } from './astral-adoption.mjs';
const names=['rom','rom-sqlite','rom-identity','rom-studio-host'];
const manifest='[package]\nname = "astral-plane"\nversion = "0.1.0-dev.1"\n\n[workspace]\nexclude = ["vendor/astrorust", "vendor/rom"]\n\n[dependencies]\n'+names.map(name=>`${name} = { path = "vendor/rom/crates/${name}" }`).join('\n')+'\nastrorust_core = { path = "vendor/astrorust/crates/astrorust_core" }\nreqwest = { version = "=0.13.5", default-features = false, features = ["json", "rustls"] }\n\n[dev-dependencies]\ntokio = { version = "=1.53.1", features = ["test-util"] }\n\n[lints.rust]\nunsafe_code = "forbid"\n';
function fixture(){
  const root=mkdtempSync(join(tmpdir(),'rom-astral-adoption-test-')),consumer=join(root,'consumer'),packageRoot=join(root,'packages');
  mkdirSync(consumer);mkdirSync(packageRoot);
  for(const name of names){const dir=join(packageRoot,name+'-0.0.3');mkdirSync(dir);writeFileSync(join(dir,'Cargo.toml'),`[package]\nname = "${name}"\nversion = "0.0.3"\n`);writeFileSync(join(dir,'LICENSE'),'MIT fixture');}
  for(const [path,text] of [['Cargo.toml',manifest],['Cargo.lock','version = 4\n'],['src/main.rs','fn main() {}'],['tests/host.rs','test'],['knowledge/library.json','[]'],['vendor/astrorust/Cargo.toml','[workspace]\n'],['vendor/astrorust/crates/astrorust_core/Cargo.toml','[package]\nname = "astrorust_core"\nversion = "0.1.0"\n']]){mkdirSync(join(consumer,path,'..'),{recursive:true});writeFileSync(join(consumer,path),text);}
  return {root,consumer,packageRoot,evidence:join(root,'evidence'),target:join(root,'target')};
}
function use(fn){const f=fixture();try{return fn(f);}finally{rmSync(f.root,{recursive:true,force:true});}}

test('manifest rewrite changes only four direct ROM paths and preserves Astrorust, features, lints, workspace and dev dependencies',()=>use(f=>{
  const packages=inspectPackages(f.packageRoot), rewritten=rewriteManifest(manifest,packages);
  let expected=manifest;for(const name of names)expected=expected.replace(`${name} = { path = "vendor/rom/crates/${name}" }`,`${name} = { version = "=0.0.3" }`);
  assert.equal(rewritten,expected);
  assert.throws(()=>rewriteManifest(manifest.replace('path = "vendor/rom/crates/rom"','path = "vendor/rom/crates/rom", features = ["private"]'),packages),/options/);
  assert.throws(()=>rewriteManifest(manifest+'\n[patch.crates-io]\nrom = { path = "external" }\n',packages),/unsupported/);
}));

test('copy creates new private app inputs and routes exact extracted packages without copying ROM vendor',()=>use(f=>{
  const record=prepareAdoption(f);
  assert.equal(readFileSync(join(f.consumer,'Cargo.toml'),'utf8'),manifest);
  assert.equal(readFileSync(join(record.app,'knowledge/library.json'),'utf8'),'[]');
  assert.ok(existsSync(join(record.app,'vendor/astrorust/Cargo.toml')));
  assert.equal(existsSync(join(record.app,'vendor/rom')),false);
  for(const name of names)assert.ok(readFileSync(join(record.app,'.cargo/config.toml'),'utf8').includes(join(f.packageRoot,name+'-0.0.3')));
  assert.throws(()=>prepareAdoption(f),/occupied/);
}));

test('source symlinks, symbolic package ancestors and unsafe output paths fail closed',()=>{
  use(f=>{symlinkSync('/tmp',join(f.consumer,'src/link'));assert.throws(()=>prepareAdoption(f),/symbolic/);});
  use(f=>{symlinkSync(join(f.packageRoot,'rom-0.0.3'),join(f.root,'linked-package'));assert.throws(()=>inspectPackages(join(f.root,'linked-package')),/symbolic/);});
  use(f=>assert.throws(()=>prepareAdoption({...f,evidence:join(f.consumer,'new')}),/unsafe/));
  use(f=>assert.throws(()=>prepareAdoption({...f,target:f.evidence}),/unsafe/));
});

test('unknown and missing extracted packages are rejected',()=>{
  use(f=>{const dir=join(f.packageRoot,'rom-private-0.0.3');mkdirSync(dir);writeFileSync(join(dir,'Cargo.toml'),'[package]\nname = "rom-private"\nversion = "0.0.3"\n');assert.throws(()=>inspectPackages(f.packageRoot),/unknown/);});
  use(f=>{rmSync(join(f.packageRoot,'rom-sqlite-0.0.3'),{recursive:true});assert.throws(()=>inspectPackages(f.packageRoot),/missing/);});
});

function graph(packages,workspace){return {workspace_root:workspace,packages:packages.map(pkg=>({id:pkg.name,name:pkg.name,version:pkg.version,source:null,manifest_path:join(pkg.directory,'Cargo.toml')})),resolve:{nodes:packages.map(pkg=>({id:pkg.name}))}};}
test('graph audit rejects checkout paths, unexpected versions, mutation and unknown local crates',()=>use(f=>{
  const packages=inspectPackages(f.packageRoot), metadata=graph(packages,f.consumer);
  assert.deepEqual(auditGraph(metadata,packages,'','',''),[]);
  for(const changed of [{manifest_path:join(f.consumer,'vendor/rom/crates/rom/Cargo.toml')},{version:'9.0.0'},{source:'registry+https://github.com/rust-lang/crates.io-index'}]){const invalid=structuredClone(metadata);Object.assign(invalid.packages[0],changed);assert.throws(()=>auditGraph(invalid,packages,'','',''),/escaped/);}
  const extra=structuredClone(metadata);extra.packages.push({id:'private',name:'private',source:null,manifest_path:join(f.consumer,'Cargo.toml')});extra.resolve.nodes.push({id:'private'});assert.throws(()=>auditGraph(extra,packages,'','',''),/local/);
  writeFileSync(join(packages[0].directory,'LICENSE'),'changed');assert.throws(()=>auditGraph(metadata,packages,'','',''),/changed/);
}));

test('registry drift requires exact candidate version, source and checksum evidence',()=>use(f=>{
  const packages=inspectPackages(f.packageRoot),metadata=graph(packages,f.consumer);
  const source='registry+https://github.com/rust-lang/crates.io-index';metadata.packages.push({id:'hmac',name:'hmac',version:'0.12.1',source});metadata.resolve.nodes.push({id:'hmac'});
  const lock=`[[package]]\nname = "hmac"\nversion = "0.12.1"\nsource = "${source}"\nchecksum = "abc"\n`;
  assert.equal(auditGraph(metadata,packages,'',lock,lock).length,1);
  assert.deepEqual(auditGraph(metadata,packages,lock,'',lock),[]);
  assert.throws(()=>auditGraph(metadata,packages,'',lock.replace('abc','other'),lock),/unexplained/);
}));

test('runner rejects absent or unbounded explicit time/output budgets before preparing or launching Cargo',async()=>{
  const f=fixture();try{await assert.rejects(runAdoption(f),/finite budget/);assert.equal(existsSync(f.evidence),false);}finally{rmSync(f.root,{recursive:true,force:true});}
});

test('failed tool retains final source drift audit without replacing its primary failure',async()=>{
  const f=fixture(),bin=join(f.root,'bin'),priorPath=process.env.PATH;
  mkdirSync(bin);
  const tool=join(bin,'rustc');
  writeFileSync(tool,`#!${process.execPath}\nimport{writeFileSync}from'node:fs';writeFileSync(${JSON.stringify(join(f.consumer,'Cargo.lock'))},'changed by failed fixture');process.exit(2);\n`);
  chmodSync(tool,0o700);
  process.env.PATH=bin+':'+priorPath;
  try {
    await assert.rejects(runAdoption({...f,candidateLock:join(f.consumer,'Cargo.lock'),metadataSeconds:3,testSeconds:3,clippySeconds:3,buildSeconds:3,outputBytes:4096}),/command failed: compiler/);
    const failure=JSON.parse(readFileSync(join(f.evidence,'failure.json')));
    assert.equal(failure.message,'adoption command failed: compiler');
    const audit=JSON.parse(readFileSync(join(f.evidence,'final-input-audit.json')));
    assert.equal(audit.originalSources['Cargo.lock'].unchanged,false);
    assert.ok(audit.packages.every(p=>p.unchanged));
    assert.equal(existsSync(join(f.evidence,'acceptance.json')),false);
  } finally {process.env.PATH=priorPath;rmSync(f.root,{recursive:true,force:true});}
});
