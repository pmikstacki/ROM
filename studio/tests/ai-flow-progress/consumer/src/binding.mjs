// One component's owner binding; captured tickets cannot outlive sign-out or disposal.
export function createBinding() {
  let epoch = 0, live = true;
  return { ticket: () => epoch, current: (/** @type {number} */ ticket) => live && ticket === epoch,
    quarantine() { epoch++; live = false; }, dispose() { epoch++; live = false; } };
}
/** @param {any} binding @param {()=>Promise<any>} load @param {(value:any)=>void} publish @param {(error:any)=>void} refuse */
export async function readBoundView(binding, load, publish, refuse) {
  const ticket = binding.ticket(); if (!binding.current(ticket)) return;
  try { const value = await load(); if (binding.current(ticket)) publish(value); }
  catch (error) { if (binding.current(ticket)) refuse(error); }
}
