import {resolve} from 'node:path';
import {startProxy} from './proxy.mjs';
for(const key of ['ROM_AI_BROWSER_HOST','ROM_AI_BROWSER_ADAPTER','ROM_AI_BROWSER_EVIDENCE','ROM_AI_BROWSER_PORT','ROM_AI_BROWSER_CONTROL'])if(!process.env[key])throw Error(`missing ${key}`);
const server=await startProxy({binary:process.env.ROM_AI_BROWSER_HOST,adapter:process.env.ROM_AI_BROWSER_ADAPTER,port:Number(process.env.ROM_AI_BROWSER_PORT),dist:resolve('dist'),evidence:process.env.ROM_AI_BROWSER_EVIDENCE,secret:process.env.ROM_AI_BROWSER_CONTROL});
for(const signal of ['SIGTERM','SIGINT'])process.once(signal,()=>{void server.close().then(()=>process.exit(0),()=>process.exit(1));});
console.log(`AI flow browser fixture ${server.base}`);
