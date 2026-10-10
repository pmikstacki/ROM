// Private original callback handoff; no values from this module may be logged.
const origin = 'https://127.0.0.1:44389';
const callbackPath = '/rom-studio/auth/callback/authentik';
function bindingCookie(header) {
 if(typeof header!=='string'||header.length>8192)throw Error('bounded original binding cookie required');
 const values=header.split(';').map(value=>value.trim()).filter(value=>value.startsWith('rom_login='));
 if(values.length!==1||!/^rom_login=[A-Za-z0-9_-]{1,4096}$/.test(values[0]))throw Error('single original binding cookie required');
 return values[0];
}
export class NativeTlsCallbackHold {
 #held; #forwarded=false;
 get captured() { return Boolean(this.#held); }
 accept(method,target,header,armed) {
  if(typeof target!=='string'||target.length>8192||!target.startsWith('/')||target.startsWith('//')||/[\\\0#]/.test(target)||typeof armed!=='boolean')throw Error('bounded original callback target required');
  const url=new URL(origin+target);
  let decoded; try { decoded=decodeURIComponent(url.pathname); } catch { throw Error('canonical callback route required'); }
  if(decoded.startsWith('/rom-studio/auth/callback/') && url.pathname!==callbackPath)throw Error('exact callback route required');
  if(url.pathname!==callbackPath)return {kind:'forward'};
  if(method!=='GET'||url.searchParams.getAll('code').length!==1||!url.searchParams.get('code')||url.searchParams.getAll('state').length!==1||!url.searchParams.get('state')||url.toString().length>8192)throw Error('original callback parameters required');
  const candidate={callback:origin+target,cookie:bindingCookie(header)};
  if(!armed){if(this.#held)throw Error('original callback already held');this.#held=candidate;return {kind:'hold',handoff:{...candidate}};}
  if(!this.#held||this.#forwarded||candidate.callback!==this.#held.callback||candidate.cookie!==this.#held.cookie)throw Error('armed original callback identity required');
  this.#forwarded=true;return {kind:'forward'};
 }
}
export function readNativeTlsBindingCookies(context) { return context.cookies(origin + callbackPath); }
