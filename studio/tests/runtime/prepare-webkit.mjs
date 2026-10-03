import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const manifestPath=fileURLToPath(new URL('./webkit-libraries.json',import.meta.url));
const manifestText=fs.readFileSync(manifestPath,'utf8'),manifest=JSON.parse(manifestText);
const source=path.resolve(process.env.ROM_PLAYWRIGHT_WEBKIT_SOURCE??path.join(os.homedir(),'.cache/ms-playwright',`webkit-${manifest.revision}`));
const destination=path.resolve(process.env.ROM_STUDIO_WEBKIT_RUNTIME??`/var/tmp/rom-studio-webkit-${manifest.revision}`);
const hash=data=>crypto.createHash('sha256').update(data).digest('hex');
const manifestHash=hash(manifestText);
if(destination===source||destination.startsWith(source+path.sep)||source.startsWith(destination+path.sep))throw Error('Runtime copy must be separate from downloaded source.');
function treeHash(directory){const digest=crypto.createHash('sha256');function visit(relative){for(const entry of fs.readdirSync(path.join(directory,relative),{withFileTypes:true}).sort((a,b)=>a.name<b.name?-1:1)){const name=path.posix.join(relative,entry.name),target=path.join(directory,name);if(entry.isDirectory())visit(name);else {digest.update(name+'\0');digest.update(entry.isSymbolicLink()?'link:'+fs.readlinkSync(target):hash(fs.readFileSync(target)));digest.update('\0');}}}visit('');return digest.digest('hex');}
if(treeHash(path.join(source,'minibrowser-wpe'))!==manifest.wpeTreeHash)throw Error('Downloaded WPE tree differs from accepted revision.');
for(const [relative,expected] of Object.entries(manifest.sourceHashes))if(hash(fs.readFileSync(path.join(source,relative)))!==expected)throw Error(`Downloaded source hash mismatch: ${relative}`);
const stampPath=path.join(destination,'rom-runtime.json');
if(fs.existsSync(destination)){
 const stamp=JSON.parse(fs.readFileSync(stampPath,'utf8'));
 if(stamp.manifestHash!==manifestHash||stamp.patchedTreeHash!==treeHash(path.join(destination,'minibrowser-wpe')))throw Error('Existing runtime is unrecognized or changed; choose a new destination.');
 console.log(path.join(destination,'pw_run.sh'));
 process.exit(0);
}
const toolRoots=[manifest.loader,manifest.patchelf].map(p=>p.split('/').slice(0,4).join('/'));
for(const store of [...new Set([...manifest.stores,...toolRoots])])if(!fs.existsSync(store))execFileSync('nix-store',['--realise',store],{stdio:['ignore','ignore','inherit']});
fs.mkdirSync(destination,{recursive:true});
fs.cpSync(path.join(source,'minibrowser-wpe'),path.join(destination,'minibrowser-wpe'),{recursive:true,dereference:false});
fs.copyFileSync(path.join(source,'pw_run.sh'),path.join(destination,'pw_run.sh'));
fs.chmodSync(path.join(destination,'pw_run.sh'),0o755);
const libraryPath=manifest.stores.map(store=>store+'/lib').join(':');
for(const binary of ['MiniBrowser','WPEGPUProcess','WPENetworkProcess','WPEWebProcess'])execFileSync(manifest.patchelf,['--set-interpreter',manifest.loader,'--set-rpath',`$ORIGIN/../lib:$ORIGIN/../sys/lib:${libraryPath}`,path.join(destination,'minibrowser-wpe/bin',binary)]);
const wrapper=path.join(destination,'minibrowser-wpe/MiniBrowser'),original=fs.readFileSync(wrapper,'utf8');
const needle='export LD_LIBRARY_PATH="${MYDIR}/lib:${MYDIR}/sys/lib"';
if(original.split(needle).length!==2)throw Error('Upstream wrapper contract changed.');
fs.writeFileSync(wrapper,original.replace(needle,`export LD_LIBRARY_PATH="\${MYDIR}/lib:\${MYDIR}/sys/lib:${libraryPath}"`));
fs.chmodSync(wrapper,0o755);
fs.writeFileSync(stampPath,JSON.stringify({revision:manifest.revision,manifestHash,source,wpeTreeHash:manifest.wpeTreeHash,patchedTreeHash:treeHash(path.join(destination,'minibrowser-wpe')),loader:manifest.loader,stores:manifest.stores},null,2)+'\n');
console.log(path.join(destination,'pw_run.sh'));
