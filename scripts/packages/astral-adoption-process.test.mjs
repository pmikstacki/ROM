import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtempSync,writeFileSync,readFileSync,existsSync,rmSync,mkdirSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import {command} from './astral-adoption.mjs';

function running(pid) {
  try {
    const stat=readFileSync(`/proc/${pid}/stat`,'utf8');
    return stat.slice(stat.lastIndexOf(')')+2).split(' ')[0]!=='Z';
  } catch(error) { if(error.code==='ENOENT')return false; throw error; }
}
async function waitFor(predicate) {
  for(let i=0;i<100;i++){if(predicate())return;await delay(20);}
  throw Error('fixture boundary did not settle');
}
function fixture() {
  const root=mkdtempSync(join(tmpdir(),'rom-adoption-process-test-'));
  const target=join(root,'target');mkdirSync(target);
  return {root,record:{app:root,evidence:root,target}};
}

test('successful tool cannot leave a live stdio-closed child',async()=>{
  const {root,record}=fixture(),marker=join(root,'child.pid'),tool=join(root,'tool.mjs');
  writeFileSync(tool,`import{spawn}from'node:child_process';import{writeFileSync}from'node:fs';const c=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});writeFileSync(${JSON.stringify(marker)},String(c.pid));setTimeout(()=>process.exit(0),100);`);
  let pid;
  try {
    await command(record,[tool],'successful',3,4096,process.execPath);
    pid=Number(readFileSync(marker,'utf8'));
    await waitFor(()=>!running(pid));
    const result=JSON.parse(readFileSync(join(root,'successful.result.json')));
    assert.equal(result.code,0);
  } finally {
    if(!pid&&existsSync(marker))pid=Number(readFileSync(marker,'utf8'));
    if(pid&&running(pid))process.kill(pid,'SIGKILL');
    rmSync(root,{recursive:true,force:true});
  }
});

for(const interrupt of ['SIGTERM','SIGINT'])test(`${interrupt} terminates the active owned tool group`,async()=>{
  const {root,record}=fixture(),marker=join(root,'tool.pid'),tool=join(root,'tool.mjs'),wrapper=join(root,'wrapper.mjs');
  writeFileSync(tool,`import{writeFileSync}from'node:fs';writeFileSync(${JSON.stringify(marker)},String(process.pid));setInterval(()=>{},1000);`);
  writeFileSync(wrapper,`import{command}from${JSON.stringify(new URL('./astral-adoption.mjs',import.meta.url).href)};try{await command(${JSON.stringify(record)},[${JSON.stringify(tool)}],'interrupted',10,4096,process.execPath);}catch{process.exitCode=1;}`);
  const child=spawn(process.execPath,[wrapper],{stdio:'ignore'});
  const closed=new Promise(resolve=>child.once('close',(code,signal)=>resolve({code,signal})));
  let pid;
  try {
    await waitFor(()=>existsSync(marker));pid=Number(readFileSync(marker,'utf8'));
    assert.ok(running(pid));child.kill(interrupt);
    await Promise.race([closed,delay(4000).then(()=>{throw Error('runner did not stop');})]);
    await waitFor(()=>!running(pid));
    const result=JSON.parse(readFileSync(join(root,'interrupted.result.json')));
    assert.equal(result.stopped,`interrupted:${interrupt}`);
  } finally {
    if(!pid&&existsSync(marker))pid=Number(readFileSync(marker,'utf8'));
    if(pid&&running(pid))process.kill(-pid,'SIGKILL');
    child.kill('SIGKILL');await closed;
    rmSync(root,{recursive:true,force:true});
  }
});

for(const failure of ['deadline','output budget'])test(`${failure} preserves failure and stops descendants`,async()=>{
  const {root,record}=fixture(),marker=join(root,'child.pid'),tool=join(root,'tool.mjs');
  writeFileSync(tool,`import{spawn}from'node:child_process';import{writeFileSync}from'node:fs';const c=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});writeFileSync(${JSON.stringify(marker)},String(c.pid));${failure==='output budget'?'process.stdout.write("x".repeat(8192));':''}setInterval(()=>{},1000);`);
  let pid;
  try {
    await assert.rejects(command(record,[tool],'bounded',1,4096,process.execPath),/command failed: bounded/);
    pid=Number(readFileSync(marker,'utf8'));await waitFor(()=>!running(pid));
    const result=JSON.parse(readFileSync(join(root,'bounded.result.json')));
    assert.equal(result.stopped,failure);
    assert.equal(result.groupAbsent,true);
    assert.ok(result.stdoutBytes+result.stderrBytes<=4096);
    assert.equal(result.outputTruncated,failure==='output budget');
  } finally {
    if(!pid&&existsSync(marker))pid=Number(readFileSync(marker,'utf8'));
    if(pid&&running(pid))process.kill(pid,'SIGKILL');
    rmSync(root,{recursive:true,force:true});
  }
});
