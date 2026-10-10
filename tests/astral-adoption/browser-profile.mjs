// Closed, owned acceptance lanes. Selecting TLS does not prove TLS execution.
export function browserProfile(origin) {
  if (origin === 'http://127.0.0.1:43902') {
    return { origin, hostname: '127.0.0.1', secure: false };
  }
  if (origin === 'https://rom-astral.test:43904') {
    return { origin, hostname: 'rom-astral.test', secure: true };
  }
  throw Error('allocated browser origin required');
}

export function requireSessionCookie(cookies, profile) {
  const expected = browserProfile(profile?.origin);
  if (profile.hostname !== expected.hostname || profile.secure !== expected.secure) {
    throw Error('canonical browser profile required');
  }
  if (!Array.isArray(cookies)) throw Error('actual session cookie required');
  const sessions = cookies.filter(cookie => cookie?.name === 'rom_session');
  if (sessions.length !== 1 || sessions[0].domain !== expected.hostname ||
      sessions[0].httpOnly !== true || sessions[0].secure !== expected.secure ||
      sessions[0].sameSite !== 'Lax' ||
      typeof sessions[0].value !== 'string' || !/^[A-Za-z0-9_-]{1,512}$/.test(sessions[0].value)) {
    throw Error('actual session cookie required');
  }
}
