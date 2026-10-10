// Fixture transport: host-held capability and HttpOnly owner binding; no new product REST API.
import {createServer} from 'node:http';
import {randomBytes,timingSafeEqual} from 'node:crypto';
import {mkdirSync,readFileSync,appendFileSync,statSync} from 'node:fs';
import {resolve,join,sep,extname} from 'node:path';
import {launchHost} from './native-process.mjs';
import {failureRecorder} from './failure-observation.mjs';
import {requireOperation,requireOrigin,requireCase} from './protocol.mjs';
const cookieName='rom_ai_fixture';
function equal(a,b){return typeof a==='string'&&a.length===b.length&&timingSafeEqual(Buffer.from(a),Buffer.from(b));}
export async function startProxy({binary,adapter,port,dist,evidence,secret,budgetMs=600000}) {
  if(!['sqlite','redb'].includes(adapter)||!secret||secret.length<32||budgetMs<1000||budgetMs>600000)throw Error('finite fixture configuration required');
  let host, selected, session='',drop=false,loggedOut=false,caseCount=0,closing;
  const trace=[];let base, startStage='request';const observeFailure=failureRecorder(evidence);
  const native=async(route,body)=>{if(!host)throw Error('native fixture not started');return host.request(route,body);};
  const record=value=>{if(trace.length>=512)throw Error('operation trace budget');trace.push(value);appendFileSync(join(evidence,'operation-trace.jsonl'),JSON.stringify(value)+'\n');};
  async function start(value,restart=false){
    startStage='native_stop';if(host)await host.stop();host=null;
    if(!restart){selected=requireCase(value);if(++caseCount>64)throw Error('case budget');startStage='case_directory';mkdirSync(join(evidence,selected.case));session=randomBytes(24).toString('hex');loggedOut=false;drop=false;}
    if(!selected)throw Error('no case selected');
    startStage='native_launch';host=await launchHost({binary,adapter,database:join(evidence,selected.case,`${adapter}.db`),domain:selected.domain,mode:selected.mode,evidence});
  }
  const server=createServer(async(request,response)=>{
    let stage='request',path;
    try {
      path=new URL(request.url,base).pathname;
      if(request.method==='POST'){
        let bytes=0;const chunks=[];for await(const chunk of request){bytes+=chunk.length;if(bytes>16384)throw Error('request byte limit');chunks.push(chunk);}const value=JSON.parse(Buffer.concat(chunks).toString('utf8')||'{}');
        if(path.startsWith('/__fixture/')){
          stage='control_authority';if(!equal(request.headers['x-fixture-control'],secret))throw Error('fixture control denied');
          let result;
          if(path==='/__fixture/start'){stage='native_launch';await start(value);response.setHeader('set-cookie',`${cookieName}=${session}; HttpOnly; SameSite=Strict; Path=/`);result={started:true};}
          else if(path==='/__fixture/restart'){stage='native_launch';await start({},true);result={restarted:true};}
          else if(path==='/__fixture/drop'){drop=true;result={armed:true};}
          else if(path==='/__fixture/trace')result={requests:trace.filter(x=>x.case===selected?.case)};
          else if(path==='/__fixture/control'){const wire=await native('/control',value);response.writeHead(wire.status,{'content-type':'application/json'});response.end(wire.text);return;}
          else if(path==='/__fixture/inspect'){const wire=await native('/inspect',{});response.writeHead(wire.status,{'content-type':'application/json'});response.end(wire.text);return;}
          else throw Error('unknown fixture control');
          response.writeHead(200,{'content-type':'application/json'});response.end(JSON.stringify(result));return;
        }
        stage='owner_authority';requireOrigin(request.headers.origin,base);
        const cookie=String(request.headers.cookie??'').split(';').map(x=>x.trim()).find(x=>x.startsWith(`${cookieName}=`))?.slice(cookieName.length+1);
        if(loggedOut||!session||!equal(cookie,session))throw Error('owner session denied');
        const route=path.slice('/api/'.length);if(!path.startsWith('/api/')||!['view','resume','cancel','logout'].includes(route))throw Error('unknown owner route');
        if(route==='logout'){loggedOut=true;await native('/control',{owner:false});response.setHeader('set-cookie',`${cookieName}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0`);response.writeHead(200,{'content-type':'application/json'});response.end('{}');return;}
        if(route==='view'){if(Object.keys(value).some(k=>k!=='durable_only')||('durable_only'in value&&typeof value.durable_only!=='boolean'))throw Error('invalid view request');}
        else requireOperation(value);
        stage='native_request';const upstream=await native('/'+route,value);
        if(route!=='view')record({case:selected.case,route,body:value,status:upstream.status,response:JSON.parse(upstream.text)});
        if(drop&&route!=='view'&&upstream.status===200){drop=false;const observed=await native('/inspect',{});if(observed.status!==200)throw Error('ack drop lacks current authorized durable observation');record({case:selected.case,drop:true,body:value,observed:JSON.parse(observed.text)});response.destroy();return;}
        response.writeHead(upstream.status,{'content-type':'application/json'});response.end(upstream.text);return;
      }
      if(request.method!=='GET')throw Error('unsupported method');
      stage='static';const file=resolve(dist,'.'+(path==='/'?'/index.html':decodeURIComponent(path)));if(!file.startsWith(resolve(dist)+sep))throw Error('static path escaped');if(statSync(file).size>4194304)throw Error('static file budget');
      response.writeHead(200,{'content-type':({'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.woff2':'font/woff2'})[extname(file)]??'application/octet-stream','cache-control':'no-store'});response.end(readFileSync(file));
    }catch(error){observeFailure({stage:error.fixtureStage??(stage==='native_launch'?startStage:stage),route:path,case:selected?.case,error,native:error.nativeObservation});if(!response.destroyed){response.writeHead(403,{'content-type':'application/json'});response.end(JSON.stringify({error:'fixture request refused'}));}}
  });
  server.requestTimeout=25000;server.headersTimeout=10000;await new Promise((yes,no)=>{server.once('error',no);server.listen(port,'127.0.0.1',yes);});base=`http://127.0.0.1:${server.address().port}`;
  const timer=setTimeout(()=>{void close();},budgetMs);
  function close(){return closing??=(async()=>{clearTimeout(timer);server.closeAllConnections();await new Promise(yes=>server.close(yes));await host?.stop();})();}
  return {base,close};
}
