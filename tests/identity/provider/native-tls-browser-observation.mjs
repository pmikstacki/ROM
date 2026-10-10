// Closed metadata only: never retain URLs, exception messages or field values.
const stages = new Set(['launch', 'navigation', 'identification-wait', 'identification-fill', 'identification-submit', 'password-wait', 'password-fill', 'authorization-submit', 'callback-wait', 'browser-drain', 'binding-cookie']);
const origins = new Set(['https://127.0.0.1:44389', 'https://127.0.0.1:44392']);
const paths = new Set(['/rom-studio/auth/login/authentik', '/rom-studio/auth/callback/authentik', '/if/flow/default-authentication-flow/', '/application/o/authorize/']);
function safeLocation(value) {
 if (typeof value !== 'string' || value.length > 8192) return null;
 try { const url=new URL(value);if(!origins.has(url.origin))return null;return {origin:url.origin,pathname:paths.has(url.pathname)?url.pathname:'/other'}; }catch{return null;}
}
export class NativeTlsBrowserObservation {
 #stages=['launch']; #locations=[]; #counts={};
 enter(stage) { if(!stages.has(stage))throw Error('closed authorization stage required');if(this.#stages.length>=16)throw Error('authorization stage bound');this.#stages.push(stage); }
 location(value) { const location=safeLocation(value);if(location&&this.#locations.length<16&&!this.#locations.some(v=>v.origin===location.origin&&v.pathname===location.pathname))this.#locations.push(location); }
 response(url,status) { if(!safeLocation(url)||!Number.isInteger(status)||status<100||status>599)return;const bucket=`${Math.floor(status/100)}xx`;this.#counts[bucket]=Math.min(128,(this.#counts[bucket]??0)+1); }
 failure(error,selectors) {
  const counts={};for(const [name,count]of Object.entries(selectors)){if(!['identification','password','frames'].includes(name)||!Number.isInteger(count)||count<0||count>32)throw Error('closed bounded selector observation required');counts[name]=count;}
  const name=['Error','TimeoutError','TypeError'].includes(error?.name)?error.name:'Other';
  const result={status:'failed',stage:this.#stages.at(-1),stages:[...this.#stages],error_name:name,locations:[...this.#locations],response_status_counts:{...this.#counts},selector_counts:counts};
  if(Buffer.byteLength(JSON.stringify(result))>4096)throw Error('authorization metadata byte bound');return result;
 }
}
