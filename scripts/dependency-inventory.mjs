#!/usr/bin/env node
// Record actual Cargo metadata and bundled notices for the all-features graph.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
const root=resolve(process.argv[2]??'.');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--all-features','--format-version','1'],{cwd:root,encoding:'utf8',maxBuffer:32*1024*1024}));
const packages=metadata.packages.filter(p=>p.source).sort((a,b)=>a.name.localeCompare(b.name)||a.version.localeCompare(b.version));
const hash=createHash('sha256').update(readFileSync(join(root,'Cargo.lock'))).digest('hex');
let inventory=`# Maintained dependency inventory\n\nGenerated from the all-features Cargo graph (including target-specific and dev dependencies).\nLockfile SHA-256: \`${hash}\`. This records declarations, not a blanket license or vulnerability certification. Native vendored code may carry additional notices.\n\n| Crate | Version | Declared license | Declared Rust floor |\n|---|---|---|---|\n`;
let notices=`# Third-party notices\n\nROM is MIT licensed; dependencies retain their own licenses. This is a source-notice collection from the locked all-features Cargo graph, including optional/target-specific/dev packages. It does not imply every package ships in every executable. Cargo source archives preserve upstream distributions.\n\nLockfile SHA-256: \`${hash}\`.\n\nSQLite is in the public domain; see https://sqlite.org/copyright.html. The optional native profile preserves the official amalgamation headers.\n\n`;
function noticeFiles(dir,depth=0){
  return readdirSync(dir).flatMap(name=>{
    const path=join(dir,name),st=statSync(path);
    if(st.isFile()&&/^(licen[cs]e|copying|copyright|notice)([-._]|$)/i.test(name))return [path];
    if(st.isDirectory()&&depth<2&&/^(licenses?|aws-lc)$/i.test(name))return noticeFiles(path,depth+1);
    return [];
  }).sort();
}
for(const p of packages){
  if(!p.license&&!p.license_file)throw Error(`${p.name}: missing license metadata`);
  inventory+=`| ${p.name} | ${p.version} | ${(p.license??'see license file').replaceAll('|','\\|')} | ${p.rust_version??'not declared'} |\n`;
  notices+=`## ${p.name} ${p.version}\n\n${p.license??'License file supplied'}. Source: ${p.repository??p.source}\n\n`;
  const paths=noticeFiles(dirname(p.manifest_path));
  if(p.license_file&&!paths.includes(p.license_file))paths.push(p.license_file);
  if(!paths.length)notices+='No separately named top-level license file was included in this Cargo archive; consult the original crate distribution and declared license.\n\n';
  for(const path of paths){const label=path.slice(dirname(p.manifest_path).length+1); notices+=`### ${label}\n\n\`\`\`text\n${readFileSync(path,'utf8').replaceAll('```','~~~')}\n\`\`\`\n\n`;}
}
writeFileSync(join(root,'docs/research/maintained-dependencies.md'),inventory);
writeFileSync(join(root,'THIRD_PARTY_NOTICES.md'),notices);
console.log(`Inventoried ${packages.length} external packages; lock ${hash}. Review declared expressions and notice coverage before release.`);
