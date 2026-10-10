// Static checks supplement actual execution. Root must separately review R1–R14 behavior coverage.
import { readFileSync, lstatSync, mkdtempSync, mkdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';import { tmpdir } from 'node:os';import { isDeepStrictEqual } from 'node:util';
import { safePath, hash } from '../skills/files.mjs';
import { requireNativePreparation } from './native-evidence.mjs';
import { requireConsumerReports } from './consumer-reports.mjs';
import { requireConsumerContext } from './consumer-context.mjs';
import { requireReleaseRequirements } from './requirements.mjs';
import { requireDiagnosticExecution, requireInstalledAiExecution } from './supplemental-consumers.mjs';
import { requireSqliteBuildEvidence } from './sqlite-evidence.mjs';
import { extractCrate } from '../../tests/ai-flows-installed/crate-extraction.mjs';
import { inspectPackages } from '../packages/astral-adoption.mjs';
import { requirePackageProvenance } from '../../tests/ai-flows-installed/provenance.mjs';
const json=path=>JSON.parse(readFileSync(path,'utf8'));
export function requireProductionAcceptance(record,lease,stage,producerRoot) {
  if(!record||record.schema_version!==1||record.retained_directory!=='evidence/production'||record.requirements_path!=='evidence/requirements/record.json'||!isDeepStrictEqual(record.source_witness,lease.witness))throw Error('invalid production source evidence');
  const context=requireConsumerContext(record.context,producerRoot),retained=safePath(stage,record.retained_directory),native=safePath(retained,'native');
  for(const key of ['revision','tree','source_inventory_sha256','archive_sha256','studio_sha256'])if(context[key]!==lease.witness[key])throw Error('production execution context differs');
  const provenance=safePath(native,'target/debug/source.json'),binary=safePath(native,'target/debug/rom-recovery-host'),stat=lstatSync(binary);
  if(!stat.isFile()||stat.nlink!==1||!(stat.mode&0o111)||hash(binary)!==record.native.binary_sha256||!isDeepStrictEqual(json(provenance),record.native.record)||record.native.record.schema_version!==2||context.host_binary!==join(record.native.record.target_directory,'debug/rom-recovery-host'))throw Error('production native identity differs');
  requireNativePreparation(record.native.record,{witness:lease.witness,source_root:context.extracted_root,target_directory:record.native.record.target_directory,binary_sha256:hash(binary)},native);
  const engine=safePath(retained,'sqlite');
  if(!isDeepStrictEqual(record.sqlite,json(safePath(engine,'profile.json')))||!isDeepStrictEqual(record.sqlite,record.native.record.engine_profile))throw Error('production SQLite profile differs');
  requireSqliteBuildEvidence(record.sqlite,engine);
  const consumers=requireConsumerReports(lease,safePath(retained,'consumers'),{binary:context.host_binary,binary_sha256:hash(binary),lock_sha256:lease.witness.lock_sha256,provenance_sha256:hash(provenance)},context);
  const supplemental=safePath(retained,'supplemental'),commands=record.supplemental.commands;
  if(!Array.isArray(commands)||commands.length!==2||commands.some(command=>command.exit_code!==0||command.bounded_abort!==false||command.spawn_failed!==false||command.cwd!==context.extracted_root)||commands[0].program!=='node'||!isDeepStrictEqual(commands[0].args,[join(context.extracted_root,'scripts/check-packages.mjs'),context.extracted_root])||commands[1].program!=='node'||!isDeepStrictEqual(commands[1].args,[join(context.extracted_root,'scripts/packages/ai-flows-consumer.mjs'),join(record.workspace,'supplemental/ai-options.json')]))throw Error('production supplemental commands differ');
  const diagnostics=requireDiagnosticExecution(readFileSync(safePath(supplemental,commands[0].stdout),'utf8'));
  const packages=json(safePath(supplemental,'package-acceptance.json'));
  if(packages.source_lock_sha256!==lease.witness.lock_sha256||packages.publication!==false||!Array.isArray(packages.libraries)||packages.libraries.length!==record.supplemental.crate_archives?.length)throw Error('production package evidence differs');
  const scratch=mkdtempSync(join(tmpdir(),'rom-production-package-review-'));
  try{
    mkdirSync(join(scratch,'packages'));
    for(const archive of record.supplemental.crate_archives){
      if(!/^rom(?:-[a-z-]+)?-\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/.test(archive.prefix))throw Error('invalid retained crate prefix');
      const path=safePath(retained,`crates/${archive.prefix}/${archive.prefix}.crate`);
      const name=archive.prefix.replace(/-\d.*$/,'');const selected=packages.libraries.find(item=>item.name===name);
      if(!selected||`${selected.name}-${selected.version}`!==archive.prefix||selected.archive_sha256!==archive.sha256||hash(path)!==archive.sha256)throw Error('retained crate differs');
      extractCrate(path,join(scratch,'packages'),archive.prefix);
    }
    requirePackageProvenance(inspectPackages(join(scratch,'packages')),lease.root);
  }finally{rmSync(scratch,{recursive:true,force:true});}
  const ai=requireInstalledAiExecution(safePath(supplemental,'ai'),lease.witness);
  if(!isDeepStrictEqual(ai.crate_archives,record.supplemental.crate_archives))throw Error('installed AI archives differ');
  const requirementsDirectory=safePath(stage,'evidence/requirements'),requirements=requireReleaseRequirements(json(safePath(stage,record.requirements_path)),lease.witness.source_inventory_sha256,requirementsDirectory,{requireComplete:true});
  lease.fence();return{consumers,diagnostics,requirements,ai_completed:ai.completed,behavior_review_required:true};
}
