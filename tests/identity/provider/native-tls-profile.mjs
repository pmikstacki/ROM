// Shared complete-case admission bound for preparation and actual execution.
export const NATIVE_TLS_PROVIDER_REMAINING_MS = 210000;
export function requireNativeTlsProviderWindow(deadline, now = Date.now()) {
  if (!Number.isSafeInteger(deadline) || !Number.isSafeInteger(now) || now < 0 || deadline - now < NATIVE_TLS_PROVIDER_REMAINING_MS) throw Error('finite provider readiness window insufficient');
}

// Linux-only child trust. A fresh native Host constructs a fresh platform verifier.
// This environment does not modify system trust or certify macOS/Windows behavior.
export function nativeTlsEnvironment({ ca, emptyCaDirectory, home, temporary }) {
  for (const path of [ca, emptyCaDirectory, home, temporary]) {
    if (typeof path !== 'string' || !path.startsWith('/') || path.split('/').includes('..')) throw Error('absolute child trust paths required');
  }
  return {
    PATH: '/run/current-system/sw/bin:/usr/bin:/bin', LANG: 'C', TZ: 'UTC',
    HOME: home, TMPDIR: temporary, SSL_CERT_FILE: ca, SSL_CERT_DIR: emptyCaDirectory,
  };
}

export function selectNativeTlsCase({ route, certificate, adapter }) {
  if (!['token', 'jwks'].includes(route) || !['trusted', 'wrong-ca', 'wrong-name', 'expired'].includes(certificate) || !['sqlite', 'redb'].includes(adapter)) throw Error('closed native TLS case required');
  return Object.freeze({ route, certificate, adapter, fresh_host: true, provider_backchannel: false,
    rotate_after: route === 'token' ? 'authorization-code-held' : 'token-forwarded' });
}

// These are native-run evidence requirements, not a substitute for executing Host.
export function assertNativeTlsOutcome(selected, observation) {
  if (observation.target_route !== selected.route || observation.authorization_code_held !== true || observation.child_trust_fenced !== true || !Number.isSafeInteger(observation.native_tcp_connections) || observation.native_tcp_connections < 1) throw Error('actual isolated native acquisition evidence required');
  for (const key of ['token_forwarded', 'tls_errors', 'token_http_requests', 'jwks_http_requests', 'native_tcp_connections', 'native_tls_handshakes']) if (!Number.isSafeInteger(observation[key]) || observation[key] < 0 || observation[key] > 16) throw Error('bounded native acquisition counts required');
  if (observation.native_tls_handshakes > observation.native_tcp_connections) throw Error('TLS handshake count exceeds TCP connections');
  if (selected.certificate === 'trusted') {
    if (observation.native_tls_handshakes < 1) throw Error('trusted native acquisition requires a completed TLS handshake');
    if (observation.native_callback_status !== 303 || observation.authenticated !== true || observation.protected_read_status !== 200 || observation.protected_value !== true || observation.token_forwarded < 1 || observation.token_http_requests < 1 || observation.jwks_http_requests < 1 || observation.tls_errors !== 0) throw Error('trusted native token/JWKS and authorized read required');
  } else {
    if (observation.native_callback_status !== 401 || observation.authenticated !== false || observation.protected_read_status !== 401 || observation.protected_value !== false || observation.tls_errors < 1) throw Error('TLS rejection must not create a session or disclose protected data');
    if (selected.route === 'token' && observation.native_tls_handshakes !== 0) throw Error('negative token TLS must not complete a handshake');
    if (selected.route === 'jwks' && (observation.native_tls_handshakes < 1 || observation.native_tcp_connections < 2)) throw Error('JWKS rejection requires trusted token TLS and a separate TCP connection');
    if (selected.route === 'token' && (observation.token_http_requests !== 0 || observation.jwks_http_requests !== 0 || observation.token_forwarded !== 0)) throw Error('negative token TLS must reject before HTTP forwarding');
    if (selected.route === 'jwks' && (observation.token_forwarded < 1 || observation.token_http_requests < 1 || observation.jwks_http_requests !== 0)) throw Error('negative JWKS TLS requires original token success before JWKS rejection');
  }
}

export function nativeTlsCutoverRequired(selected) { return selected.certificate !== 'trusted' && (selected.certificate !== 'wrong-ca' || selected.route === 'jwks'); }
