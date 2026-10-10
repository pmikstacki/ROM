// Finite, host-owned process groups for isolated consumer adoption.
import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';

function signalGroup(pid, signal) {
  try { process.kill(-pid,signal); return true; }
  catch(error) { if(error.code==='ESRCH')return false; throw error; }
}
async function finishGroup(pid) {
  if(!signalGroup(pid,'SIGTERM'))return true;
  for(let i=0;i<10;i++){await delay(20);if(!signalGroup(pid,0))return true;}
  signalGroup(pid,'SIGKILL');
  for(let i=0;i<50;i++){await delay(20);if(!signalGroup(pid,0))return true;}
  return false;
}

export async function command(record,args,label,seconds,outputBytes,program='cargo') {
  const env={...process.env,CARGO_BUILD_JOBS:'2',CARGO_INCREMENTAL:'0',CARGO_TARGET_DIR:record.target};
  const child=spawn(program,args,{cwd:record.app,env,detached:true,stdio:['ignore','pipe','pipe']});
  let stdout=Buffer.alloc(0),stderr=Buffer.alloc(0),stopped=null,escalation,seen=0;
  const stop=reason=>{
    if(stopped)return;
    stopped=reason;
    if(child.pid)signalGroup(child.pid,'SIGTERM');
    escalation=setTimeout(()=>{if(child.pid)signalGroup(child.pid,'SIGKILL');},1000);
  };
  const interrupted=signal=>()=>stop(`interrupted:${signal}`);
  const onInt=interrupted('SIGINT'),onTerm=interrupted('SIGTERM');
  process.once('SIGINT',onInt);process.once('SIGTERM',onTerm);
  const timer=setTimeout(()=>stop('deadline'),seconds*1000);
  for(const [stream,key] of [[child.stdout,'stdout'],[child.stderr,'stderr']])stream.on('data',chunk=>{
    seen+=chunk.length;
    const kept=chunk.subarray(0,Math.max(0,outputBytes-stdout.length-stderr.length));
    if(key==='stdout')stdout=Buffer.concat([stdout,kept]);else stderr=Buffer.concat([stderr,kept]);
    if(seen>outputBytes)stop('output budget');
  });
  let code=null,signal=null,spawnError=null,groupAbsent=false;
  try {
    await new Promise(resolve=>{
      child.once('error',error=>{spawnError=error;resolve();});
      child.once('close',(exit,exitSignal)=>{code=exit;signal=exitSignal;resolve();});
    });
    // A successful leader can leave descendants with closed stdio behind.
    // Cleanup applies to success, failure and interruption alike.
    if(child.pid)groupAbsent=await finishGroup(child.pid);else groupAbsent=true;
    writeFileSync(join(record.evidence,`${label}.stdout.log`),stdout);
    writeFileSync(join(record.evidence,`${label}.stderr.log`),stderr);
    const result={program,args,code,signal,stopped,spawnError:spawnError?.code??null,processGroup:child.pid??null,groupAbsent,stdoutBytes:stdout.length,stderrBytes:stderr.length,seenBytes:seen,outputTruncated:seen>outputBytes};
    writeFileSync(join(record.evidence,`${label}.result.json`),JSON.stringify(result,null,2)+'\n');
    if(code!==0||stopped||spawnError||!groupAbsent)throw Error(`adoption command failed: ${label}`);
    return stdout.toString('utf8');
  } finally {
    clearTimeout(timer);clearTimeout(escalation);
    process.removeListener('SIGINT',onInt);process.removeListener('SIGTERM',onTerm);
    if(child.pid&&!groupAbsent)signalGroup(child.pid,'SIGKILL');
  }
}
