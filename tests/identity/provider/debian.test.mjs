import test from 'node:test';
import assert from 'node:assert/strict';
import { debianDataMember, admitToolTar, requirePackageDigest } from './debian.mjs';
import { createHash } from 'node:crypto';

function ar(members) {
  const chunks = [Buffer.from('!<arch>\n')];
  for (const [name, body] of members) {
    const bytes = Buffer.from(body);
    chunks.push(Buffer.from(`${(name + '/').padEnd(16)}${'0'.padEnd(12)}${'0'.padEnd(6)}${'0'.padEnd(6)}${'100644'.padEnd(8)}${String(bytes.length).padEnd(10)}\x60\n`), bytes);
    if (bytes.length % 2) chunks.push(Buffer.from('\n'));
  }
  return Buffer.concat(chunks);
}
function tar(name, { type = '0', link = '', body = 'fixture' } = {}) {
  const header = Buffer.alloc(512), bytes = Buffer.from(body);
  header.write(name); header.write('0000644\0', 100); header.write('0000000\0', 108); header.write('0000000\0', 116);
  header.write(bytes.length.toString(8).padStart(11, '0') + '\0', 124);
  header.write('00000000000\0', 136); header.fill(32, 148, 156); header.write(type, 156); header.write(link, 157);
  header.write('ustar\0', 257); header.write('00', 263);
  const checksum = header.reduce((total, byte) => total + byte, 0);
  header.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148);
  return Buffer.concat([header, bytes, Buffer.alloc((512 - bytes.length % 512) % 512), Buffer.alloc(1024)]);
}
test('Debian admission extracts only the bounded compressed data member', () => {
  const bytes = ar([['debian-binary', '2.0\n'], ['control.tar.xz', 'control'], ['data.tar.xz', 'data']]);
  assert.equal(debianDataMember(bytes).toString(), 'data');
  assert.throws(() => debianDataMember(Buffer.concat([bytes, Buffer.from('extra')])));
});
test('actual Debian BSD-style member names without a slash retain the same closed member contract', () => {
  const bytes = ar([['debian-binary', '2.0\n'], ['control.tar.xz', 'control'], ['data.tar.xz', 'data']]);
  let offset = 8;
  for (const name of ['debian-binary', 'control.tar.xz', 'data.tar.xz']) {
    const size = Number(bytes.subarray(offset + 48, offset + 58).toString().trim());
    bytes.write(name.padEnd(16), offset); offset += 60 + size + size % 2;
  }
  assert.equal(debianDataMember(bytes).toString(), 'data');
});
test('Debian admission rejects alternate or duplicate members and wrong package format', () => {
  for (const members of [
    [['debian-binary', '1.0\n'], ['control.tar.xz', 'c'], ['data.tar.xz', 'd']],
    [['debian-binary', '2.0\n'], ['control.tar.xz', 'c'], ['data.tar.xz', 'd'], ['data.tar.xz', 'again']],
    [['debian-binary', '2.0\n'], ['control.tar.xz', 'c'], ['data.tar.xz', 'd'], ['maintainer', 'script']],
  ]) assert.throws(() => debianDataMember(ar(members)));
});
test('tool tar admission returns regular file bytes after checking the complete archive', () => {
  const members = admitToolTar(tar('./usr/bin/certutil'));
  assert.equal(members.get('usr/bin/certutil').bytes.toString(), 'fixture');
});
test('observed GNU ustar signature keeps the same checksum and path admission', () => {
  const bytes = tar('./usr/bin/certutil');
  bytes.write('ustar  \0', 257); bytes.fill(32, 148, 156);
  const checksum = bytes.subarray(0, 512).reduce((total, byte) => total + byte, 0);
  bytes.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148);
  assert.equal(admitToolTar(bytes).get('usr/bin/certutil').bytes.toString(), 'fixture');
});
test('archive admission rejects absolute/traversal paths, escaping links and unsupported entry types', () => {
  for (const [name, options] of [
    ['/root/file', {}], ['../file', {}], ['./usr/../../file', {}],
    ['./usr/link', { type: '2', link: '../../../outside' }],
    ['./usr/link', { type: '2', link: '/etc/passwd' }],
    ['./usr/device', { type: '3' }], ['./usr/pax', { type: 'x' }],
  ]) assert.throws(() => admitToolTar(tar(name, options)));
});
test('bounded internal relative links remain metadata and cannot supply selected file bytes', () => {
  const member = admitToolTar(tar('./usr/lib/link', { type: '2', link: 'target', body: '' })).get('usr/lib/link');
  assert.equal(member.type, '2'); assert.equal(member.link, 'usr/lib/target'); assert.equal(member.bytes, null);
});
test('tar checksum, truncation, trailing content and entry-size bounds fail closed', () => {
  const bytes = tar('./usr/bin/certutil');
  const bad = Buffer.from(bytes); bad[0] ^= 1;
  for (const value of [bad, bytes.subarray(0, 600), Buffer.concat([bytes, Buffer.from('hidden')])]) assert.throws(() => admitToolTar(value));
  assert.throws(() => admitToolTar(bytes, { maxFileBytes: 3 }));
  assert.throws(() => admitToolTar(bytes, { maxBytes: 512 }));
});
test('pinned package admission rejects byte drift and length drift before archive parsing', () => {
  const bytes = Buffer.from('fixture'), sha256 = createHash('sha256').update(bytes).digest('hex');
  assert.doesNotThrow(() => requirePackageDigest(bytes, { sha256, bytes: bytes.length }));
  assert.throws(() => requirePackageDigest(Buffer.from('changed'), { sha256, bytes: bytes.length }));
  assert.throws(() => requirePackageDigest(bytes, { sha256, bytes: bytes.length + 1 }));
  assert.throws(() => requirePackageDigest(bytes, { sha256: 'unknown', bytes: bytes.length }));
});
