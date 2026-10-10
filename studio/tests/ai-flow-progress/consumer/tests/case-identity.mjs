import {createHash} from 'node:crypto';
/** @param {any} identity */
export function browserCase(identity){
  const {adapter,domain,mode,project,testId,retry,repeat}=identity;
  if(!['sqlite','redb'].includes(adapter)||!['publication','triage'].includes(domain)||!['normal','budget','unknown'].includes(mode)||!['chromium','webkit'].includes(project)||typeof testId!=='string'||!testId||testId.length>512||!Number.isInteger(retry)||retry<0||retry>2||!Number.isInteger(repeat)||repeat<0||repeat>4)throw Error('invalid configured browser case identity');
  const digest=createHash('sha256').update(JSON.stringify([adapter,domain,mode,project,testId])).digest('hex').slice(0,24);
  return `case-${adapter}-${project}-${digest}-r${retry}-n${repeat}`;
}
