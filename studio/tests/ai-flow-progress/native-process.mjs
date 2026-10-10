// Fixture-owned native child; bounded logs, startup, lifetime and terminal drain.
import { spawn } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import {redactedLog,nativeProcessThreads} from './failure-observation.mjs';
export async function launchHost({binary, adapter, database, domain, mode, evidence}) {
  const key = randomBytes(32).toString('hex');
  const child = spawn(binary, [adapter, database, domain, mode, '0', '60000'], { env: { ...process.env, ROM_AI_FIXTURE_KEY: key }, detached: true, stdio: ['ignore','pipe','pipe'] });
  let address, bytes = 0, closed = false, phase = 'native_spawn', exit = {}, stdoutBytes = 0, stderrBytes = 0;
  const logs = new Map(['stdout','stderr'].map(label => [label,redactedLog(text=>{if(text)appendFileSync(join(evidence,`${domain}-${mode}-host-${label}.log`),text);},[key,process.env.ROM_AI_BROWSER_CONTROL])]));
  const terminal = new Promise(resolve => { child.once('exit', (code,signal) => {closed=true;exit={code,signal};resolve(exit);}); child.once('error', error => {closed=true;exit={spawn_errno:error.code};resolve(exit);}); });
  const signal = value => { if(child.pid) {try {process.kill(-child.pid,value);} catch(error) {if(error.code!=='ESRCH')throw error;} } };
  const lifetime = setTimeout(()=>signal('SIGKILL'),65000);
  let stopped;
  async function stop() {
    if(stopped)return stopped;
    stopped=(async()=>{
      if(address&&!closed)try {await request('/control',{release:true});} catch {}
      if(!closed)signal('SIGTERM');
      const escalation=setTimeout(()=>signal('SIGKILL'),1000);
      let deadline;
      try { await Promise.race([terminal,new Promise((_,reject)=>{deadline=setTimeout(()=>reject(Error('native child drain deadline')),4000);})]); }
      finally {clearTimeout(deadline);clearTimeout(escalation);clearTimeout(lifetime);signal('SIGKILL');child.stdout.destroy();child.stderr.destroy();for(const logger of logs.values())logger.flush();}
    })();return stopped;
  }
  async function request(route,body={}) {
    if(!address||closed)throw Error('native fixture unavailable');
    const response=await fetch(`http://${address}${route}`,{method:'POST',headers:{'content-type':'application/json','x-fixture-key':key},body:JSON.stringify(body),redirect:'error',signal:AbortSignal.timeout(22000)});
    const reader=response.body.getReader();let seen=0;const chunks=[];
    try {for(;;){const {done,value}=await reader.read();if(done)break;seen+=value.length;if(seen>65536)throw Error('native projection byte limit');chunks.push(Buffer.from(value));}}
    finally {await reader.cancel().catch(()=>{});}
    return {status:response.status,text:Buffer.concat(chunks).toString('utf8')};
  }
  try {
    await new Promise((resolve,reject)=>{
      phase='native_ready_wait';let line='';const timer=setTimeout(()=>reject(Error('native startup deadline')),10000);
      terminal.then(result=>{clearTimeout(timer);reject(Error(`native exited before ready: ${JSON.stringify(result)}`));});
      for(const [stream,label] of [[child.stdout,'stdout'],[child.stderr,'stderr']])stream.on('data',chunk=>{
        bytes+=chunk.length;if(label==='stdout')stdoutBytes+=chunk.length;else stderrBytes+=chunk.length;if(bytes>1048576){phase='native_log_budget';signal('SIGKILL');reject(Error('native log budget'));return;}
        logs.get(label).append(chunk);
        if(label==='stdout'&&!address){line+=chunk.toString('utf8');if(line.includes('\n'))try {
          const ready=JSON.parse(line.split('\n')[0]);if(!/^127\.0\.0\.1:\d+$/.test(ready.address)||ready.adapter!==adapter||ready.domain!==domain)throw Error('invalid native ready identity');address=ready.address;clearTimeout(timer);resolve();
        }catch(error){phase='native_ready_identity';clearTimeout(timer);reject(error);}}
      });
    });
    return {request,stop};
  }catch(error){const threads=nativeProcessThreads(child.pid);try {await stop();}catch { /* Preserve the primary stage and terminal observation. */ }Object.assign(error,{fixtureStage:phase,nativeObservation:{...exit,stdout_bytes:stdoutBytes,stderr_bytes:stderrBytes,threads}});throw error;}
}
