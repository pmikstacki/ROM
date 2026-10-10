import {test,expect} from '@playwright/test';
import type {Page,TestInfo,Locator} from '@playwright/test';
async function control(page:Page,path:string,body:object={}) {const response=await page.request.post('/__test/'+path,{data:body});expect(response.ok()).toBe(true);return response.json();}
async function session(page:Page,body:object={}) {const response=await page.request.post('/__app/session',{data:body});expect(response.ok()).toBe(true);return response.json();}
const browserErrors = new WeakMap<Page, string[]>();
test.beforeEach(async({page},info)=>{const errors:string[]=[];browserErrors.set(page,errors);page.on('pageerror',error=>{errors.push(error.stack??String(error));void info.attach('uncaught-browser-error',{body:Buffer.from(error.stack??String(error)),contentType:'text/plain'});});});
test.afterEach(async({page})=>{expect(browserErrors.get(page)??[], 'No uncaught browser errors').toEqual([]);});
const details=(page:Page)=>page.getByRole('region',{name:'Resource details'});
async function fieldMode(form:Locator,name:string,value:string) {
 const page=form.page();
 await form.getByRole('button',{name:`${name} mode`,exact:true}).click();
 await page.locator('[role="listbox"][data-state="open"] [role="option"][data-rom-select-value="'+value+'"]').click();
}
async function loseCreation(page:Page,info:TestInfo) {
 await seed(page,info);
 const id=`created-${info.project.name}-${Date.now()}`;
 await page.getByRole('button',{name:'Create Resource',exact:true}).click();
 const form=page.getByRole('region',{name:'Create Resource',exact:true});
 await form.getByRole('textbox',{name:'Resource ID',exact:true}).fill(id);
 for(const [name,value] of [['owner','alice'],['title','accepted-create'],['count','9007199254740993']]) {
  await fieldMode(form,name,'value');
  await form.getByRole('textbox',{name:`${name} value`,exact:true}).fill(value);
 }
 await fieldMode(form,'flag','value');
 await fieldMode(form,'optional','null');
 await control(page,'drop',{id});
 await expect(form.getByRole('button',{name:'Create resource',exact:true})).toBeEnabled();
 await form.getByRole('button',{name:'Create resource',exact:true}).click();
 await expect(form.getByText('Creation unknown. Commit knowledge: unknown.',{exact:true})).toBeVisible();
 const original=await control(page,'trace',{id}),wire=original.bodies.at(-1),key=JSON.parse(wire).idempotency;
 expect(wire).toContain('9007199254740993');expect(JSON.parse(wire).expected).toBeNull();
 await control(page,'resume');
 const committed=await control(page,'inspect',{id,idempotency:key,operation:'create'});
 expect(committed.receipt.identity).toBe(committed.expected_identity);expect(committed.events_for_id).toBe(1);
 return {form,id,original,wire,key,committed};
}
test('AP004 AP005 creation recovery clears only the confirmed original draft after restart',async({page},info)=>{
 const {form,id,original,wire,key,committed}=await loseCreation(page,info);
 await page.reload();
 await page.getByRole('button',{name:'Create Resource',exact:true}).click();
 expect((await control(page,'trace',{id})).bodies).toHaveLength(original.bodies.length);
 await form.getByRole('button',{name:'Restore creation',exact:true}).click();
 await expect(form.getByRole('textbox',{name:'Resource ID',exact:true})).toHaveValue(id);
 await form.getByRole('button',{name:'Retry creation',exact:true}).click();
 await expect(form).toBeHidden();
 const replay=await control(page,'trace',{id});expect(replay.bodies.at(-1)).toBe(wire);
 const final=await control(page,'inspect',{id,idempotency:key,operation:'create'});
 expect(final.events_for_id).toBe(1);expect(final.receipt.identity).toBe(committed.receipt.identity);
 await page.getByRole('button',{name:'Create Resource',exact:true}).click();
 await form.getByRole('button',{name:'Restore creation',exact:true}).click();
 await expect(form.getByRole('textbox',{name:'Resource ID',exact:true})).toHaveValue('');
 await attach(info,'creation-original-draft-cleanup',{wire,key,committed,final,replay});
});
test('AP004 AP005 creation recovery preserves later invalid raw draft and exact command across reload',async({page},info)=>{
 const {form,id,original,wire,key,committed}=await loseCreation(page,info);
 await form.getByRole('textbox',{name:'Resource ID',exact:true}).fill('later-local-id');
 await form.getByRole('textbox',{name:'count value',exact:true}).fill('invalid-later-count');
 await expect(form.getByText('Local draft saved',{exact:true})).toBeVisible();
 await page.reload();
 await page.getByRole('button',{name:'Create Resource',exact:true}).click();
 expect((await control(page,'trace',{id})).bodies).toHaveLength(original.bodies.length);
 await form.getByRole('button',{name:'Restore creation',exact:true}).click();
 await expect(form.getByRole('textbox',{name:'Resource ID',exact:true})).toHaveValue('later-local-id');
 await expect(form.getByRole('textbox',{name:'count value',exact:true})).toHaveValue('invalid-later-count');
 expect((await control(page,'trace',{id})).bodies).toHaveLength(original.bodies.length);
 await form.getByRole('button',{name:'Retry creation',exact:true}).click();
 await expect(form.getByText('Creation succeeded. Commit knowledge: committed.',{exact:true})).toBeVisible();
 const replay=await control(page,'trace',{id});expect(replay.bodies.at(-1)).toBe(wire);
 const final=await control(page,'inspect',{id,idempotency:key,operation:'create'});expect(final.events_for_id).toBe(1);expect(final.receipt.identity).toBe(committed.receipt.identity);
 await expect(form.getByRole('textbox',{name:'count value',exact:true})).toHaveValue('invalid-later-count');
 await attach(info,'creation-wire-and-database',{wire,key,committed,final,replay});
});
async function open(page:Page,id:string) {await page.getByRole('button',{name:`Open ${id}`,exact:true}).click();await expect(details(page)).toBeVisible();}
async function createNote(page:Page,id:string,owner:'alice'|'bob') {
 const {generation}=await control(page,'session-state'),role=owner==='alice'?'owner':'other';
 const response=await page.request.post('/rom-studio/api/invoke',{headers:{authorization:`Bearer fixture-${role}-${generation}`,'x-rom-csrf':`fixture-csrf-${generation}`,'content-type':'application/json'},data:`{"kind":"recovery-notes","id":"${id}","expected":null,"idempotency":"seed-${id}","operation":{"type":"create","input":{"owner":"${owner}","title":"private-${id}","count":9007199254740993,"optional":null,"flag":false}}}`});
 expect(response.ok()).toBe(true);
}
async function seed(page:Page,info:TestInfo) {
 await session(page,{subject:'alice',authenticated:true,outage:false,expiresIn:3600,releaseCheck:true});await control(page,'revoke',{revoked:false});
 const id=`app-${info.project.name}-${Date.now()}`;await createNote(page,id,'alice');
 await page.goto('./');await expect(page.getByRole('button',{name:'Check session',exact:true})).toBeVisible();await open(page,id);return id;
}
async function check(page:Page) {await page.getByRole('button',{name:/^Check (session|connection)$/}).click();}
async function loseSave(page:Page,id:string) {
 await details(page).getByRole('textbox',{name:'title value',exact:true}).first().fill('accepted-A');
 await control(page,'drop',{id});await details(page).getByRole('button',{name:/^Save 1 change$/}).click();
 await expect(page.getByText('unknown. Commit knowledge: unknown.',{exact:true})).toBeVisible();
 const trace=await control(page,'trace',{id}),wire=trace.bodies.at(-1),key=JSON.parse(wire).idempotency;
 await control(page,'resume');const committed=await control(page,'inspect',{id,idempotency:key,operation:'patch'});
 expect(committed.receipt.identity).toBe(committed.expected_identity);expect(committed.events_for_id).toBe(2);
 return {wire,key,committed};
}
async function attach(info:TestInfo,name:string,value:object) {await info.attach(name,{body:Buffer.from(JSON.stringify(value,null,2)),contentType:'application/json'});}

