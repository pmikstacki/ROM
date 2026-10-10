export function forwardedHeaders(input, lane) {
  const host = { host: '127.0.0.1:44389', provider: '127.0.0.1:44392' }[lane];
  if (!host) throw Error('unapproved proxy lane');
  const output = {};
  for (const [name, value] of Object.entries(input)) {
    const key = name.toLowerCase();
    if (key === 'host' || key === 'forwarded' || key.startsWith('x-forwarded-') || ['connection', 'proxy-connection', 'keep-alive', 'transfer-encoding', 'upgrade', 'te', 'trailer'].includes(key)) continue;
    output[key] = value;
  }
  output.host = host; output['x-forwarded-host'] = host; output['x-forwarded-proto'] = 'https';
  return output;
}
export function admitProxyRequest(lane, method, path) {
  if (!['GET', 'POST', 'HEAD', 'OPTIONS'].includes(method) || typeof path !== 'string' || path.length > 8192 || !path.startsWith('/') || path.startsWith('//') || /[\\\0#]/.test(path)) throw Error('proxy request outside profile');
  if (lane === 'private') {
    if (!(method === 'POST' && path === '/application/o/token/') && !(method === 'GET' && path === '/application/o/rom-synthetic-identity/jwks/')) throw Error('unapproved private provider route');
  } else if (lane === 'host') { if (!path.startsWith('/rom-studio/')) throw Error('unapproved Host route'); }
  else if (lane !== 'provider') throw Error('unapproved proxy lane');
}
