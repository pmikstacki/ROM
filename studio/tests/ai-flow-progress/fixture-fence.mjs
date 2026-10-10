// Byte identities for maintained authoring inputs and their isolated browser copies.
import {lstatSync,readdirSync,readFileSync} from 'node:fs';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {isDeepStrictEqual} from 'node:util';
const generated=new Set(['consumer/node_modules','consumer/dist','consumer/test-results','consumer/playwright-report','host/target']);
export const runtimeFiles=['proxy.mjs','server.mjs','native-process.mjs','protocol.mjs','failure-observation.mjs'];
function inventory(root,starts){
  const files={};function visit(path){if(generated.has(path))return;const full=join(root,path),info=lstatSync(full);if(info.isDirectory()){for(const entry of readdirSync(full).sort())visit(path?`${path}/${entry}`:entry);}else if(info.isFile())files[path]=createHash('sha256').update(readFileSync(full)).digest('hex');else throw Error('fixture contains nonregular input');}
  for(const start of starts)visit(start);return files;
}
export function fixtureSources(root){return inventory(root,['']);}
export function copiedSources(root){return inventory(root,['consumer','runtime']);}
export function selectedCopyInputs(maintained){return Object.fromEntries(Object.entries(maintained).flatMap(([path,hash])=>path.startsWith('consumer/')?[[path,hash]]:runtimeFiles.includes(path)?[[`runtime/${path}`,hash]]:[]));}
export function requireCopiedInputs(maintained,copied){if(!isDeepStrictEqual(selectedCopyInputs(maintained),copied))throw Error('copied fixture input changed');}
export function requireFixtureFreeze(value,current){
  if(!value||!Array.isArray(value.files)||!value.files.length)throw Error('independent fixture freeze required');
  const selected={};for(const entry of value.files){const prefix='studio/tests/ai-flow-progress/';if(typeof entry.path!=='string'||!entry.path.startsWith(prefix)||!/^[0-9a-f]{64}$/.test(entry.sha256??'')||Object.hasOwn(selected,entry.path.slice(prefix.length)))throw Error('invalid fixture freeze');selected[entry.path.slice(prefix.length)]=entry.sha256;}
  if(!isDeepStrictEqual(selected,current))throw Error('maintained fixture differs from selected freeze');return selected;
}
