export function preflight(text,{maxBytes=65536,maxDepth=6}={}) {
 if(new TextEncoder().encode(text).length>maxBytes) throw Error('Document size limit reached.');
 let depth=0,quoted=false,escaped=false;
 for(const char of text) {
  if(quoted) {if(escaped) escaped=false;else if(char==='\\') escaped=true;else if(char==='"') quoted=false;}
  else if(char==='"') quoted=true;
  else if(char==='{'||char==='[') {if(++depth>maxDepth) throw Error('Document depth limit reached.');}
  else if(char==='}'||char===']') depth--;
 }
}
