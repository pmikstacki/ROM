import test from 'node:test';import assert from 'node:assert/strict';import{requireProviderLimits,requireMembership,requireDrainedCgroup}from'./cgroup.mjs';
test('provider cgroup rejects uncapped, broader or partial controls',()=>{const limits={cpu:'400000 100000',memory:'4294967296',pids:'512'};assert.doesNotThrow(()=>requireProviderLimits(limits));for(const field of['cpu','memory','pids'])assert.throws(()=>requireProviderLimits({...limits,[field]:'max'}),/provider cgroup/);assert.throws(()=>requireProviderLimits({...limits,cpu:'800000 100000'}),/provider cgroup/);});
test('resource membership must identify the exact fresh unified cgroup',()=>{assert.doesNotThrow(()=>requireMembership('0::/rom-identity-0123456789abcdef\n','/rom-identity-0123456789abcdef'));for(const value of['0::/\n','0::/rom-identity-0123456789abcdef-other\n','1:cpu:/rom-identity-0123456789abcdef\n'])assert.throws(()=>requireMembership(value,'/rom-identity-0123456789abcdef'),/cgroup membership/);});
test('drain rejects live descendants, duplicate population fields and malformed events',()=>{
 assert.doesNotThrow(()=>requireDrainedCgroup('populated 0\nfrozen 0\n'));
 for(const value of['populated 1\nfrozen 0\n','frozen 0\n','populated 0\npopulated 1\n','populated false\n','populated 0\nunknown 0\n',null])assert.throws(()=>requireDrainedCgroup(value),/cgroup drain/);
});
