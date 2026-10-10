// Admit fixed Debian tool archives before writing selected regular-file projections.
// No package scripts, permissions, owners, links, or archive paths are materialized.
import { posix } from 'node:path';
import { createHash } from 'node:crypto';

const MIB = 1024 ** 2;
const fail = () => { throw Error('invalid bounded Debian tool archive'); };
export function requirePackageDigest(content, expected) {
  if (!Buffer.isBuffer(content) || !Number.isSafeInteger(expected?.bytes) || expected.bytes < 1 || expected.bytes > 2 * MIB || content.length !== expected.bytes || !/^[a-f0-9]{64}$/.test(expected.sha256) || createHash('sha256').update(content).digest('hex') !== expected.sha256) fail();
}
export function debianDataMember(bytes) {
  if (!Buffer.isBuffer(bytes) || bytes.length > 2 * MIB || bytes.subarray(0, 8).toString() !== '!<arch>\n') fail();
  const expected = ['debian-binary', 'control.tar.xz', 'data.tar.xz'];
  let offset = 8, data;
  for (const name of expected) {
    if (offset + 60 > bytes.length) fail();
    const header = bytes.subarray(offset, offset + 60).toString('latin1');
    if (![name, `${name}/`].includes(header.slice(0, 16).trim()) || header.slice(58) !== '`\n' || !/^\d+\s*$/.test(header.slice(48, 58))) fail();
    const size = Number(header.slice(48, 58).trim());
    if (!Number.isSafeInteger(size) || size < 1 || size > 2 * MIB || offset + 60 + size > bytes.length) fail();
    const body = bytes.subarray(offset + 60, offset + 60 + size);
    if (name === 'debian-binary' && body.toString() !== '2.0\n') fail();
    if (name === 'data.tar.xz') data = body;
    offset += 60 + size;
    if (size % 2) { if (bytes[offset] !== 10) fail(); offset++; }
  }
  if (offset !== bytes.length) fail();
  return data;
}
function field(header, start, length) {
  const raw = header.subarray(start, start + length);
  const end = raw.indexOf(0);
  return raw.subarray(0, end < 0 ? raw.length : end).toString('latin1');
}
function octal(raw) {
  if (!/^[0-7]+[\0 ]*$/.test(raw)) fail();
  const value = Number.parseInt(raw, 8);
  if (!Number.isSafeInteger(value)) fail();
  return value;
}
function path(raw) {
  if (!/^[A-Za-z0-9._+/-]+$/.test(raw) || raw.startsWith('/')) fail();
  const value = raw.replace(/^\.\//, '').replace(/\/$/, '');
  if (value && value.split('/').some(part => !part || part === '..' || part === '.')) fail();
  return value;
}
export function admitToolTar(bytes, { maxBytes = 16 * MIB, maxFileBytes = 8 * MIB } = {}) {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > 16 * MIB || !Number.isSafeInteger(maxFileBytes) || maxFileBytes < 1 || maxFileBytes > 8 * MIB || !Buffer.isBuffer(bytes) || bytes.length > maxBytes || bytes.length < 1024 || bytes.length % 512) fail();
  const members = new Map();
  let offset = 0, ended = false;
  while (offset + 512 <= bytes.length) {
    const header = bytes.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) {
      if (offset + 1024 > bytes.length || !bytes.subarray(offset).every(byte => byte === 0)) fail();
      ended = true; break;
    }
    const expected = octal(header.subarray(148, 156).toString('latin1'));
    const sum = header.reduce((total, byte, index) => total + (index >= 148 && index < 156 ? 32 : byte), 0);
    const signature = header.subarray(257, 265).toString('latin1');
    const posixSignature = 'ustar\0' + '00';
    if (sum !== expected || ![posixSignature, 'ustar  \0'].includes(signature)) fail();
    const prefix = signature === posixSignature ? field(header, 345, 155) : '';
    const name = path(`${prefix ? `${prefix}/` : ''}${field(header, 0, 100)}`);
    const type = header[156] === 0 ? '0' : String.fromCharCode(header[156]);
    const size = octal(header.subarray(124, 136).toString('latin1'));
    if (!['0', '2', '5'].includes(type) || (type !== '0' && size !== 0) || size > maxFileBytes || members.has(name) || members.size >= 1024 || offset + 512 + size > bytes.length || (!name && type !== '5')) fail();
    let link = null;
    if (type === '2') {
      const target = field(header, 157, 100);
      if (!/^[A-Za-z0-9._+/-]+$/.test(target) || target.startsWith('/')) fail();
      link = posix.normalize(posix.join(posix.dirname(name), target));
      if (link === '..' || link.startsWith('../') || link === '.') fail();
    }
    members.set(name, { type, link, bytes: type === '0' ? bytes.subarray(offset + 512, offset + 512 + size) : null });
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  if (!ended || !members.size) fail();
  return members;
}
