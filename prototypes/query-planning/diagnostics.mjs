// Compile-negative authoring probes; fixtures stay outside normal Cargo test targets.
import {execFileSync} from 'node:child_process';
import {mkdirSync,writeFileSync} from 'node:fs';
const root='/var/tmp/rom-query-probe-diagnostics';mkdirSync(root+'/src',{recursive:true});
let report='';
for(const [feature,expression] of [['generated','generated::InventoryQuery::new().amount_ge("wrong")'],['hybrid','hybrid::AMOUNT.ge("wrong")']]){
 writeFileSync(root+'/Cargo.toml',`[package]\nname="query-probe-negative"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\nrom-query-planning-prototype={path=${JSON.stringify(process.cwd())},default-features=false,features=["${feature}"]}\n[profile.dev]\ndebug=0\n`);
 writeFileSync(root+'/src/main.rs',`use rom_query_planning_prototype::*;fn main(){let _=${expression};}\n`);
 let failed=false;try{execFileSync('cargo',['check','--offline','--manifest-path',root+'/Cargo.toml'],{stdio:'pipe'});}catch(e){const out=e.stderr.toString();if(!out.includes('expected `u64`, found `&str`'))throw e;report+=`${feature}: expected compile failure (E0308): expected u64, found &str\n`;failed=true;}
 if(!failed)throw Error(feature+' incorrectly accepted string operand');
}
writeFileSync('results/diagnostics.txt',report);process.stdout.write(report);
