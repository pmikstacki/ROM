// Closed execution metadata. Raw child output and exception text are discarded.
const stages = new Map([
 ['initial','supervisor'],['browser-trust-initialize','private-browser-trust'],['browser-trust-import','private-browser-trust'],
 ['native-host-launch','native-host'],['native-host-executable','native-host'],['native-host-environment','native-host'],['native-host-listener','native-host'],
 ['proxy-launch','native-tls-proxy'],['proxy-listeners','native-tls-proxy'],['browser-launch','original-code-browser'],['browser-handoff','original-code-browser'],
 ['native-acquisition','native-host'],['native-outcome','native-host'],['drain','supervisor'],
]);
const roles=new Set(['private-browser-trust','native-host','native-tls-proxy','original-code-browser']);
const categories=new Set(['process-birth','executable','trust-environment','listener','authorization-handoff','native-outcome','drain']);
const fixtureStages=new Set(['configuration','runtime','seed','host-construction','listener','private-control']);
export class NativeTlsStageObserver {
 #clock;#start;#stageStart;#stage='initial';#transitions=0;#attempts=0;#guards=[];#streams=new Map();#bytes=new Map();#overflow=false;
 #markers={ready:0,drained:0,stages:{}};
 constructor(clock=()=>performance.now()){this.#clock=clock;this.#start=this.#time();this.#stageStart=this.#start;}
 #time(){const value=this.#clock();if(!Number.isFinite(value)||value<0)throw Error('finite observer clock required');return value;}
 enter(stage){if(!stages.has(stage)||++this.#transitions>32)throw Error('closed execution stage required');this.#stage=stage;this.#stageStart=this.#time();this.#attempts=0;}
 listenerAttempt(){if(!['native-host-listener','proxy-listeners'].includes(this.#stage)||this.#attempts>=40)throw Error('bounded listener observation required');this.#attempts++;}
 guard(category,passed){if(!categories.has(category)||typeof passed!=='boolean'||this.#guards.length>=32)throw Error('closed execution guard required');this.#guards.push({category,passed,stage:this.#stage,role:stages.get(this.#stage),attempts:this.#attempts});}
 #line(role,channel,line){if(role!=='native-host')return;if(channel==='stderr'){const match=/^host_fixture_stage=([a-z-]+)$/.exec(line);if(match&&fixtureStages.has(match[1]))this.#markers.stages[match[1]]=Math.min(32,(this.#markers.stages[match[1]]??0)+1);return;}try{const value=JSON.parse(line),keys=Object.keys(value).sort().join(',');if(keys==='lane,status,tls_verified'&&value.status==='ready'&&value.lane==='native-tls-source-authoring'&&value.tls_verified===false)this.#markers.ready=Math.min(32,this.#markers.ready+1);else if(keys==='status'&&value.status==='drained')this.#markers.drained=Math.min(32,this.#markers.drained+1);}catch{}}
 consume(role,channel,bytes){if(!roles.has(role)||!['stdout','stderr'].includes(channel)||!Buffer.isBuffer(bytes))throw Error('closed child output role required');const count=(this.#bytes.get(role)??0)+bytes.length;if(count>1048576){this.#overflow=true;this.#bytes.set(role,1048576);return;}this.#bytes.set(role,count);const key=role+':'+channel,state=this.#streams.get(key)??{line:'',drop:false};const parts=bytes.toString('utf8').split('\n');for(let i=0;i<parts.length;i++){if(!state.drop){if(state.line.length+parts[i].length>4096){state.line='';state.drop=true;this.#overflow=true;}else state.line+=parts[i];}if(i<parts.length-1){if(!state.drop)this.#line(role,channel,state.line);state.line='';state.drop=false;}}this.#streams.set(key,state);}
 snapshot(error){const now=this.#time(),bounded=value=>Math.min(300000,Math.max(0,Math.floor(value)));const result={stage:this.#stage,role:stages.get(this.#stage),elapsed_ms:bounded(now-this.#start),stage_elapsed_ms:bounded(now-this.#stageStart),listener_attempts:this.#attempts,guards:this.#guards.map(value=>({...value})),host_markers:{ready:this.#markers.ready,drained:this.#markers.drained,stages:{...this.#markers.stages}},output_overflow:this.#overflow,error_name:error?(['Error','TypeError','RangeError','TimeoutError'].includes(error.name)?error.name:'Other'):null};if(Buffer.byteLength(JSON.stringify(result))>8192)throw Error('execution observation byte bound');return result;}
}
