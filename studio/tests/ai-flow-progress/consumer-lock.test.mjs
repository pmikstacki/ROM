import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const read=relative=>JSON.parse(readFileSync(new URL(relative,import.meta.url),'utf8'));
test('frozen consumer lock admits the current Studio package including its ROM UI dependency',()=>{
  const lock=read('./consumer/package-lock.json'),consumer=read('./consumer/package.json'),producer=read('../../package.json');
  assert.equal(lock.name,consumer.name);assert.equal(lock.packages[''].name,consumer.name);
  assert.deepEqual(lock.packages[''].dependencies,consumer.dependencies);assert.deepEqual(lock.packages[''].devDependencies,consumer.devDependencies);
  assert.equal(lock.packages['node_modules/rom-studio'].version,producer.version);
  assert.deepEqual(lock.packages['node_modules/rom-studio'].dependencies,producer.dependencies);
  assert.equal(lock.packages['node_modules/rom-ui'].resolved,producer.dependencies['rom-ui']);
  assert.match(lock.packages['node_modules/rom-ui'].integrity,/^sha512-[A-Za-z0-9+/]+=*$/);
});
