export const protectedReadPath = '/rom-studio/api/read';
// This function is serialized by Playwright into the actual browser context.
export async function readProtectedResource({ token, path }) {
  const headers = { 'content-type': 'application/json' };
  if (token !== null) headers['x-rom-csrf'] = token;
  const response = await fetch(path, { method: 'POST', headers, body: JSON.stringify({ kind: 'fixture-documents', id: 'private' }) });
  let body;
  try { body = await response.json(); } catch {}
  return { status: response.status, protected_value: body?.value?.content === 'Synthetic protected fixture document' };
}
