// Closed synthetic-provider API. Evidence excludes request bodies and credentials.
export function admitProviderApiRequest(role,path,method){
 const key=role==='key'&&((path==='/api/v3/providers/oauth2/1/'&&['GET','PATCH'].includes(method))||(path==='/api/v3/crypto/certificatekeypairs/generate/'&&method==='POST'));
 const account=role==='account'&&method==='POST'&&(path==='/api/v3/core/users/'||/^\/api\/v3\/core\/users\/[1-9][0-9]{0,8}\/set_password\/$/.test(path));
 if(!key&&!account)throw Error('closed synthetic provider endpoint required');
}
export class SyntheticProviderApi{
 #token;#role;#observations=[];
 constructor(environment,role){if(!['key','account'].includes(role))throw Error('closed provider control role');this.#role=role;this.#token=/^AUTHENTIK_BOOTSTRAP_TOKEN=(.+)$/m.exec(environment)?.[1];if(typeof this.#token!=='string'||this.#token.length>256)throw Error('synthetic provider API credential absent');}
 async request(path,method='GET',body){
  admitProviderApiRequest(this.#role,path,method);if(this.#observations.length>=(this.#role==='key'?8:4))throw Error('finite synthetic API request budget');const started=Date.now(),observation={path,method,timeout_ms:8000};this.#observations.push(observation);
  try{const response=await fetch('http://127.0.0.1:44390'+path,{method,headers:{Authorization:`Bearer ${this.#token}`,'Content-Type':'application/json'},...(body?{body:JSON.stringify(body)}:{}),redirect:'error',signal:AbortSignal.timeout(8000)});observation.http_status=response.status;if(!response.ok)throw Error('actual provider control rejected');if(response.status===204)return null;let bytes=0;const chunks=[];for await(const chunk of response.body){bytes+=chunk.length;if(bytes>65536)throw Error('bounded provider control response');chunks.push(Buffer.from(chunk));}return JSON.parse(Buffer.concat(chunks).toString('utf8'));}catch(error){observation.failure=error?.name==='TimeoutError'?'deadline':'acquisition';throw error;}finally{observation.elapsed_ms=Date.now()-started;}
 }
 records(){return this.#observations.map(value=>({...value}));}
}
