// Measure application rebuilds with cached dependencies. This is not a cold dependency benchmark.
import {execFileSync} from 'node:child_process';
import {statSync,utimesSync,writeFileSync} from 'node:fs';
const target=process.env.CARGO_TARGET_DIR;if(!target)throw Error('CARGO_TARGET_DIR required');
let csv='phase,strategy,sample,seconds,binary_bytes\n';
const names=['generated','hybrid','runtime'];
for(let sample=0;sample<5;sample++)for(let offset=0;offset<names.length;offset++){
 const feature=names[(sample+offset)%names.length];
 execFileSync('cargo',['clean','-p','rom-query-planning-prototype','--release'],{stdio:'pipe'});
 for(const phase of ['application_clean','main_edit']){
  if(phase==='main_edit'){const now=new Date;utimesSync('src/main.rs',now,now);}
  const start=process.hrtime.bigint();
  execFileSync('cargo',['build','--release','--locked','--offline','--no-default-features','--features',feature],{stdio:'pipe'});
  const seconds=Number(process.hrtime.bigint()-start)/1e9;
  const bytes=statSync(target+'/release/rom-query-planning-prototype').size;
  csv+=`${phase},${feature},${sample},${seconds.toFixed(6)},${bytes}\n`;
  writeFileSync('results/build-cost.csv',csv);
 }
}
