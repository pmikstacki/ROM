import { createClient } from 'rom-studio/client';
import type { DurablePrincipal } from 'rom-studio/recovery';
export const owner: DurablePrincipal = { authority: 'recovery-fixture', kind: 'human', subject: 'alice' };
export const other: DurablePrincipal = { authority: 'recovery-fixture', kind: 'human', subject: 'bob' };
export function transport(generation: number, subject = 'alice') {
  return createClient({ base: '/api', csrf: () => `fixture-csrf-${generation}`, fetch: (url, init) => {
    const headers = new Headers(init?.headers); headers.set('authorization', `Bearer fixture-${subject === 'alice' ? 'owner' : 'other'}-${generation}`);
    return fetch(url, { ...init, headers });
  } });
}
export async function control(path: string, value: object = {}): Promise<Record<string, unknown>> {
  const response = await fetch('/__test/' + path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(value) });
  if (!response.ok) throw Error('fixture control failed');
  return response.json();
}
