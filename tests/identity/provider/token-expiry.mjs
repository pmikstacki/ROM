// The provider supplies this timing. This helper does not alter tokens or clocks.
export function expiryDeadline(timing,now){
 if(!timing||Object.keys(timing).some(key=>!['kid','subject_sha256','iat','exp','received_at_ms','token_sha256'].includes(key))||!Number.isSafeInteger(now)||!Number.isSafeInteger(timing.iat)||!Number.isSafeInteger(timing.exp)||!Number.isSafeInteger(timing.received_at_ms)||timing.iat<0||timing.exp<=timing.iat||timing.exp-timing.iat>60||timing.received_at_ms<timing.iat*1000||timing.received_at_ms>=timing.exp*1000)throw Error('bounded original provider expiry required');
 const deadline=timing.exp*1000+2000;if(deadline<=now||deadline-now>62000)throw Error('original expiry outside finite wait');return deadline;
}