test('AP006 startup and concurrent explicit session checks share one actual session acquisition',async({page},info)=>{
 await seed(page,info);const before=await session(page,{holdCheck:true});
 await check(page);await page.getByRole('button',{name:'Check session',exact:true}).dispatchEvent('click');
 await expect.poll(async()=>(await session(page)).sessionRequests).toBe(before.sessionRequests+1);
 await session(page,{releaseCheck:true});await expect(page.getByRole('button',{name:'Check session',exact:true})).toBeEnabled();
 expect((await session(page)).sessionRequests).toBe(before.sessionRequests+1);
});
test('AP005 AP006 transient session outage retains invalid draft and selected context with dispatch disabled',async({page},info)=>{
 const id=await seed(page,info),count=details(page).getByRole('textbox',{name:'count value',exact:true}).first();await count.fill('invalid-count');
 await session(page,{outage:true});await check(page);
 await expect(page.getByText('Context is stale. Check your session before saving.',{exact:true})).toBeVisible();
 await expect(count).toHaveValue('invalid-count');await expect(page.getByRole('button',{name:`Open ${id}`,exact:true})).toBeVisible();
 await expect(details(page).getByRole('button',{name:/^Save/}).first()).toBeDisabled();
 await session(page,{outage:false,renew:true});await check(page);await expect(count).toHaveValue('invalid-count');
 await expect(page.getByText('Context is stale. Check your session before saving.',{exact:true})).toHaveCount(0);
});
test('AP004 AP006 accepted patch loses acknowledgement and renewed current session retries exact wire while retaining later invalid draft',async({page},info)=>{
 const id=await seed(page,info),lost=await loseSave(page,id);await details(page).getByRole('textbox',{name:'count value',exact:true}).first().fill('invalid-later');
 await session(page,{renew:true});await check(page);await expect(details(page).getByRole('textbox',{name:'count value',exact:true}).first()).toHaveValue('invalid-later');
 await page.getByRole('button',{name:'Retry same mutation',exact:true}).click();await expect(page.getByText('unknown. Commit knowledge: unknown.',{exact:true})).toHaveCount(0);
 const trace=await control(page,'trace',{id}),after=await control(page,'inspect',{id,idempotency:lost.key,operation:'patch'});
 expect(trace.bodies.at(-1)).toBe(lost.wire);expect(after.events_for_id).toBe(2);expect(after.counts).toEqual(lost.committed.counts);
 await expect(details(page).getByRole('textbox',{name:'count value',exact:true}).first()).toHaveValue('invalid-later');await attach(info,'exact-patch-replay',{id,lost,trace,after});
});
test('AP004 AP005 uncertain delete blocks New row and page navigation without losing current view',async({page},info)=>{
 const id=await seed(page,info);await details(page).getByRole('checkbox',{name:`Confirm deletion of ${id}`}).check();
 await control(page,'drop',{id});await details(page).getByRole('button',{name:'Delete Resource',exact:true}).click();await expect(page.getByText('unknown. Commit knowledge: unknown.',{exact:true})).toBeVisible();
 await expect(page.getByRole('button',{name:'Create Resource',exact:true})).toBeDisabled();await expect(page.getByRole('button',{name:'Settings',exact:true}).first()).toBeDisabled();
 await expect(details(page)).toBeVisible();await expect(page.getByRole('region',{name:'Create Resource'})).toHaveCount(0);
 const trace=await control(page,'trace',{id}),wire=trace.bodies.at(-1),key=JSON.parse(wire).idempotency;await control(page,'resume');
 await page.getByRole('button',{name:'Retry same mutation',exact:true}).click();const after=await control(page,'inspect',{id,idempotency:key,operation:'delete'});
 expect((await control(page,'trace',{id})).bodies.at(-1)).toBe(wire);expect(after.events_for_id).toBe(2);await attach(info,'exact-delete-replay',{id,wire,after});
});
test('AP004 AP005 AP006 changed principal and fresh permission denial hide all prior private editor bytes without automatic retry',async({page},info)=>{
 const id=await seed(page,info),lost=await loseSave(page,id);await details(page).getByRole('textbox',{name:'count value',exact:true}).first().fill('private-invalid-later');
 const before=await control(page,'trace',{id});await control(page,'revoke',{revoked:true});await check(page);
 await expect(details(page)).toHaveCount(0);expect(await page.locator('input, textarea').evaluateAll(nodes=>nodes.some(node=>(node as HTMLInputElement).value.includes('private-invalid-later')))).toBe(false);
 await session(page,{subject:'bob'});await control(page,'revoke',{revoked:false});await check(page);await expect(details(page)).toHaveCount(0);
 expect((await control(page,'inspect',{id,idempotency:lost.key,operation:'patch'})).counts).toEqual(lost.committed.counts);
 const otherId=id+'-bob';await createNote(page,otherId,'bob');const afterSetup=await control(page,'inspect',{id,idempotency:lost.key,operation:'patch'});await check(page);await open(page,otherId);
 await page.getByRole('button',{name:'Restore saved draft and intent',exact:true}).click();
 expect(await page.locator('input,textarea').evaluateAll(nodes=>nodes.some(node=>(node as HTMLInputElement).value.includes('private-invalid-later')))).toBe(false);
 await expect(page.getByRole('button',{name:'Retry same mutation',exact:true})).toHaveCount(0);
 expect((await control(page,'trace',{id})).bodies).toEqual(before.bodies);const unchanged=await control(page,'inspect',{id,idempotency:lost.key,operation:'patch'});
 expect(unchanged.counts).toEqual(afterSetup.counts);expect(unchanged.events_for_id).toBe(lost.committed.events_for_id);expect(unchanged.receipt).toEqual(lost.committed.receipt);await attach(info,'principal-and-denial',{id,lost,before,afterSetup,unchanged});
});
test('AP004 AP005 AP006 reload restores accepted intent and invalid editor only after explicit owner target selection',async({page},info)=>{
 const id=await seed(page,info),lost=await loseSave(page,id);await details(page).getByRole('textbox',{name:'count value',exact:true}).first().fill('invalid-count');
 const action=details(page).locator('form').filter({has:page.getByRole('heading',{name:'save',exact:true})});
 await action.getByRole('button',{name:'count mode',exact:true}).click();
 await page.getByRole('option',{name:'Set value',exact:true}).click();
 await action.getByRole('textbox',{name:'count value',exact:true}).fill('invalid-action-count');
 await expect(page.getByRole('button',{name:'Check session',exact:true})).toBeEnabled();
 await page.reload();await expect(page.getByRole('button',{name:'Check session',exact:true})).toBeVisible();await expect(details(page)).toHaveCount(0);
 const before=await control(page,'trace',{id});await open(page,id);await page.getByRole('button',{name:'Restore saved draft and intent',exact:true}).click();
 await expect(details(page).getByRole('textbox',{name:'count value',exact:true}).first()).toHaveValue('invalid-count');
 await expect(details(page).locator('form').filter({has:page.getByRole('heading',{name:'save',exact:true})}).getByRole('textbox',{name:'count value',exact:true})).toHaveValue('invalid-action-count');expect((await control(page,'trace',{id})).bodies).toEqual(before.bodies);
 await session(page,{renew:true});await check(page);await page.getByRole('button',{name:'Retry same mutation',exact:true}).click();
 expect((await control(page,'trace',{id})).bodies.at(-1)).toBe(lost.wire);expect((await control(page,'inspect',{id,idempotency:lost.key,operation:'patch'})).counts).toEqual(lost.committed.counts);
 await expect(details(page).getByRole('textbox',{name:'count value',exact:true}).first()).toHaveValue('invalid-count');
});
test('AP006 AP021 expiry and logout invalidate held session work before private views can return',async({page},info)=>{
 const id=await seed(page,info);await details(page).getByRole('textbox',{name:'title value',exact:true}).first().fill('private-before-expiry');
 await session(page,{expiresIn:1});await check(page);await expect(details(page)).toHaveCount(0,{timeout:5000});
 await expect(page.getByRole('button',{name:'Sign out',exact:true})).toHaveCount(0);
 await session(page,{expiresIn:3600});await check(page);await open(page,id);await session(page,{holdCheck:true});await check(page);
 await page.getByRole('button',{name:'Sign out',exact:true}).click();await session(page,{releaseCheck:true});await expect(details(page)).toHaveCount(0);
 await expect(page.getByRole('button',{name:'Sign out',exact:true})).toHaveCount(0);
 await expect(page.getByText('private-before-expiry',{exact:true})).toHaveCount(0);
 // Same-owner reacquisition explicitly restores the draft before a new mutation.
 await session(page,{authenticated:true,expiresIn:3600});await check(page);await open(page,id);
 await page.getByRole('button',{name:'Restore saved draft and intent',exact:true}).click();
 await expect(details(page).getByRole('textbox',{name:'title value',exact:true}).first()).toHaveValue('private-before-expiry');
 const lost=await loseSave(page,id),before=await control(page,'trace',{id});
 await page.getByRole('button',{name:'Sign out',exact:true}).click();await expect(details(page)).toHaveCount(0);
 await session(page,{authenticated:true,renew:true,expiresIn:3600});await check(page);await open(page,id);
 expect((await control(page,'trace',{id})).bodies).toEqual(before.bodies);
 await page.getByRole('button',{name:'Restore saved draft and intent',exact:true}).click();
 expect((await control(page,'trace',{id})).bodies).toEqual(before.bodies);
 await page.getByRole('button',{name:'Retry same mutation',exact:true}).click();
 expect((await control(page,'trace',{id})).bodies.at(-1)).toBe(lost.wire);
 expect((await control(page,'inspect',{id,idempotency:lost.key,operation:'patch'})).counts).toEqual(lost.committed.counts);
});

test('Maintained main startup rejects missing ambiguous mistyped invalid and blocked storage before App mount',async({page},info)=>{
 const before=(await session(page)).sessionRequests;
 for(const failure of ['missing','duplicate','wrongtype','invalid','blocked']) {
  if(failure==='blocked') await page.addInitScript(()=>{Object.defineProperty(indexedDB,'open',{value:()=>{throw new DOMException('Storage unavailable','SecurityError');}});});
  await page.goto('startup.html?case='+failure);
  await expect(page.getByRole('alert')).toHaveText('Studio could not start. Check host configuration and browser storage.');
  await expect(page.getByRole('button',{name:/^Check (session|connection)$/})).toHaveCount(0);
  await expect(page.getByRole('region',{name:'Resource details'})).toHaveCount(0);
  expect((await session(page)).sessionRequests).toBe(before);
  await info.attach('startup-'+failure,{body:Buffer.from(await page.locator('#app').innerText()),contentType:'text/plain'});
 }
});
