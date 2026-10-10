// Synthetic browser-session composition over the existing real HTTP/receipt fixture.
import { createServer } from 'node:http';
import { readFileSync, appendFileSync } from 'node:fs';
import { resolve, join, extname, sep } from 'node:path';
import { startProxy } from '../mutation-recovery/proxy.mjs';
export async function startAppProxy(options) {
  const backend = await startProxy({ ...options, port: 0, dist: undefined });
  let subject = 'alice', outage = false, authenticated = true, expiresAt = Math.floor(Date.now()/1000)+3600;
  const credentials = new Map(); let sessionRequests = 0, heldCheck = null;
  async function backendControl(path, body={}) {
    const response=await fetch(backend.base+'/__test/'+path,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body),signal:AbortSignal.timeout(15000)});
    if(!response.ok)throw Error('backend control failed');return response.json();
  }
  const json=(response,status,value)=>{response.writeHead(status,{'content-type':'application/json'});response.end(JSON.stringify(value));};
  const server=createServer(async(request,response)=>{
    try {
      const path=new URL(request.url,'http://localhost').pathname;
      let body='',bytes=0; for await(const chunk of request){bytes+=chunk.length;if(bytes>1048576)throw Error('body limit');body+=chunk;}
      if(path==='/__app/session') {
        const config=JSON.parse(body||'{}');
        if(config.subject!==undefined){if(!['alice','bob'].includes(config.subject))throw Error('invalid synthetic principal');subject=config.subject;await backendControl('session');}
        if(config.renew)await backendControl('session');
        if(config.outage!==undefined)outage=!!config.outage;
        if(config.authenticated!==undefined)authenticated=!!config.authenticated;
        if(config.expiresIn!==undefined)expiresAt=Math.floor(Date.now()/1000)+config.expiresIn;
        if(config.holdCheck){if(heldCheck)throw Error('check already held');let release;const promise=new Promise(resolve=>{release=resolve;});heldCheck={promise,release};}
        if(config.releaseCheck){heldCheck?.release();heldCheck=null;}
        json(response,200,{subject,outage,authenticated,expiresAt,sessionRequests});return;
      }
      if(path==='/rom-studio/auth/session') {
        sessionRequests++;if(outage){json(response,503,{error:'overloaded'});return;}
        const {generation}=await backendControl('session-state');
        if(!authenticated){json(response,200,{authenticated:false,generation:String(generation)});return;}
        const csrf=`fixture-csrf-${generation}`;credentials.set(csrf,{subject,generation});
        const captured={authenticated:true,generation:String(generation),csrf_token:csrf,user_id:subject,expires_at:expiresAt};
        if(heldCheck)await heldCheck.promise;
        if(response.destroyed)return;
        json(response,200,captured);return;
      }
      if(path==='/rom-studio/auth/providers'){json(response,200,{providers:[],primary:null});return;}
      if(path==='/rom-studio/auth/logout'){authenticated=false;json(response,200,{});return;}
      if(path.startsWith('/__test/')||path.startsWith('/rom-studio/api/')) {
        const api=path.startsWith('/rom-studio/api/'),headers={'content-type':'application/json'};
        if(api){const token=String(request.headers['x-rom-csrf']??'');const captured=credentials.get(token);
          if(captured){headers['x-rom-csrf']=token;headers.authorization=`Bearer fixture-${captured.subject==='alice'?'owner':'other'}-${captured.generation}`;}
          // Explicit setup traffic is confined to the disposable fixture authority.
          if(request.headers.authorization){headers.authorization=String(request.headers.authorization);headers['x-rom-csrf']=token;}
        }
        const forwarded=api?path.replace('/rom-studio',''):path;
        const upstream=await fetch(backend.base+forwarded,{method:request.method,headers,body:request.method==='POST'?body:undefined,signal:AbortSignal.timeout(20000)});
        response.writeHead(upstream.status,{'content-type':upstream.headers.get('content-type')??'application/json'});response.end(await upstream.text());return;
      }
      if(request.method!=='GET'||!path.startsWith('/rom-studio/'))throw Error('unknown fixture route');
      const relative=decodeURIComponent(path.slice('/rom-studio'.length));
      const file=resolve(options.dist,'.'+(relative==='/'?'/index.html':relative));
      if(!file.startsWith(resolve(options.dist)+sep))throw Error('static path escaped');
      const content=readFileSync(file),types={'.html':'text/html','.js':'text/javascript','.css':'text/css','.woff2':'font/woff2','.svg':'image/svg+xml'};
      response.writeHead(200,{'content-type':types[extname(file)]??'application/octet-stream'});response.end(content);
    } catch(error) {
      appendFileSync(join(options.evidence,'app-proxy-errors.log'),String(error)+'\n');
      if(!response.destroyed){if(response.headersSent || String(request.url).startsWith('/rom-studio/api/invoke'))response.destroy();else json(response,error.code==='ENOENT'?404:503,{error:'closed'});}
    }
  });
  server.requestTimeout=30000;server.headersTimeout=30000;
  await new Promise((yes,no)=>{server.once('error',no);server.listen(options.port,'127.0.0.1',yes);});
  let closing;const timer=setTimeout(()=>void close(),options.budgetMs??180000);
  function close(){return closing??=(async()=>{clearTimeout(timer);server.closeAllConnections();await new Promise(yes=>server.close(yes));await backend.close();})();}
  return {base:`http://127.0.0.1:${server.address().port}`,close};
}
