import {namespaceTools} from './trust-namespace.mjs';

/** Transport selection only. Source and process admission remain with the supervisor. */
export function identityBrowserCommand(engine,node,entry,configuration,cryptoWrapper){
 if(!['chromium','webkit'].includes(engine)||![node,entry,configuration].every(value=>typeof value==='string'&&value.startsWith('/')))throw Error('closed identity browser command');
 if(engine==='chromium')return{executable:node,args:[entry,configuration]};
 if(typeof cryptoWrapper?.path!=='string'||!cryptoWrapper.path.startsWith('/'))throw Error('patched crypto prerequisite');
 return{executable:namespaceTools.unshare,args:['--mount','--propagation','private','--fork',node,entry,configuration]};
}
