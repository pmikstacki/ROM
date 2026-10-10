// Nix's base32 uses reverse five-bit groups of little-endian digest bytes.
export function nixSha256(sri) {
  if (typeof sri !== 'string' || !/^sha256-[A-Za-z0-9+/]{43}=$/.test(sri)) throw Error('canonical SHA256 SRI required');
  const text = sri.slice(7), bytes = Buffer.from(text, 'base64');
  if (bytes.length !== 32 || bytes.toString('base64') !== text) throw Error('canonical SHA256 digest required');
  const alphabet = '0123456789abcdfghijklmnpqrsvwxyz'; let encoded = '';
  for (let group = 51; group >= 0; group--) {
    const bit = group * 5, index = Math.floor(bit / 8), shift = bit % 8;
    const value = (bytes[index] >> shift) | (index + 1 < bytes.length ? bytes[index + 1] << (8 - shift) : 0);
    encoded += alphabet[value & 31];
  }
  return `sha256:${encoded}`;
}
