// A durable work item may be a Source reaction before the FlowTick notification.
// Inspect after each acknowledged item; stop before another callback can dispatch.
export async function awaitPhysicalRead({step,inspect}) {
  for(let index=0;index<16;index++){
    await step();
    const value=await inspect(),status=value?.view?.read_progress?.status;
    if(value?.view?.state!=='ToolsPending'||value.provider_calls!==1)throw Error('unexpected read progression');
    if(status==='Active'&&value.physical_started===true&&value.read_calls===1)return value;
    if(status!=='Queued'||value.physical_started!==false||value.read_calls!==0)throw Error('unexpected read progression');
  }
  throw Error('physical read did not start within 16 work items');
}
