export function admitTlsObservation(expected, value) {
  if (expected === 'trusted') {
    if (value?.navigated !== true || value.title !== 'ROM isolated TLS trust fixture' || value.requests < 1 || value.navigation_error !== null) throw Error('trusted TLS navigation prerequisite failed');
    return;
  }
  const code = { 'untrusted-ca': 'AUTHORITY_INVALID', 'wrong-san': 'COMMON_NAME_INVALID', expired: 'DATE_INVALID' }[expected];
  if (!code || value?.navigated !== false || value.requests !== 0 || typeof value.navigation_error !== 'string') throw Error('negative TLS prerequisite failed');
  const error = value.navigation_error;
  // Chromium supplies a cause-specific certificate code. WebKit supplies a general TLS error;
  // the separately recorded synthetic certificate establishes the controlled negative input.
  if (error.includes('ERR_CERT_') ? !error.includes(`ERR_CERT_${code}`) : !/certificate/i.test(error) || /timeout|refused|unreachable/i.test(error)) throw Error('failure was not the expected certificate rejection');
}
