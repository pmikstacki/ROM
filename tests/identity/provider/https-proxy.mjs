// Isolated TLS termination and exact private backchannel routes. No request or credential logging.
import { readFileSync, writeFileSync, lstatSync, realpathSync, existsSync } from 'node:fs';
import { createServer as httpsServer } from 'node:https';
import { createServer as httpServer, request as upstreamRequest } from 'node:http';
import { X509Certificate, createHash } from 'node:crypto';
import {projectTokenTiming} from './token-timing.mjs';
import { forwardedHeaders, admitProxyRequest } from './proxy-contract.mjs';
const root = '/var/tmp/rom-010-authentik-20261007/run/volume';
export async function serveOwnedIdentityProxies(configurationPath) {
  if (!configurationPath?.startsWith(`${root}/private/https-host-authoring-`) || realpathSync(configurationPath) !== configurationPath || lstatSync(configurationPath).size > 4096) throw Error('owned proxy configuration required');
  const configuration = JSON.parse(readFileSync(configurationPath, 'utf8'));
  if (!['certificate,key,result,stop_file','certificate,key,result,stop_file,token_timing_directory'].includes(Object.keys(configuration).sort().join(','))) throw Error('closed proxy configuration required');
  for (const name of ['certificate','key','result','stop_file']) if (typeof configuration[name] !== 'string' || !configuration[name].startsWith(`${root}/`) || configuration[name].split('/').includes('..')) throw Error('owned proxy paths required');
  const key = readFileSync(configuration.key), cert = readFileSync(configuration.certificate), certificate = new X509Certificate(cert);
  if (Date.parse(certificate.validFrom) > Date.now() || Date.parse(certificate.validTo) <= Date.now() || certificate.checkIP('127.0.0.1') !== '127.0.0.1') throw Error('current owned TLS certificate required');
  let requests=0, inflight=0, bytes=0, rejected=0, tlsErrors=0, tokenCount=0;const timingDirectory=configuration.token_timing_directory;if(timingDirectory&&(typeof timingDirectory!=='string'||!timingDirectory.startsWith(`${root}/private/https-host-authoring-`)||realpathSync(timingDirectory)!==timingDirectory||!lstatSync(timingDirectory).isDirectory()||(lstatSync(timingDirectory).mode&0o777)!==0o700))throw Error('private token timing directory');
  const servers=[], upstreams=new Set();
  const route = lane => (request,response) => {
    try { admitProxyRequest(lane, request.method, request.url); if (++requests > 512 || inflight >= 32) throw Error('proxy request cap'); }
    catch { rejected++;response.writeHead(400);response.end();return; }
    inflight++;
    const target=upstreamRequest({host:'127.0.0.1',port:lane==='host'?44391:44390,method:request.method,path:request.url,headers:forwardedHeaders(request.headers,lane==='host'?'host':'provider'),timeout:10000});
    upstreams.add(target);let requestBytes=0,responseBytes=0,finished=false;
    const finish=()=>{if(finished)return;finished=true;inflight--;upstreams.delete(target);};
    const fail=()=>{if(finished)return;target.destroy();if(!response.headersSent)response.writeHead(502);response.end();finish();};
    target.on('timeout',fail);target.on('error',fail);request.on('aborted',fail);response.on('close',()=>{target.destroy();finish();});
    request.on('data',chunk=>{requestBytes+=chunk.length;if(requestBytes>65536)fail();});
    target.on('response',incoming=>{
      const headers={...incoming.headers};delete headers.connection;delete headers['transfer-encoding'];
      response.writeHead(incoming.statusCode??502,headers);
      const capture=timingDirectory&&lane==='private'&&request.url==='/application/o/token/'&&incoming.statusCode===200,chunks=[];incoming.on('data',chunk=>{if(capture&&responseBytes+chunk.length<=65536)chunks.push(chunk);responseBytes+=chunk.length;bytes+=chunk.length;if(responseBytes>4*1024**2||bytes>32*1024**2)fail();});incoming.on('error',fail);incoming.on('end',()=>{if(capture){try{if(responseBytes>65536||++tokenCount>16)throw Error('token timing bound');writeFileSync(`${timingDirectory}/token-${tokenCount}.json`,JSON.stringify(projectTokenTiming(Buffer.concat(chunks),Date.now())),{flag:'wx',mode:0o600});}catch{rejected++;}}finish();});incoming.pipe(response);
    });request.pipe(target);
  };
  try {
    for (const [lane,port] of [['host',44389],['provider',44392],['private',44393]]) {
      const server=lane==='private'?httpServer(route(lane)):httpsServer({key,cert,minVersion:'TLSv1.2'},route(lane));server.requestTimeout=15000;server.headersTimeout=10000;server.maxHeadersCount=64;server.on('tlsClientError',()=>tlsErrors++);servers.push(server);
      await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(port,'127.0.0.1',resolve);});
    }
    console.log(JSON.stringify({status:'ready',ports:[44389,44392,44393],lane:'source-authoring-only'}));
    const deadline=Date.now()+120000;
    await new Promise(resolve=>{
      const finish=()=>{clearInterval(interval);process.removeListener('SIGTERM',finish);resolve();};
      const interval=setInterval(()=>{if(Date.now()>=deadline||existsSync(configuration.stop_file))finish();},50);process.once('SIGTERM',finish);
    });
  } finally {
    for(const target of upstreams)target.destroy();
    for(const server of servers){server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
    writeFileSync(configuration.result,JSON.stringify({schema:'rom-owned-https-proxy-authoring-v1',requests,response_bytes:bytes,rejected,tls_errors:tlsErrors,certificate_der_sha256:createHash('sha256').update(certificate.raw).digest('hex'),provider_acceptance:false,production_dependency_admission:false},null,2)+'\n',{flag:'wx',mode:0o600});
  }
}
