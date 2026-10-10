import { test, expect } from '@playwright/test';
import type { APIRequestContext, Page, TestInfo } from '@playwright/test';
const counter = { value:0 };
async function control(request: APIRequestContext, path: string, body: object = {}) { const response = await request.post('/__test/' + path, { data:body }); expect(response.ok()).toBe(true); return response.json(); }
async function inspect(request: APIRequestContext, id: string, key: string) { return control(request,'inspect',{id,idempotency:key,operation:'save'}); }
async function seed(page: Page, testInfo: TestInfo, profile = 'notes') {
  const id = `${testInfo.project.name}-${++counter.value}-${Date.now()}`;
  const session = await control(page.request,'session-state'); await control(page.request,'revoke',{revoked:false});
  const response = await page.request.post('/api/invoke',{headers:{authorization:`Bearer fixture-owner-${session.generation}`,'x-rom-csrf':`fixture-csrf-${session.generation}`,'content-type':'application/json'},data:`{"kind":"recovery-notes","id":"${id}","expected":null,"idempotency":"seed-${id}","operation":{"type":"create","input":{"owner":"alice","title":"seed","count":9007199254740993,"optional":null,"flag":false}}}`});
  expect(response.ok()).toBe(true); await page.goto(`/?id=${id}&profile=${profile}`); await expect(page.getByTestId('loaded')).toHaveText('ready'); return id;
}
async function edit(page: Page, text: string) { await page.locator('#draft').fill(text); if (text !== '!') await expect(page.getByTestId('draft-wire')).toContainText(text); }
async function lose(page: Page, id: string, title = 'A') { await edit(page,title); await control(page.request,'drop',{id}); await page.getByRole('button',{name:'Save',exact:true}).click(); await expect(page.getByTestId('phase')).toHaveText('unknown'); await expect(page.getByTestId('knowledge')).toHaveText('unknown'); const trace = await control(page.request,'trace',{id}); const body = trace.bodies.at(-1); const key = JSON.parse(body).idempotency;await control(page.request,'resume'); return {body,key,committed:await inspect(page.request,id,key)}; }
async function attach(testInfo: TestInfo, label: string, value: object) { await testInfo.attach(label,{body:Buffer.from(JSON.stringify(value,null,2)),contentType:'application/json'}); }

test('lost acknowledgement, restart, IndexedDB reload and renewed session preserve A while retaining C',async ({page},info) => {
  const id = await seed(page,info), lost = await lose(page,id);
  expect(lost.committed.row.revision).toBe(2); expect(lost.committed.events_for_id).toBe(2);
  expect(lost.committed.receipt.identity).toBe(lost.committed.expected_identity);expect(lost.committed.wire).toContain('9007199254740993');
  await edit(page,'B'); await edit(page,'C'); await page.getByRole('button',{name:'Navigate',exact:true}).click();
  await expect(page.getByTestId('view')).toHaveText('editor'); await expect(page.locator('#draft')).toBeFocused();
  await control(page.request,'restart'); await page.reload(); await expect(page.getByTestId('loaded')).toHaveText('ready');
  await expect(page.locator('#draft')).toHaveValue('C'); await expect(page.getByTestId('phase')).toHaveText('unknown');
  await page.getByRole('button',{name:'Renew session'}).click(); await expect(page.getByTestId('message')).toHaveText('Session renewed');
  await page.getByRole('button',{name:'Retry accepted save'}).click(); await expect(page.getByTestId('phase')).toHaveText('succeeded');
  await expect(page.locator('#draft')).toHaveValue('C'); await expect(page.getByTestId('draft-wire')).toContainText('C');await expect(page.getByTestId('result')).toContainText('9007199254740993');
  const after = await inspect(page.request,id,lost.key), trace = await control(page.request,'trace',{id});
  expect(trace.bodies.at(-1)).toBe(lost.body); expect(after.counts).toEqual(lost.committed.counts); expect(after.events_for_id).toBe(2);
  await page.getByRole('button',{name:'Save',exact:true}).click(); await expect(page.getByTestId('phase')).toHaveText('succeeded');
  await expect(page.getByTestId('result')).toContainText('C');
  await page.getByRole('button',{name:'Navigate',exact:true}).click(); await expect(page.getByTestId('view')).toHaveText('notes-list');
  await attach(info,'durable-replay',{id,lost,after,trace});
});

