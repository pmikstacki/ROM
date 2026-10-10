import {resolve} from 'node:path';
import {startAppProxy} from './proxy.mjs';
for(const key of ['ROM_RECOVERY_HOST','ROM_RECOVERY_ADAPTER','ROM_RECOVERY_DATABASE','ROM_RECOVERY_EVIDENCE','ROM_RECOVERY_PORT'])if(!process.env[key])throw Error(`missing ${key}`);
const server=await startAppProxy({binary:process.env.ROM_RECOVERY_HOST,adapter:process.env.ROM_RECOVERY_ADAPTER,database:process.env.ROM_RECOVERY_DATABASE,evidence:process.env.ROM_RECOVERY_EVIDENCE,port:Number(process.env.ROM_RECOVERY_PORT),dist:resolve('dist'),budgetMs:Number(process.env.ROM_APP_BUDGET_MS??1200000)});
for(const signal of ['SIGINT','SIGTERM'])process.once(signal,()=>void server.close().then(()=>process.exit()));
console.log(server.base+'/rom-studio/');
