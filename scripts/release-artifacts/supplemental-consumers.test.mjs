import test from 'node:test';import assert from 'node:assert/strict';
import { mkdtempSync,mkdirSync,writeFileSync } from 'node:fs';import { join } from 'node:path';import { tmpdir } from 'node:os';
import { digest } from '../skills/files.mjs';
import { requireDiagnosticExecution, requireInstalledAiExecution } from './supplemental-consumers.mjs';
test('diagnostic acceptance requires all three real public tests, not a general exit-zero summary',()=>{
 const names=['public_host_export_preserves_links_without_private_payloads','full_or_closed_host_reader_does_not_block_commands','stalled_then_failed_host_exporter_cannot_reject_a_committed_command'];
 const stdout=names.map(n=>`test ${n} ... ok`).join('\n');assert.equal(requireDiagnosticExecution(stdout).length,3);
 for(const name of names)assert.throws(()=>requireDiagnosticExecution(stdout.replace(`test ${name} ... ok`,'')),/missing executed/);
});
test('installed AI report cannot replace absent native executions or substitute source identity',()=>{
 const directory=mkdtempSync(join(tmpdir(),'rom-ai-admission-'));mkdirSync(join(directory,'evidence'));
 const source={archive_sha256:'a'.repeat(64),source_inventory_sha256:digest([])};
 writeFileSync(join(directory,'acceptance.json'),JSON.stringify({completed:true,source_archive_sha256:source.archive_sha256,source:{files:[],source_inventory_sha256:source.source_inventory_sha256,source_mode:'committed-release-source'},offline:true,cargo_jobs:2,incremental:false,publication:false,results:[{},{},{},{}]}));
 assert.throws(()=>requireInstalledAiExecution(directory,source),/ENOENT/);
 assert.throws(()=>requireInstalledAiExecution(directory,{...source,archive_sha256:'c'.repeat(64)}),/invalid installed/);
});

test('AI process evidence requires actual drained execution and preserved byte counts for compiler and compile gates',async()=>{
 const {requireAiProcessEvidence}=await import('./supplemental-consumers.mjs');
 const directory=mkdtempSync(join(tmpdir(),'rom-ai-process-'));mkdirSync(join(directory,'evidence'));
 const stdout='successful compile\n',stderr='';const record={program:'node',args:['/owned/checker.mjs'],code:0,signal:null,stopped:null,spawnError:null,processGroup:123,groupAbsent:true,stdoutBytes:Buffer.byteLength(stdout),stderrBytes:0,seenBytes:Buffer.byteLength(stdout),outputTruncated:false};
 for(const [file,data]of [['gate.result.json',JSON.stringify(record)],['gate.stdout.log',stdout],['gate.stderr.log',stderr]])writeFileSync(join(directory,'evidence',file),data);
 assert.equal(requireAiProcessEvidence(directory,'gate').stdout,stdout);
 for(const delta of [{groupAbsent:false},{stopped:'deadline'},{signal:'SIGTERM'},{stdoutBytes:0},{program:'other'},{code:1}]){
 writeFileSync(join(directory,'evidence/gate.result.json'),JSON.stringify({...record,...delta}));assert.throws(()=>requireAiProcessEvidence(directory,'gate', 'node'),/AI process/);
 }
});

test('installed AI final input observation is required and must match original source and resolved package/consumer witnesses',async()=>{
 const {requireInstalledAiObservation}=await import('./supplemental-consumers.mjs');
 const source={files:[{path:'source.rs',mode:'100644',sha256:'a'.repeat(64)}]},report={source,source_archive_sha256:'b'.repeat(64),crate_archives:[{path:'/owned/core.crate',sha256:'c'.repeat(64)}],packages:[{name:'rom',files:{'Cargo.toml':'d'.repeat(64)}}],consumers:[{name:'app',files:{'Cargo.lock':'e'.repeat(64)}}]};
 const observation={producer:{'source.rs':'a'.repeat(64)},extracted_producer:{'source.rs':'a'.repeat(64)},source_archive_sha256:'b'.repeat(64),crate_archives:[{path:'/owned/core.crate',sha256:'c'.repeat(64)}],packages:[{name:'rom',expected:report.packages[0].files,observed:report.packages[0].files}],consumers:[{name:'app',expected:report.consumers[0].files,observed:report.consumers[0].files}]};
 assert.equal(requireInstalledAiObservation(report,observation),true);
 for(const mutate of [o=>o.producer.extra='f'.repeat(64),o=>o.packages[0].observed={'Cargo.toml':'f'.repeat(64)},o=>o.consumers=[],o=>o.crate_archives[0].sha256='f'.repeat(64)]){
 const changed=structuredClone(observation);mutate(changed);assert.throws(()=>requireInstalledAiObservation(report,changed),/AI final inputs/);
 }
});
