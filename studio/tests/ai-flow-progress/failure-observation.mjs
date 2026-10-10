// Private bounded observations contain fixed categories, never request/error payloads or authority.
import {StringDecoder} from 'node:string_decoder';
import {appendFileSync,readFileSync} from 'node:fs';
import {join} from 'node:path';
const stages=new Set(['request','control_authority','case_directory','native_launch','native_spawn','native_ready_wait','native_ready_identity','native_log_budget','native_stop','owner_authority','native_request','static']);
const errno=value=>typeof value==='string'&&/^[A-Z][A-Z0-9_]{0,31}$/.test(value)?value:null;
export function failureObservation(value){
  const native=value.native??{};
  return {stage:stages.has(value.stage)?value.stage:'request',route:['/__fixture/start','/__fixture/restart','/__fixture/control','/__fixture/inspect','/api/view','/api/resume','/api/cancel','/api/logout'].includes(value.route)?value.route:null,case:typeof value.case==='string'&&/^[a-z0-9_-]{1,80}$/.test(value.case)?value.case:null,errno:errno(value.error?.code),native:{code:Number.isInteger(native.code)?native.code:null,signal:errno(native.signal),spawn_errno:errno(native.spawn_errno),stdout_bytes:Number.isSafeInteger(native.stdout_bytes)?native.stdout_bytes:0,stderr_bytes:Number.isSafeInteger(native.stderr_bytes)?native.stderr_bytes:0,threads:Number.isSafeInteger(native.threads)?native.threads:null}};
}
export function failureRecorder(evidence){let count=0;return value=>{if(count++>=128)return;appendFileSync(join(evidence,'fixture-failure-stages.jsonl'),JSON.stringify(failureObservation(value))+'\n');};}
export function nativeProcessThreads(pid){try {const text=readFileSync(`/proc/${pid}/status`,'utf8');return Number(/^Threads:\s+(\d+)$/m.exec(text)?.[1]??0);}catch{return null;}}
export function redactedLog(write,secrets){
  const known=secrets.filter(value=>typeof value==='string'&&value.length>=32),hold=Math.max(0,...known.map(value=>value.length-1)),decoder=new StringDecoder('utf8');let pending='';
  const redact=value=>known.reduce((text,secret)=>text.replaceAll(secret,'[redacted]'),value);
  return {append(bytes){const value=redact(pending+decoder.write(bytes)),cut=Math.max(0,value.length-hold);write(value.slice(0,cut));pending=value.slice(cut);},flush(){write(redact(pending+decoder.end()));pending='';}};
}
