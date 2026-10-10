// Fixture transport accepts only owner operations; browser bytes cannot select authority.
export function requireOperation(value) {
  if (!value || Object.keys(value).sort().join(',') !== 'expected_revision,idempotency,run_id' || value.run_id !== 'browser-run' || !/^[A-Za-z0-9._-]{1,128}$/.test(value.idempotency ?? '') || typeof value.expected_revision !== 'string' || !/^[1-9]\d{0,19}$/.test(value.expected_revision) || BigInt(value.expected_revision) > 18446744073709551615n) throw Error('invalid owner operation');
  return value;
}
export function requireOrigin(origin, base) {
  if (origin !== base) throw Error('fixture origin denied');
}
export function requireCase(value) {
  if (!['publication', 'triage'].includes(value.domain) || !['normal', 'budget', 'unknown'].includes(value.mode ?? 'normal') || !/^[a-z0-9_-]{1,80}$/.test(value.case ?? '')) throw Error('invalid finite browser case');
  return { domain: value.domain, mode: value.mode ?? 'normal', case: value.case };
}
