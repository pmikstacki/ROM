import {test,expect} from '@playwright/test';
import {browserCase} from './case-identity.mjs';
test('native startup diagnosis',async({page})=>{
  const info=test.info(),caseId=browserCase({adapter:process.env.ROM_AI_BROWSER_ADAPTER,domain:'publication',mode:'normal',project:info.project.name,testId:info.testId,retry:info.retry,repeat:info.repeatEachIndex});
  const headers={'x-fixture-control':process.env.ROM_AI_BROWSER_CONTROL!};
  const started=await page.request.post('/__fixture/start',{headers,data:{domain:'publication',mode:'normal',case:caseId}});expect(started.status()).toBe(200);
  const inspected=await page.request.post('/__fixture/inspect',{headers,data:{}});expect(inspected.status()).toBe(200);const value=await inspected.json();expect(value.view.state).toBe('Queued');expect(value.provider_calls).toBe(0);expect(value.read_calls).toBe(0);
});