test('current revocation and principal switch hide results and never retry another owner automatically',async ({page},info) => {
  const id = await seed(page,info), lost = await lose(page,id);
  await control(page.request,'revoke',{revoked:true}); await page.getByRole('button',{name:'Retry accepted save'}).click();
  await expect(page.getByTestId('phase')).toHaveText('rejected'); await expect(page.getByTestId('knowledge')).toHaveText('unknown'); await expect(page.getByTestId('result')).toBeEmpty();
  const before = await control(page.request,'trace',{id});
  await page.getByRole('button',{name:'Switch principal'}).click(); await expect(page.getByTestId('phase')).toHaveText('quarantined');
  await expect(page.locator('#draft')).toHaveValue(''); await expect(page.getByTestId('draft-wire')).toBeEmpty();
  await page.getByRole('button',{name:'Restore owner intent'}).click(); await expect(page.getByTestId('phase')).toHaveText('quarantined');
  expect((await control(page.request,'trace',{id})).bodies).toEqual(before.bodies);
  await page.getByRole('button',{name:'Switch principal'}).click(); await expect(page.getByTestId('phase')).toHaveText('quarantined');
  await control(page.request,'revoke',{revoked:false}); await page.getByRole('button',{name:'Restore owner intent'}).click();
  await expect(page.locator('#draft')).toHaveValue('A'); await page.getByRole('button',{name:'Retry accepted save'}).click(); await expect(page.getByTestId('phase')).toHaveText('succeeded');
  expect((await inspect(page.request,id,lost.key)).counts).toEqual(lost.committed.counts);
  await attach(info,'authorized-replay',{id,lost,before,after:await control(page.request,'trace',{id})});
});

test('invalid editor text survives reload and blocked navigation separately from the accepted command',async ({page},info) => {
  const id = await seed(page,info), lost = await lose(page,id);
  await edit(page,'!'); await expect(page.locator('#draft')).toHaveAttribute('aria-invalid','true');
  await page.getByRole('button',{name:'Navigate',exact:true}).click(); await expect(page.getByTestId('view')).toHaveText('editor');
  await page.reload(); await expect(page.getByTestId('loaded')).toHaveText('ready'); await expect(page.locator('#draft')).toHaveValue('!');
  await page.getByRole('button',{name:'Retry accepted save'}).click(); await expect(page.getByTestId('phase')).toHaveText('succeeded');
  await page.getByRole('button',{name:'Navigate',exact:true}).click(); await expect(page.getByTestId('view')).toHaveText('editor'); await expect(page.locator('#draft')).toBeFocused();
  expect((await inspect(page.request,id,lost.key)).counts).toEqual(lost.committed.counts);
  await attach(info,'invalid-editor-draft',{id,lost,trace:await control(page.request,'trace',{id})});
});

test('unrelated inventory editor composes the helper with its own validation and navigation policy',async ({page},info) => {
  const id = await seed(page,info,'inventory'); await expect(page.getByRole('heading')).toHaveText('Inventory note editor');
  const lost = await lose(page,id,'SKU:bolt'); await page.getByRole('button',{name:'Navigate',exact:true}).click();
  await expect(page.getByTestId('message')).toHaveText('Inventory changes retained');
  await page.reload(); await expect(page.getByTestId('loaded')).toHaveText('ready'); await page.getByRole('button',{name:'Retry accepted save'}).click();
  await expect(page.getByTestId('phase')).toHaveText('succeeded'); await page.getByRole('button',{name:'Navigate',exact:true}).click();
  await expect(page.getByTestId('view')).toHaveText('inventory-overview');
  expect((await inspect(page.request,id,lost.key)).counts).toEqual(lost.committed.counts);
  await attach(info,'inventory-replay',{id,lost});
});

async function persisted(page: Page) {
  return page.evaluate(() => new Promise<{version:string;payload:string}[]>((resolve,reject) => {
    const request=indexedDB.open('rom-recovery-host-fixture-v1',1);
    request.onerror=()=>reject(request.error);
    request.onsuccess=()=>{const transaction=request.result.transaction('intents','readonly'),read=transaction.objectStore('intents').getAll();read.onsuccess=()=>resolve(read.result);read.onerror=()=>reject(read.error);};
  }));
}

test('principal change while a committed real HTTP result is withheld rejects late disclosure',async ({page},info) => {
  const id=await seed(page,info);await edit(page,'private-owner-draft');await control(page.request,'hold',{id});
  await page.getByRole('button',{name:'Save',exact:true}).click();await expect(page.getByTestId('phase')).toHaveText('submitting');
  const trace=await control(page.request,'trace',{id}),key=JSON.parse(trace.bodies.at(-1)).idempotency;
  await expect.poll(async () => (await inspect(page.request,id,key)).row.revision).toBe(2);
  const committed=await inspect(page.request,id,key);expect(committed.row.revision).toBe(2);expect(committed.receipt.identity).toBe(committed.expected_identity);
  const payload=await persisted(page);expect(payload).toHaveLength(1);expect(payload[0].payload).toContain(key);
  expect(payload[0].payload).not.toMatch(/fixture-owner|fixture-csrf|host_stamp|authorization|csrf|generation/);
  await page.getByRole('button',{name:'Switch principal'}).click();await expect(page.getByTestId('phase')).toHaveText('quarantined');
  await expect(page.locator('#draft')).toHaveValue('');await expect(page.getByTestId('result')).toBeEmpty();await expect(page.getByTestId('draft-wire')).toBeEmpty();
  await control(page.request,'release');await expect(page.getByTestId('message')).toHaveText('Save outcome unresolved');
  await expect(page.getByTestId('phase')).toHaveText('quarantined');await expect(page.getByTestId('result')).toBeEmpty();
  expect((await control(page.request,'trace',{id})).bodies).toEqual(trace.bodies);
  expect((await inspect(page.request,id,key)).counts).toEqual(committed.counts);
  await attach(info,'late-principal-disclosure',{id,committed,payload,trace});
});
