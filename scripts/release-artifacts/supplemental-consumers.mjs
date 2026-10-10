// Reuse the actual extracted package and installed-AI runners; metadata cannot replace their tests.
import { mkdirSync, readFileSync, writeFileSync, realpathSync, lstatSync } from 'node:fs';
import { join } from 'node:path';
import { hash, digest, safePath } from '../skills/files.mjs';
import { recordedCommand } from './command-result.mjs';
import { requireAiResults, requireAiCompiler } from '../../tests/ai-flows-installed/contracts.mjs';
import { isDeepStrictEqual } from 'node:util';
import { runChild } from '../skills/process.mjs';
const diagnostics=['public_host_export_preserves_links_without_private_payloads','full_or_closed_host_reader_does_not_block_commands','stalled_then_failed_host_exporter_cannot_reject_a_committed_command'];
export function requireDiagnosticExecution(stdout) {
  for(const name of diagnostics)if(!stdout.split('\n').includes(`test ${name} ... ok`))throw Error('missing executed public diagnostic test');
  return [...diagnostics];
}
export function requireAiProcessEvidence(directory, label, program) {
  const result = JSON.parse(readFileSync(safePath(directory, `evidence/${label}.result.json`), 'utf8'));
  const stdout = readFileSync(safePath(directory, `evidence/${label}.stdout.log`), 'utf8');
  const stderr = readFileSync(safePath(directory, `evidence/${label}.stderr.log`), 'utf8');
  if (result.code !== 0 || result.signal !== null || result.stopped !== null || result.spawnError !== null ||
      result.groupAbsent !== true || result.outputTruncated !== false || (program && result.program !== program) ||
      !Number.isSafeInteger(result.processGroup) || result.processGroup <= 0 || !Array.isArray(result.args) ||
      result.stdoutBytes !== Buffer.byteLength(stdout) || result.stderrBytes !== Buffer.byteLength(stderr) ||
      result.seenBytes !== result.stdoutBytes + result.stderrBytes || result.seenBytes > 8 * 1024 * 1024)
    throw Error('installed AI process incomplete');
  return { result, stdout, stderr };
}
export function requireInstalledAiObservation(report, observation) {
  const expected = Object.fromEntries(report.source.files.map(file => [file.path, file.sha256]));
  if (!isDeepStrictEqual(observation.producer, expected) || !isDeepStrictEqual(observation.extracted_producer, expected) ||
      observation.source_archive_sha256 !== report.source_archive_sha256 ||
      !isDeepStrictEqual(observation.crate_archives, report.crate_archives.map(({ path, sha256 }) => ({ path, sha256 }))))
    throw Error('installed AI final inputs differ');
  for (const kind of ['packages', 'consumers']) {
    const items = report[kind], actual = observation[kind];
    if (!Array.isArray(items) || !Array.isArray(actual) || items.length !== actual.length ||
        new Set(actual.map(item => item.name)).size !== actual.length) throw Error('installed AI final inputs differ');
    for (const item of items) {
      const found = actual.find(other => other.name === item.name);
      if (!found || !isDeepStrictEqual(found.expected, item.files) || !isDeepStrictEqual(found.observed, item.files))
        throw Error('installed AI final inputs differ');
    }
  }
  return true;
}
export function requireInstalledAiExecution(directory, expectedSource) {
  const report=JSON.parse(readFileSync(safePath(directory,'acceptance.json'),'utf8'));
  if(report.completed!==true||report.source_archive_sha256!==expectedSource.archive_sha256||report.source?.source_inventory_sha256!==expectedSource.source_inventory_sha256||!Array.isArray(report.source.files)||digest(report.source.files)!==expectedSource.source_inventory_sha256||report.source?.source_mode!=='committed-release-source'||report.offline!==true||report.cargo_jobs!==2||report.incremental!==false||report.publication!==false||report.results?.length!==4)throw Error('invalid installed AI execution');
  for(const [name,kind] of [['rom-ai-flows-consumer','flows'],['rom-ai','ai'],['rom-openrouter','adapter']]){
    const execution=requireAiProcessEvidence(directory, `${name}-tests`, 'cargo');
    if(!['test','--offline','--locked'].every(arg=>execution.result.args.includes(arg)))throw Error('installed AI test command differs');
    const observed=requireAiResults(execution.stdout,kind);
    const claimed=report.results.find(item=>item.name===name);if(claimed?.passed!==observed.passed||claimed.failed!==0||claimed.ignored!==0)throw Error('installed AI result differs');
  }
  for(const name of ['rom-ai-flows-consumer','rom-ai-tools-compile','rom-ai','rom-openrouter'])for(const phase of ['resolve','metadata','final-metadata']){
    const execution=requireAiProcessEvidence(directory,`${name}-${phase}`,'cargo');
    if(!execution.result.args.includes('metadata')||!execution.result.args.includes('--offline')||(phase!=='resolve'&&!execution.result.args.includes('--locked')))throw Error('installed AI metadata command differs');
  }
  const compileExecution=requireAiProcessEvidence(directory,'public-readonly-compile');
  const compilerExecution=requireAiProcessEvidence(directory,'compiler','rustc');
  if(compilerExecution.result.args.length!==1||compilerExecution.result.args[0]!=='-Vv')throw Error('installed AI compiler command differs');
  const compile=report.results.find(item=>item.name==='rom-ai-tools-compile');
  if(compile?.positive_apis!==true||compile.negative_execute!=='E0599:cannot_execute.rs:4'||!compileExecution.stdout.includes('execute correctly rejected with E0599 at source line 4'))throw Error('missing installed tool compile acceptance');
  requireAiCompiler(compilerExecution.stdout,report.compiler.release);
  requireInstalledAiObservation(report,JSON.parse(readFileSync(safePath(directory,'final-input-observation.json'),'utf8')));
  return report;
}
export async function runSupplementalConsumers(lease, workspace, source, archive, runner=runChild) {
  const directory=join(workspace,'supplemental');mkdirSync(directory);mkdirSync(join(directory,'evidence'));mkdirSync(join(directory,'tmp'));
  const target=join(directory,'package-target'),env={...process.env,CARGO_TARGET_DIR:target,ROM_PACKAGE_TARGET_DIR:target,TMPDIR:join(directory,'tmp'),CARGO_INCREMENTAL:'0',CARGO_NET_OFFLINE:'true'};
  const selectedRunner=(program,args,options)=>runner(program,args,{...options,env:{...options.env,...env},timeout:2400000,maxBytes:32*1024*1024});
  lease.fence();
  const packaged=await recordedCommand('node',[join(lease.root,'scripts/check-packages.mjs'),lease.root],lease.root,directory,'packages',selectedRunner);
  if(packaged.exit_code!==0||packaged.bounded_abort||packaged.spawn_failed)throw Error('extracted package acceptance failed');
  const stdout=readFileSync(join(directory,packaged.stdout),'utf8');requireDiagnosticExecution(stdout);
  const matches=[...stdout.matchAll(/^Packaged consumer and reference application passed using \d+ archives\. Evidence retained: (.+)$/gm)];
  if(matches.length!==1)throw Error('package execution evidence missing');
  const packageDirectory=matches[0][1];if(realpathSync(packageDirectory)!==packageDirectory||!packageDirectory.startsWith(join(directory,'tmp')+'/'))throw Error('package execution evidence escaped');
  const packages=JSON.parse(readFileSync(safePath(packageDirectory,'acceptance.json'),'utf8'));
  if(packages.source_lock_sha256!==lease.witness.lock_sha256||!Array.isArray(packages.libraries)||!packages.libraries.length||packages.libraries.length>32||packages.publication!==false)throw Error('package execution source mismatch');
  const archives=packages.libraries.map(item=>{
    if(!/^rom(?:-[a-z-]+)?$/.test(item.name)||item.version!==source.package_version)throw Error('package identity mismatch');
    const prefix=`${item.name}-${item.version}`,path=join(target,'package',prefix+'.crate'),stat=lstatSync(path);
    if(!stat.isFile()||stat.nlink!==1||hash(path)!==item.archive_sha256)throw Error('package archive changed');
    return{path,prefix,sha256:item.archive_sha256};
  });
  const compiler=/^rustc (\d+\.\d+\.\d+)/.exec(source.tools?.rustc??'')?.[1];if(!compiler)throw Error('missing selected compiler');
  const options={sourceRoot:lease.root,sourceArchive:archive.path,sourceArchiveSha256:archive.sha256,sourcePrefix:archive.prefix,source:{source_mode:'committed-release-source',...source,source_inventory_sha256:digest(source.files)},crateArchives:archives,output:join(directory,'ai'),target:join(directory,'ai-target'),compiler,metadataSeconds:60,testSeconds:600,compileSeconds:600,outputBytes:8*1024*1024};
  const optionsPath=join(directory,'ai-options.json');writeFileSync(optionsPath,JSON.stringify(options,null,2)+'\n',{flag:'wx'});
  lease.fence();
  const ai=await recordedCommand('node',[join(lease.root,'scripts/packages/ai-flows-consumer.mjs'),optionsPath],lease.root,directory,'ai',selectedRunner);
  if(ai.exit_code!==0||ai.bounded_abort||ai.spawn_failed)throw Error('extracted installed AI acceptance failed');
  const report=requireInstalledAiExecution(options.output,lease.witness);lease.fence();
  return{directory,packageDirectory,packages,archives,commands:[packaged,ai],ai:report,diagnostics:requireDiagnosticExecution(stdout)};
}
