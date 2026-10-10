// The persistent owned bridge waits for its parent's actual birth admission.
import {spawnSync} from 'node:child_process';
import {readFileSync,writeFileSync,readlinkSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {namespaceTools} from './trust-namespace.mjs';
import {mountAcknowledged} from './mount-ack.mjs';
import {readProcessIdentity} from './launch.mjs';
const value=JSON.parse(process.argv[2]),prefix='/var/tmp/rom-010-authentik-20261007/run/volume/private/tls-';
if(!value.ack.path.startsWith(prefix)||!value.result.startsWith(prefix)||value.namespace!==readlinkSync('/proc/self/ns/mnt')||value.membership!==readFileSync('/proc/self/cgroup','utf8').trim()||!Array.isArray(value.args)||!((value.args.length===3&&value.args[0]==='--bind'&&value.args[1].startsWith(prefix)&&value.args[1].endsWith('/trust-view')&&value.args[2]==='/etc/ssl/certs')||JSON.stringify(value.args)===JSON.stringify(['--bind','-o','remount,ro','/etc/ssl/certs'])))throw Error('owned exact mount command required');
const deadline=Date.now()+4000;while(!mountAcknowledged(value.ack)){if(Date.now()>=deadline)throw Error('mount admission deadline');await new Promise(resolve=>setTimeout(resolve,10));}
const birth=readProcessIdentity(process.pid),tool=namespaceTools.mount,result=spawnSync(tool,value.args,{timeout:5000,stdio:'ignore',env:{PATH:'/run/current-system/sw/bin:/usr/bin:/bin',LANG:'C',TZ:'UTC'}});
writeFileSync(value.result,JSON.stringify({bridge_birth:birth,namespace:value.namespace,membership:value.membership,tool,tool_sha256:createHash('sha256').update(readFileSync(tool)).digest('hex'),args:value.args,exit_code:result.status,signal:result.signal,tool_pid:result.pid,tool_pid_continuous_birth_admission:false})+'\n',{flag:'wx',mode:0o600});process.exitCode=result.status===0?0:1;
