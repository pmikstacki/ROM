import {spawnSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
const manifest='tests/compile/Cargo.toml';
for(const [name,message,line] of [['unsupported','Field',5],['serde','rejects independent serde',5],['duplicate','duplicate or empty',6],['wrong_input','mismatched types',6],['wrong_selector','mismatched types',5]]) {
 const result=spawnSync('cargo',['check','--manifest-path',manifest,'--locked','--bin',name,'--message-format=json'],{encoding:'utf8'});
 const diagnostics=result.stdout.split('\n').filter(Boolean).flatMap(l=>{try{const v=JSON.parse(l);return v.reason==='compiler-message'?[v.message]:[];}catch{return [];}});
 const error=diagnostics.find(d=>d.level==='error'&&d.message.includes(message)&&d.spans.some(s=>s.is_primary&&s.file_name.endsWith(`${name}.rs`)&&s.line_start===line));
 if(result.status===0||!error){console.error(result.stderr,result.stdout);throw Error(`${name}: missing expected primary diagnostic at source line ${line}`);}
 console.log(`Expected compile failure: ${name}, primary line ${line}`);
}
const result=spawnSync('cargo',['run','--manifest-path',manifest,'--locked','--bin','renamed'],{stdio:'inherit'});
if(result.status!==0)process.exit(result.status??1);
const documented=spawnSync('cargo',['check','--manifest-path',manifest,'--locked','--bin','documented'],{stdio:'inherit'});
if(documented.status!==0)process.exit(documented.status??1);
const manifestText=readFileSync('Cargo.toml','utf8');
if(!manifestText.includes('rust-version = "1.99"'))throw Error('toolchain floor changed without verifier update');
