// Real production acceptance executes only from the verified full source extraction.
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { runChild } from '../skills/process.mjs';
import { hash } from '../skills/files.mjs';
import { withVerifiedExtraction } from './consumer-source.mjs';
import { prepareConsumerHost } from './native-preparation.mjs';
import { runConsumerGates } from './consumers.mjs';
import { sqliteProfileEnvironment, fenceSqliteProfile } from './sqlite-profile.mjs';
import { runSupplementalConsumers } from './supplemental-consumers.mjs';
import { retainRegularEvidence, consumerRetentionPaths, recoveryRetentionPaths } from './production-evidence.mjs';
import { retainProductionRequirements } from './production-inputs.mjs';

export async function executeProductionAcceptance({root,stage,source,artifacts,frontend,assets,results,tools,requirements,engine},runner=runChild) {
  const workspace=mkdtempSync(join(dirname(stage),'.rom-production-work-'));
  return withVerifiedExtraction(stage,source,artifacts,frontend,async lease=>{
    fenceSqliteProfile(engine);
    const selectedRunner=(program,args,options)=>runner(program,args,{...options,env:sqliteProfileEnvironment(options.env,engine.directory)});
    const native=await prepareConsumerHost(lease,join(workspace,'native'),selectedRunner,engine);
    const context={extracted_root:lease.root,evidence_root:join(workspace,'consumers'),host_binary:native.binary,...Object.fromEntries(['revision','tree','source_inventory_sha256','archive_sha256','studio_sha256'].map(key=>[key,lease.witness[key]]))};
    await runConsumerGates(lease,{root,stage,assets,results,context,native:{directory:join(workspace,'native'),record:native.record}},selectedRunner);
    const supplemental=await runSupplementalConsumers(lease,workspace,{...source,tools},{path:join(stage,artifacts.source.path),sha256:artifacts.source.sha256,prefix:artifacts.source.prefix},selectedRunner);
    const retained=join(stage,'evidence/production');mkdirSync(retained);mkdirSync(join(retained,'consumers'));
    for(const kind of ['controls','recovery-sqlite','recovery-redb'])retainRegularEvidence(join(context.evidence_root,kind),join(retained,'consumers',kind),[...consumerRetentionPaths,...(kind==='controls'?[]:recoveryRetentionPaths)]);
    retainRegularEvidence(join(workspace,'native'),join(retained,'native'),['evidence','target/debug/source.json','target/debug/rom-recovery-host']);
    retainRegularEvidence(engine.directory,join(retained,'sqlite'),['evidence','profile.json','sqlite-autoconf-3530400.tar.gz','source/sqlite-autoconf-3530400/sqlite3.c','include','lib']);
    retainRegularEvidence(supplemental.directory,join(retained,'supplemental'),['evidence','ai-options.json','ai/acceptance.json','ai/final-input-observation.json','ai/evidence']);
    mkdirSync(join(retained,'crates'));for(const archive of supplemental.archives)retainRegularEvidence(dirname(archive.path),join(retained,'crates',archive.prefix),[archive.prefix+'.crate']);
    const packageRecord=readFileSync(join(supplemental.packageDirectory,'acceptance.json'));
    writeFileSync(join(retained,'supplemental/package-acceptance.json'),packageRecord,{flag:'wx'});
    const ledger=retainProductionRequirements(requirements,stage);lease.fence();fenceSqliteProfile(engine);
    return{schema_version:1,workspace,context,source_witness:lease.witness,retained_directory:'evidence/production',native:{record:native.record,binary_sha256:hash(native.binary)},sqlite:engine.record,supplemental:{commands:supplemental.commands,package_directory:supplemental.packageDirectory,crate_archives:supplemental.archives},requirements_path:ledger.path};
  });
}
