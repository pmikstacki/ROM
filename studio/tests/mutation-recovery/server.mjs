import { resolve } from 'node:path';
import { startProxy } from './proxy.mjs';
const required = ['ROM_RECOVERY_HOST','ROM_RECOVERY_ADAPTER','ROM_RECOVERY_DATABASE','ROM_RECOVERY_EVIDENCE','ROM_RECOVERY_PORT'];
  for (const key of required) if (!process.env[key]) throw Error(`missing ${key}`);
  const server = await startProxy({ binary:process.env.ROM_RECOVERY_HOST, adapter:process.env.ROM_RECOVERY_ADAPTER, database:process.env.ROM_RECOVERY_DATABASE, port:Number(process.env.ROM_RECOVERY_PORT), dist:resolve('dist'), evidence:process.env.ROM_RECOVERY_EVIDENCE });
  for (const signal of ['SIGINT','SIGTERM']) process.once(signal, () => { void server.close().then(() => process.exit()); });
  console.log(`recovery proxy ${server.base}`);
