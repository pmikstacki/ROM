// Caller admission and result checks do not replace actual provider or browser execution.
export function requireBrowserWindow(ready, witness, now) {
  if (ready?.status !== 'ready' || !/^[a-f0-9]{32}$/.test(ready.unique)) {
    throw Error('ready owned provider required');
  }
  if (witness?.planned_browser_ms !== 240000 || witness?.required_provider_window_ms !== 330000 ||
      !Number.isSafeInteger(now) || !Number.isSafeInteger(ready.ready_deadline_unix_ms)) {
    throw Error('exact declared finite browser budget required');
  }
  const remaining = ready.ready_deadline_unix_ms - now;
  if (remaining < witness.required_provider_window_ms) throw Error('insufficient remaining provider window');
  return remaining;
}

export function requireRecoveryResult(result) {
  const stats = result?.stats;
  if (!stats || stats.expected !== 4 || stats.unexpected !== 0 || stats.skipped !== 0 || stats.flaky !== 0) {
    throw Error('four actual successful recovery cases required');
  }
  return 4;
}
