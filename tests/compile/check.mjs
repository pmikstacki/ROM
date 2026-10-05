import {spawnSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
const presentationFailures=[
 ["presentation_duplicate","duplicate presentation option",3],
 ["presentation_duplicate_group","duplicate presentation group",6],
 ["presentation_empty_label","presentation text must contain 1 to 256 UTF-8 bytes",3],
 ["presentation_field_duplicate","duplicate presentation option",5],
 ["presentation_field_malformed","expected string literal",5],
 ["presentation_help_oversized","presentation text must contain 1 to 2048 UTF-8 bytes",6],
 ["presentation_input","expected one crate option",3],
 ["presentation_input_field","expected one rename option",4],
 ["presentation_label_oversized","presentation text must contain 1 to 256 UTF-8 bytes",5],
 ["presentation_malformed","expected parentheses",3],
 ["presentation_settings_duplicate","duplicate settings presentation option",6],
 ["presentation_settings_key_duplicate","duplicate presentation option",5],
 ["presentation_settings_missing","presentation needs label",3],
 ["presentation_title_bool","title_field requires a built-in string field",3],
 ["presentation_title_renamed","title_field must name a canonical Resource field",3],
 ["presentation_unknown_group","unknown presentation group",5],
 ["presentation_unknown_option","expected one name, version or crate option",3],
 ["presentation_wrong_field","expected one rename option",5],
 ["presentation_wrong_type","expected one name, version or crate option",3],
];
const manifest='tests/compile/Cargo.toml';
for(const [name,message,line] of [...presentationFailures,["resource_version_zero","version must be a positive u32 integer",3],["resource_version_duplicate","duplicate version option",3],["resource_version_string","version must be a positive u32 integer",3],["resource_version_float","version must be a positive u32 integer",3],["resource_version_overflow","version must be a positive u32 integer",3],["input_version","Input does not support version",3],['unsupported','Field',5],['serde','rejects independent serde',5],['duplicate','duplicate or empty',6],['wrong_input','mismatched types',6],['wrong_selector','mismatched types',5],['input_unsupported','Field',4],['input_duplicate','duplicate or empty',5],['input_serde','rejects independent serde',4],['input_generic','requires concrete fields',3],['input_duplicate_option','expected one rename',4]]) {
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

const payload=spawnSync('cargo',['run','--manifest-path',manifest,'--locked','--bin','input_renamed'],{stdio:'inherit'});
if(payload.status!==0)process.exit(payload.status??1);
const renamed=spawnSync('cargo',['run','--manifest-path','tests/compile-renamed/Cargo.toml','--locked'],{stdio:'inherit'});
if(renamed.status!==0)process.exit(renamed.status??1);

const versioned=spawnSync('cargo',['run','--manifest-path',manifest,'--locked','--bin','resource_version'],{stdio:'inherit'});
if(versioned.status!==0)process.exit(versioned.status??1);

for(const name of ['presentation_valid','presentation_shadowed_builtin']) {
 const result=spawnSync('cargo',['run','--manifest-path',manifest,'--locked','--bin',name],{stdio:'inherit'});
 if(result.status!==0)process.exit(result.status??1);
}
