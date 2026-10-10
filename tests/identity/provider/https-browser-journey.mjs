import { readFileSync, writeFileSync, lstatSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { chromium, webkit } from '/root/ROM/studio/node_modules/@playwright/test/index.mjs';
import { admitBrowserStorage } from './browser-storage.mjs';
import { requireBrowserSocketBudget } from './browser-paths.mjs';
import { awaitLoginDispatch } from './login-dispatch.mjs';
import { classifyBrowserFailure } from './browser-failure.mjs';
import {snapshotLoadedCrypto} from './crypto-wrapper.mjs';
import {ProviderAccountControl} from './provider-account-control.mjs';
import {SigningKeyControl,admitJwksKeys} from './provider-key-control.mjs';
import {expiryDeadline} from './token-expiry.mjs';
import { protectedReadPath, readProtectedResource } from './protected-read.mjs';
const root='/var/tmp/rom-010-authentik-20261007/run/volume',origin='https://127.0.0.1:44389';
export async function runHttpsIdentityBrowser(path) {
 if(!path?.startsWith(`${root}/private/https-host-authoring-`)||lstatSync(path).size>4096)throw Error('owned browser configuration');
 const configuration=JSON.parse(readFileSync(path,'utf8'));
 requireBrowserSocketBudget(process.env.TMPDIR);admitBrowserStorage(root,{home:process.env.HOME,temporary:process.env.TMPDIR});
 const environment=readFileSync(configuration.environment,'utf8'),password=/^AUTHENTIK_BOOTSTRAP_PASSWORD=(.+)$/m.exec(environment)?.[1];if(!password)throw Error('synthetic password');
 let context,stage='browser-launch',sequence=0,session,signing,originalSigning;
 const result={schema:'rom-original-provider-https-lifecycle-authoring-v1',engine:configuration.engine,adapter:configuration.adapter,status:'starting',callbacks:[],navigation:[],sessions:[],controls:[],reads:[],csrf_rejections:[],local_logout:false,tls:true,production_dependency_admission:false,artifact_admission:false,original_consumer_acceptance:false};
 function latestTiming(){const files=readdirSync(configuration.token_timing_directory).filter(name=>/^token-[0-9]+\.json$/.test(name)).sort((a,b)=>Number(a.match(/[0-9]+/)[0])-Number(b.match(/[0-9]+/)[0]));if(!files.length||files.length>16)throw Error('original token timing absent');const path=configuration.token_timing_directory+'/'+files.at(-1),file=lstatSync(path);if(!file.isFile()||file.nlink!==1||file.size>4096||(file.mode&0o777)!==0o600)throw Error('private token timing record');return JSON.parse(readFileSync(path));}
 async function currentSession(){const response=await context.request.get(`${origin}/rom-studio/auth/session`,{timeout:5000});const observation={status:response.status()};result.sessions.push(observation);if(response.status()!==200)throw Error('session response');const value=await response.json();observation.authenticated=value.authenticated===true;observation.expected_user=value.user_id==='fixture-user';observation.has_csrf=typeof value.csrf_token==='string';return value;}
 async function login(page,{expectedStatus=303,username='akadmin',credential=password}={}){
  const callback=page.waitForResponse(response=>{const url=new URL(response.url());return url.origin===origin&&url.pathname==='/rom-studio/auth/callback/authentik';},{timeout:20000});
  // A rejection is still observed below; avoid an unhandled rejection during form entry.
  callback.catch(()=>{});
  stage='login-navigation';
  const navigation=await page.goto(`${origin}/rom-studio/auth/login/authentik`,{timeout:15000});
  const observation={status:navigation?.status()??null,origin:new URL(page.url()).origin===origin?'host':new URL(page.url()).origin==='https://127.0.0.1:44392'?'provider':'unexpected',form_ready:false,credentials_submitted:false};result.navigation.push(observation);
  stage='login-dispatch';
  const identification=page.locator('ak-stage-identification').locator('input[name="uidField"]');
  const dispatch=await awaitLoginDispatch(identification.waitFor({state:'visible',timeout:20000}),callback);
  if(dispatch.kind==='form'){
   observation.form_ready=true;stage='login-identification';await identification.fill(username);await page.getByRole('button',{name:/log in|continue/i}).click();stage='login-password';const passwordStage=page.locator('ak-stage-password');await passwordStage.waitFor({state:'visible',timeout:10000});await passwordStage.locator('input[name="password"]').fill(credential);await passwordStage.locator('input[name="password"]').press('Enter');observation.credentials_submitted=true;
  }
  stage='public-callback';const response=dispatch.kind==='callback'?dispatch.response:await callback;result.callbacks.push(response.status());if(response.status()!==expectedStatus)throw Error('public callback status mismatch');if(expectedStatus===401){if((await currentSession()).authenticated!==false)throw Error('denied callback authenticated session');return;}
  stage='callback-redirect';await page.waitForURL(url=>url.origin===origin&&url.pathname==='/rom-studio/',{timeout:10000});
  stage='current-session';result.session_cookie_present=(await context.cookies(origin)).some(cookie=>cookie.name==='rom_session'&&cookie.secure&&cookie.httpOnly);
  session=await currentSession();
  if(session.authenticated!==true||session.user_id!=='fixture-user'||typeof session.csrf_token!=='string')throw Error('current linked session absent');
 }
 async function read(page,csrf=session?.csrf_token){
  const value=await page.evaluate(readProtectedResource,{token:csrf??null,path:protectedReadPath});
  result.reads.push({status:value.status,protected_value:value.protected_value});return value;
 }
 async function control(action,expectedRevision){
  const next=++sequence,request=`${configuration.controls}/request-${String(next).padStart(2,'0')}.json`,response=`${configuration.controls}/response-${String(next).padStart(2,'0')}.json`;
  writeFileSync(request,JSON.stringify({sequence:next,action}),{flag:'wx',mode:0o600});
  const deadline=Date.now()+5000;let acknowledgement;
  while(Date.now()<deadline){try{if(lstatSync(response).isSymbolicLink()||lstatSync(response).size>4096)throw Error('private acknowledgement type');acknowledgement=JSON.parse(readFileSync(response,'utf8'));break;}catch(error){if(error.code!=='ENOENT'&&!(error instanceof SyntaxError))throw error;}await new Promise(resolve=>setTimeout(resolve,25));}
  if(acknowledgement?.sequence!==next||acknowledgement.status!=='committed'||acknowledgement.revision!==(expectedRevision??(next%2?2:3)))throw Error('control commit acknowledgement');result.controls.push({sequence:next,action,revision:acknowledgement.revision});
 }
 try{
  const executable=configuration.engine==='chromium'?'/root/.nix-profile/bin/chromium':configuration.crypto_wrapper?.path;if(typeof executable!=='string')throw Error('patched browser executable required');
  context=await(configuration.engine==='chromium'?chromium:webkit).launchPersistentContext(`${process.env.HOME}/browser-profile`,{headless:true,executablePath:executable});
  result.browser_version=context.browser()?.version();const page=await context.newPage();
  stage='public-discovery';
  const discovery=await context.request.get('https://127.0.0.1:44392/application/o/rom-synthetic-identity/.well-known/openid-configuration',{timeout:5000});const metadata=await discovery.json();if(discovery.status()!==200||metadata.issuer!=='https://127.0.0.1:44392/application/o/rom-synthetic-identity/')throw Error('original public issuer mismatch');
  const jwks=await context.request.get(metadata.jwks_uri,{timeout:5000});if(jwks.status()!==200)throw Error('original public JWKS');result.public_jwks_sha256=createHash('sha256').update(await jwks.body()).digest('hex');
  stage='fresh-public-callback';await login(page);if(configuration.engine==='webkit')result.loaded_crypto=snapshotLoadedCrypto(configuration.crypto_wrapper.libraries.map(value=>value.path));stage='protected-read';let value=await read(page);if(value.status!==200||!value.protected_value)throw Error('current protected read');
  stage='csrf-negative';
  const wrongOrigin=await context.request.post(`${origin}${protectedReadPath}`,{headers:{origin:'https://wrong.invalid','x-rom-csrf':session.csrf_token},data:{kind:'fixture-documents',id:'private'},timeout:5000});if(wrongOrigin.status()!==401)throw Error('wrong origin accepted');result.csrf_rejections.push({case:'wrong-origin',status:wrongOrigin.status()});
  for(const [name,token]of[['missing-csrf',null],['wrong-csrf','synthetic-wrong-token']]){value=await read(page,token);if(value.status!==403||value.protected_value)throw Error('CSRF rejection');result.csrf_rejections.push({case:name,status:value.status});}
  value=await read(page);if(value.status!==200||!value.protected_value)throw Error('CSRF rejection changed session');
  for(const resource of['user','link','provider']){
   stage=`${resource}-revocation`;await control(`disable-${resource}`);value=await read(page);if(value.status!==401||value.protected_value)throw Error('revoked actor read accepted');
   await control(`enable-${resource}`);value=await read(page);if(value.status!==401||value.protected_value||(await currentSession()).authenticated!==false)throw Error('stale session revived');
   stage=`${resource}-fresh-login`;await login(page);value=await read(page);if(value.status!==200||!value.protected_value)throw Error('fresh current actor read denied');
  }
  if(configuration.lifecycle_directory){
   stage='provider-outage-precondition';const timing=latestTiming();const deadline=timing.received_at_ms+32000;if(timing.exp*1000<deadline+30000)throw Error('original token expires before outage oracle');result.outage={original_token_timing:timing,proof_interval_seconds:30,wait_until_ms:deadline};
   async function lifecycle(action,sequence){const directory=configuration.lifecycle_directory;writeFileSync(directory+'/request-'+sequence+'.json',JSON.stringify({action,sequence}),{flag:'wx',mode:0o600});const end=Date.now()+20000;while(Date.now()<end){try{const path=directory+'/response-'+sequence+'.json',file=lstatSync(path);if(!file.isFile()||file.nlink!==1||file.size>8192||(file.mode&0o777)!==0o600)throw Error('private lifecycle acknowledgement');const value=JSON.parse(readFileSync(path));if(value.sequence!==sequence||value.action!==action||value.status!==(action==='pause-server'?'stopped':'freshly-admitted'))throw Error('lifecycle acknowledgement mismatch');return value;}catch(error){if(error.code!=='ENOENT'&&!(error instanceof SyntaxError))throw error;}await new Promise(resolve=>setTimeout(resolve,50));}throw Error('server lifecycle acknowledgement timeout');}
   stage='provider-real-stop';result.outage.stopped=await lifecycle('pause-server',1);
   await new Promise(resolve=>setTimeout(resolve,Math.max(0,deadline-Date.now())));stage='provider-outage-protected-read';const start=Date.now();value=await read(page);result.outage.failure={status:value.status,protected_value:value.protected_value,elapsed_ms:Date.now()-start};if(value.status!==503||value.protected_value||result.outage.failure.elapsed_ms>6000)throw Error('outage disclosure or unbounded failure');
   stage='provider-real-resume';result.outage.resumed=await lifecycle('resume-server',2);let healthy=false;for(let i=0;i<40;i++){try{const response=await context.request.get('https://127.0.0.1:44392/-/health/ready/',{timeout:1000});if(response.status()===200){healthy=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,250));}if(!healthy)throw Error('resumed provider readiness');
   stage='provider-same-session-recovery';value=await read(page);result.outage.same_session_recovery={status:value.status,protected_value:value.protected_value};stage='provider-fresh-login-recovery';await login(page);value=await read(page);result.outage.fresh_login_recovery={status:value.status,protected_value:value.protected_value};if(value.status!==200||!value.protected_value)throw Error('fresh recovery denied');
  }
  if(configuration.accounts){
   stage='linked-disabled-user-callback';const linkedTiming=latestTiming();result.accounts={linked_original_token:linkedTiming};await control('disable-user',4);value=await read(page);if(value.status!==401||value.protected_value)throw Error('disabled current user disclosed data');await login(page,{expectedStatus:401});result.accounts.disabled_linked_user_callback=401;await control('enable-user',5);await login(page);value=await read(page);if(value.status!==200||!value.protected_value)throw Error('restored current user denied');result.accounts.restored_linked_user=200;
   stage='actual-unlinked-account-create';const account=await new ProviderAccountControl(environment).create();result.accounts.unlinked_account=account.account;result.accounts.provider_controls=account.controls;await context.clearCookies();stage='actual-unlinked-account-callback';await login(page,{expectedStatus:401,username:account.credentials.username,credential:account.credentials.password});value=await read(page);const unlinkedTiming=latestTiming();if(typeof linkedTiming.subject_sha256!=='string'||typeof unlinkedTiming.subject_sha256!=='string'||linkedTiming.subject_sha256===unlinkedTiming.subject_sha256)throw Error('unlinked provider did not issue a distinct original subject');result.accounts.unlinked_original_token=unlinkedTiming;result.accounts.unlinked_callback=401;result.accounts.unlinked_read={status:value.status,protected_value:value.protected_value};if(value.status!==401||value.protected_value)throw Error('unlinked original provider identity disclosed data');
   await context.clearCookies();stage='linked-login-after-unlinked';await login(page);value=await read(page);result.accounts.linked_recovery={status:value.status,protected_value:value.protected_value};if(value.status!==200||!value.protected_value)throw Error('linked login after unlinked denied');
  }
  if(configuration.rotation){
   stage='actual-signing-cutover-precondition';const timing=latestTiming(),deadline=timing.received_at_ms+32000;if(typeof timing.kid!=='string'||timing.exp*1000<deadline+30000)throw Error('original signing token precondition');signing=new SigningKeyControl(environment);originalSigning=await signing.original();const oldJwks=await context.request.get(metadata.jwks_uri,{timeout:5000});if(oldJwks.status()!==200)throw Error('original JWKS acquisition');const originalKeys=admitJwksKeys(await oldJwks.json());if(!originalKeys.includes(timing.kid))throw Error('original token key absent');
   stage='actual-signing-key-generation';const generated=await signing.generate();if(generated.id===originalSigning)throw Error('generated key reused original');result.rotation={original_key:originalSigning,generated_key:generated,original_token_timing:timing,original_jwks_keys:originalKeys,wait_until_ms:deadline};await signing.select(generated.id);
   stage='actual-selected-jwks';const selectedJwks=await context.request.get(metadata.jwks_uri,{timeout:5000});if(selectedJwks.status()!==200)throw Error('selected JWKS acquisition');const selectedKeys=admitJwksKeys(await selectedJwks.json());if(JSON.stringify(selectedKeys)===JSON.stringify(originalKeys))throw Error('actual JWKS did not change');result.rotation.selected_jwks_keys=selectedKeys;result.rotation.old_token_still_published=selectedKeys.includes(timing.kid);
   await new Promise(resolve=>setTimeout(resolve,Math.max(0,deadline-Date.now())));stage='actual-old-key-revalidation';value=await read(page);result.rotation.old_session_read={status:value.status,protected_value:value.protected_value};const expected=result.rotation.old_token_still_published?200:401;if(value.status!==expected||value.protected_value!==(expected===200))throw Error('old session disagrees with actual key publication');if(expected===401&&(await currentSession()).authenticated!==false)throw Error('removed signing key retained session');
   stage='actual-new-key-login';await login(page);value=await read(page);const fresh=latestTiming();result.rotation.fresh_login={status:value.status,protected_value:value.protected_value,original_token_timing:fresh};if(value.status!==200||!value.protected_value||fresh.kid===timing.kid||!selectedKeys.includes(fresh.kid))throw Error('new original signing token not accepted');await signing.select(originalSigning);originalSigning=undefined;result.rotation.original_key_restored=true;
  }
  if(configuration.expiry){
   stage='original-token-expiry-precondition';const timing=latestTiming(),deadline=expiryDeadline(timing,Date.now());result.expiry={original_token_timing:timing,wait_until_ms:deadline};
   await new Promise(resolve=>setTimeout(resolve,Math.max(0,deadline-Date.now())));stage='original-token-expired-read';value=await read(page);result.expiry.expired_read={status:value.status,protected_value:value.protected_value};if(value.status!==401||value.protected_value)throw Error('expired original token disclosed data');const expiredSession=await currentSession();result.expiry.session_anonymous=expiredSession.authenticated===false;if(!result.expiry.session_anonymous)throw Error('expired original token kept session');
   stage='expiry-fresh-login';await login(page);value=await read(page);result.expiry.fresh_login={status:value.status,protected_value:value.protected_value};if(value.status!==200||!value.protected_value)throw Error('fresh login after expiry denied');
  }
  stage='local-logout';const status=await page.evaluate(async token=>(await fetch('/rom-studio/auth/logout',{method:'POST',headers:{'x-rom-csrf':token}})).status,session.csrf_token);if(status!==204||(await currentSession()).authenticated!==false)throw Error('local session logout');value=await read(page);if(value.status!==401||value.protected_value)throw Error('logged-out protected read');result.local_logout=true;if(result.outage&&result.outage.same_session_recovery.status!==200){stage='provider-same-session-recovery-regression';throw Error('temporary provider outage destroyed current session');}result.status='passed';
 }catch(error){result.status='failed';result.stage=stage;result.failure=classifyBrowserFailure(error);process.exitCode=1;}
 finally{if(originalSigning){try{await signing.select(originalSigning);result.rotation.original_key_restored=true;}catch{result.status='failed';result.stage='original-signing-key-restore';process.exitCode=1;}}if(signing)result.signing_controls=signing.records();await context?.close();result.storage=admitBrowserStorage(root,{home:process.env.HOME,temporary:process.env.TMPDIR});writeFileSync(configuration.result,JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o600});console.log(JSON.stringify({status:result.status,stage:result.stage??'complete',engine:result.engine,adapter:result.adapter,production_dependency_admission:false}));}
}
