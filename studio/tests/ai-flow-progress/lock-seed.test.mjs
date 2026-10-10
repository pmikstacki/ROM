import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {seedProducerLock} from './lock-seed.mjs';
import {auditRegistryLock} from '../../../tests/ai-flows-installed/provenance.mjs';
const lock='# frozen producer\nversion = 4\n\n[[package]]\nname = "cc"\nversion = "1.5.1"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "'+'a'.repeat(64)+'"\n';
const digest=value=>createHash('sha256').update(value).digest('hex');
function fixture(){const directory=mkdtempSync(join(tmpdir(),'rom-ai-lock-seed-')),producer=join(directory,'producer'),host=join(directory,'host');mkdirSync(producer);mkdirSync(host);writeFileSync(join(producer,'Cargo.lock'),lock);return {producer,host};}
test('host starts from exact admitted producer lock bytes; producer remains unchanged',()=>{
  const {producer,host}=fixture();const result=seedProducerLock(producer,host,digest(lock));assert.equal(result,digest(lock));assert.equal(readFileSync(join(host,'Cargo.lock'),'utf8'),lock);assert.equal(readFileSync(join(producer,'Cargo.lock'),'utf8'),lock);auditRegistryLock(readFileSync(join(host,'Cargo.lock'),'utf8'),[lock]);
  const drift=lock.replace('version = "1.5.1"','version = "1.6.0"');assert.throws(()=>auditRegistryLock(drift,[lock]),/unexplained registry drift/);
});
test('lock seeding refuses wrong producer identity and occupied host lock',()=>{
  const {producer,host}=fixture();assert.throws(()=>seedProducerLock(producer,host,'b'.repeat(64)),/producer lock/);assert.throws(()=>readFileSync(join(host,'Cargo.lock')),/ENOENT/);seedProducerLock(producer,host,digest(lock));assert.throws(()=>seedProducerLock(producer,host,digest(lock)),/EEXIST/);
});
