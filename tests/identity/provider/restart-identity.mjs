import {requireContainerIdentity} from './container-identity.mjs';
import {sameProcessIdentity} from './launch.mjs';

/** Admit a stopped container without changing ordinary live-PID admission. */
export function admitStoppedProvider(expected,actual){
 if(actual?.pid!==0)throw Error('provider must be stopped');
 // Reuse every immutable container/limit check. The sentinel is never process evidence.
 requireContainerIdentity(expected,{...actual,pid:2});
}
export function admitProviderRebirth(before,after){
 if(!before?.birth||!before.conmon||!after?.birth||!after.conmon||sameProcessIdentity(before.birth,after.birth)||sameProcessIdentity(before.conmon,after.conmon)||before.netns!==after.netns)throw Error('fresh provider rebirth required');
}
