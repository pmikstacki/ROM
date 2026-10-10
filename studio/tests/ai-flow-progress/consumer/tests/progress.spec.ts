import {test,expect,type Page} from '@playwright/test';
import {browserCase} from './case-identity.mjs';
import {awaitPhysicalRead} from './read-progression.mjs';
async function control(page:Page,route:string,body:unknown={}) {
  const response=await page.request.post(`/__fixture/${route}`,{headers:{'x-fixture-control':process.env.ROM_AI_BROWSER_CONTROL!},data:body});
  expect(response.status()).toBe(200);return response.json();
}
async function start(page:Page,domain:string,mode='normal',grant?:'tool'|'row'|'owner') {
  const info=test.info(),caseId=browserCase({adapter:process.env.ROM_AI_BROWSER_ADAPTER,domain,mode,project:info.project.name,testId:grant?`${info.testId}/grant:${grant}`:info.testId,retry:info.retry,repeat:info.repeatEachIndex});
  await control(page,'start',{domain,mode,case:caseId});
  await page.goto(`/?domain=${domain}`);await expect(page.getByTestId('loaded')).toHaveText('ready');
}
async function refresh(page:Page){await page.getByRole('button',{name:'Refresh progress',exact:true}).click();}
async function awaiting(page:Page){await expect.poll(async()=>{await refresh(page);return page.getByTestId('read-progress').textContent();}).toBe('Read awaiting authorized recovery');}
async function readReady(page:Page) {
  await control(page,'control',{until_queued:true});await refresh(page);await expect(page.getByTestId('read-progress')).toHaveText('Read queued');
  const active=await awaitPhysicalRead({step:()=>control(page,'control',{steps:1}),inspect:()=>control(page,'inspect')});
  await refresh(page);await expect(page.getByTestId('read-progress')).toHaveText('Read active; caller timeout does not end physical work');
  expect(active.physical_started).toBe(true);expect(active.read_calls).toBe(1);expect(active.provider_calls).toBe(1);
  await page.getByRole('button',{name:'Inspect durable activity',exact:true}).click();await expect(page.getByTestId('read-progress')).toHaveText('Read activity unknown');
  await control(page,'control',{release:true});await awaiting(page);
}
for(const domain of ['publication','triage']) {
  test(`${domain}: durable progress and unknown acknowledgement reload recover once`,async({page})=>{
    await start(page,domain);await readReady(page);const before=await control(page,'inspect');
    await control(page,'drop');await page.getByRole('button',{name:'Recover read',exact:true}).click();await expect(page.getByTestId('operation-knowledge')).toHaveText('unknown');
    await page.reload();await expect(page.getByTestId('loaded')).toHaveText('ready');await expect(page.getByTestId('operation-knowledge')).toHaveText('unknown');
    await page.getByRole('button',{name:'Retry original operation',exact:true}).click();await expect(page.getByTestId('operation-knowledge')).toHaveText('confirmed');
    const trace=await control(page,'trace');const operations=trace.requests.filter((entry:any)=>entry.route==='resume');expect(operations).toHaveLength(2);expect(operations[1].body).toEqual(operations[0].body);expect(trace.requests.some((entry:any)=>entry.drop)).toBe(true);
    await control(page,'control',{steps:32});await refresh(page);await expect(page.getByTestId('state')).toHaveText('Completed');await expect(page.getByTestId('committed-output')).toHaveText('{"complete":true}');
    const complete=await control(page,'inspect');expect(complete.read_calls).toBe(2);expect(complete.provider_calls).toBe(2);
    if(domain==='publication'){expect(complete.domain.prepared).toBe(true);expect(complete.domain.published).toBe(true);expect(complete.domain.head_revision).toBe(2);}else{expect(complete.domain.classification).toBe('urgent');expect(complete.domain.ticket_revision).toBe(2);}
    await page.reload();await expect(page.getByTestId('loaded')).toHaveText('ready');await control(page,'restart');await refresh(page);await expect(page.getByTestId('state')).toHaveText('Completed');await control(page,'control',{steps:32});const restarted=await control(page,'inspect');expect(restarted.domain).toEqual(complete.domain);expect(restarted.provider_calls).toBe(0);expect(restarted.read_calls).toBe(0);expect(before.provider_calls).toBe(1);
  });
  test(`${domain}: delayed authorized output cannot return after sign out`,async({page})=>{
    await start(page,domain);await readReady(page);await page.getByRole('button',{name:'Recover read',exact:true}).click();await expect(page.getByTestId('operation-knowledge')).toHaveText('confirmed');await control(page,'control',{steps:32});await refresh(page);await expect(page.getByTestId('state')).toHaveText('Completed');
    let arrived!:()=>void,release!:()=>void;
    const observed=new Promise<void>(resolve=>{arrived=resolve;}),held=new Promise<void>(resolve=>{release=resolve;});
    await page.route('**/api/view',async route=>{const response=await route.fetch();expect(response.status()).toBe(200);expect((await response.json()).output).toEqual({complete:true});arrived();await held;await route.fulfill({response});});
    await page.getByRole('button',{name:'Refresh progress',exact:true}).click();await observed;
    await page.getByRole('button',{name:'Sign out',exact:true}).click();await expect(page.getByTestId('message')).toHaveText('Signed out; earlier operation outcome is retained');
    const delivered=page.waitForResponse('**/api/view');release();await (await delivered).finished();
    await page.evaluate(()=>new Promise<void>(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve()))));
    await expect(page.getByRole('region',{name:'Authorized flow progress'})).toHaveCount(0);await expect(page.getByTestId('committed-output')).toHaveCount(0);await expect(page.getByTestId('message')).toHaveText('Signed out; earlier operation outcome is retained');
  });
  test(`${domain}: current owner, tool and row grants control projection and recovery`,async({page})=>{
    for(const grant of ['tool','row','owner'] as const){
      await start(page,domain,'normal',grant);await readReady(page);const before=await control(page,'inspect');
      await control(page,'control',{[grant]:false});await refresh(page);await expect(page.getByRole('region',{name:'Authorized flow progress'})).toHaveCount(0);
      const result=await page.evaluate(async({revision,grant})=>{
        const headers={'content-type':'application/json'},projection=await fetch('/api/view',{method:'POST',headers,body:'{}'});
        const response=await fetch('/api/resume',{method:'POST',headers,body:JSON.stringify({run_id:'browser-run',expected_revision:String(revision),idempotency:`revoked-${grant}`})});
        return {projection:projection.status,status:response.status,body:await response.json()};
      },{revision:before.view.revision,grant});expect(result.projection).toBe(403);
      if(grant==='owner')expect(result.status).toBe(403);
      else{
        // Public read recovery durably refuses denied current tool/row grants.
        expect(result.status).toBe(200);expect(result.body.state).toBe('Failed');expect(result.body.failure).toBe('Denied');expect(result.body.output).toBeNull();expect(result.body.read_progress).toBeNull();expect(result.body.counters).toEqual(before.view.counters);
      }
      await control(page,'control',{[grant]:true});await refresh(page);const restored=await control(page,'inspect');
      expect(restored.read_calls).toBe(before.read_calls);expect(restored.provider_calls).toBe(before.provider_calls);expect(restored.domain).toEqual(before.domain);await expect(page.getByTestId('committed-output')).toHaveCount(0);
      if(grant==='owner'){
        expect(restored.view.revision).toBe(before.view.revision);await expect(page.getByTestId('read-progress')).toHaveText('Read awaiting authorized recovery');
      }else{
        await expect(page.getByTestId('state')).toHaveText('Failed');await expect(page.getByTestId('failure')).toHaveText('Denied');await expect(page.getByRole('button',{name:'Recover read',exact:true})).toBeDisabled();expect(restored.view.read_progress).toBeNull();
        const retryStatus=await page.evaluate(async({revision,grant})=>{
          const response=await fetch('/api/resume',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({run_id:'browser-run',expected_revision:String(revision),idempotency:`restored-${grant}`})});return response.status;
        },{revision:restored.view.revision,grant});expect(retryStatus).toBe(409);
        await page.reload();await expect(page.getByTestId('loaded')).toHaveText('ready');await expect(page.getByTestId('state')).toHaveText('Failed');await expect(page.getByTestId('failure')).toHaveText('Denied');
        await control(page,'control',{steps:32});await refresh(page);await expect(page.getByRole('button',{name:'Recover read',exact:true})).toBeDisabled();const drained=await control(page,'inspect');
        expect(drained.view.state).toBe('Failed');expect(drained.view.failure).toBe('Denied');expect(drained.view.revision).toBe(restored.view.revision);expect(drained.view.output).toBeNull();expect(drained.view.read_progress).toBeNull();expect(drained.view.counters).toEqual(before.view.counters);expect(drained.read_calls).toBe(before.read_calls);expect(drained.provider_calls).toBe(before.provider_calls);expect(drained.domain).toEqual(before.domain);await expect(page.getByTestId('committed-output')).toHaveCount(0);
      }
    }
    await page.getByRole('button',{name:'Sign out',exact:true}).click();await expect(page.getByRole('region',{name:'Authorized flow progress'})).toHaveCount(0);await expect(page.getByRole('button',{name:'Recover read',exact:true})).toBeDisabled();
  });
  test(`${domain}: stale/concurrent recovery preserves exact identities without duplicate effects`,async({page})=>{
    await start(page,domain);await readReady(page);
    const result=await page.evaluate(async()=>{
      const post=async(route:string,body:unknown)=>fetch(`/api/${route}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});
      const view=await(await post('view',{})).json();const expected_revision=String(view.revision);
      const stale=await post('resume',{run_id:'browser-run',expected_revision:String(BigInt(expected_revision)-1n),idempotency:'stale'});
      const responses=await Promise.all(['concurrent-one','concurrent-two'].map(idempotency=>post('resume',{run_id:'browser-run',expected_revision,idempotency})));return {stale:stale.status,statuses:responses.map(x=>x.status)};
    });expect(result.stale).toBe(409);expect(result.statuses.sort()).toEqual([200,409]);
    await control(page,'control',{steps:32});const actual=await control(page,'inspect');expect(actual.read_calls).toBe(2);expect(actual.provider_calls).toBe(2);
  });
  test(`${domain}: exhausted generation budget is visible and output is withheld`,async({page})=>{
    await start(page,domain,'budget');await readReady(page);await page.getByRole('button',{name:'Recover read',exact:true}).click();await expect(page.getByTestId('operation-knowledge')).toHaveText('confirmed');await control(page,'control',{steps:32});await refresh(page);await expect(page.getByTestId('state')).toHaveText('Failed');await expect(page.getByTestId('failure')).toHaveText('BudgetExhausted');await expect(page.getByTestId('committed-output')).toHaveCount(0);expect((await control(page,'inspect')).provider_calls).toBe(1);
  });
  test(`${domain}: cancellation retains unknown provider effects without resubmission`,async({page})=>{
    await start(page,domain,'unknown');await control(page,'control',{steps:12});await refresh(page);await expect(page.getByTestId('state')).toHaveText('AwaitingReconciliation');await page.getByRole('button',{name:'Request cancellation',exact:true}).click();await expect(page.getByTestId('operation-knowledge')).toHaveText('confirmed');await control(page,'control',{steps:12});const current=await control(page,'inspect');expect(current.view.cancel_requested).toBe(true);expect(current.view.output).toBeNull();expect(current.provider_calls).toBe(1);await page.reload();await expect(page.getByTestId('loaded')).toHaveText('ready');await expect(page.getByTestId('committed-output')).toHaveCount(0);
  });
}
