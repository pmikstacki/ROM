import { parseWire, stringifyWire } from 'rom-studio';
import type { WireObject, WireValue } from 'rom-studio';
export const principal = { authority: 'external-ai-consumer', subject: 'author', kind: 'embedded' };
export async function post(route: string, body: WireObject): Promise<WireValue> {
  if (!['view', 'resume', 'cancel', 'logout'].includes(route)) throw Error('unsupported owner flow operation');
  const response = await fetch(`/api/${route}`, { method: 'POST', credentials: 'same-origin', redirect: 'error', headers: { 'content-type': 'application/json', accept: 'application/json' }, body: stringifyWire(body), signal: AbortSignal.timeout(22000) });
  if (!response.body) throw Error('missing flow projection');
  const reader = response.body.getReader(), chunks: Uint8Array[] = [];
  let bytes = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read(); if (done) break;
      bytes += value.length; if (bytes > 65536) throw Error('flow projection byte limit');
      chunks.push(value);
    }
  } finally { await reader.cancel().catch(() => {}); }
  const all = new Uint8Array(bytes); let offset = 0;
  for (const chunk of chunks) { all.set(chunk, offset); offset += chunk.length; }
  const text = new TextDecoder('utf-8', { fatal: true }).decode(all);
  const value = parseWire(text, 65536) as WireObject;
  if (!response.ok) { const error = new Error(String(value.error)); Object.assign(error, { category: String(value.error) }); throw error; }
  return value;
}
