// Fixture-owned recovery for FlowClient operations; stored identity never grants server authority.
const statuses = new Set(['Queued', 'Active', 'AwaitingRecovery', 'ActivityUnknown']);
const states = new Set(['Queued', 'Prepared', 'Executing', 'ToolsPending', 'Waiting', 'AwaitingReconciliation', 'Completed', 'Cancelled', 'Failed', 'CancelRequested']);
/** @param {any} view */
export function allowsRecovery(view) {
  return view !== null && !['Completed','Cancelled','Failed'].includes(view.state);
}
/** @param {any} value */
function revision(value) {
  if (typeof value === 'number' && Number.isSafeInteger(value) && value > 0) value = String(value);
  if (!(typeof value === 'string' && /^[1-9]\d{0,19}$/.test(value)) && !(typeof value === 'bigint' && value > 0n)) throw Error('invalid exact flow revision');
  if (BigInt(value) > 18446744073709551615n) throw Error('invalid exact flow revision');
  return String(value);
}
/** @param {any} value */
function validState(value) {
  if (typeof value === 'string') return states.has(value) && value !== 'Waiting';
  if (!value || Object.keys(value).join(',') !== 'Waiting') return false;
  const waiting = value.Waiting;
  if (!waiting || Object.keys(waiting).join(',') !== 'retry_at_unix_ms') return false;
  try { if (waiting.retry_at_unix_ms === 0) return true; revision(waiting.retry_at_unix_ms); return true; } catch { return false; }
}
/** @param {any} value */
export function validateRunView(value) {
  if (!value || !validState(value.state) || typeof value.run_id !== 'string' || value.run_id.length > 64 || !Array.isArray(value.milestones) || value.milestones.length > 512) throw Error('invalid authorized flow view');
  const keys = ['run_id', 'revision', 'state', 'milestones', 'counters', 'failure', 'cancel_requested', 'output', 'read_progress'];
  if (Object.keys(value).some(key => !keys.includes(key))) throw Error('unexpected flow projection field');
  const progress = value.read_progress;
  if (progress !== null && progress !== undefined) {
    if (Object.keys(progress).sort().join(',') !== 'ordinal,status' || !statuses.has(progress.status) || BigInt(progress.ordinal) < 1n || BigInt(progress.ordinal) > 32n) throw Error('invalid advisory read progress');
  }
  return { ...value, revision: revision(value.revision) };
}
/** @param {any} progress */
export function progressText(progress) {
  if (!progress) return 'No pending read';
  switch (progress.status) {
    case 'Queued': return 'Read queued';
    case 'Active': return 'Read active; caller timeout does not end physical work';
    case 'AwaitingRecovery': return 'Read awaiting authorized recovery';
    case 'ActivityUnknown': return 'Read activity unknown';
    default: throw Error('invalid advisory read progress');
  }
}
/** @param {any} options */
export function createFlowRecovery(options) {
  /** @type {any} */
  let record = null;
  let active = false, epoch = 0, quarantined = false;
  /** @type {any} */
  let state = { phase: 'idle', knowledge: 'not_attempted', view: null, error: null, pending: false };
  const principal = JSON.stringify(options.principal), notify = () => options.changed?.({ ...state });
  /** @param {number} ticket */
  const requireCurrent = ticket => { if (quarantined || ticket !== epoch) throw Error('flow recovery binding changed'); };
  /** @param {any} stored */
  const decode = stored => {
    const value = JSON.parse(stored.payload);
    if (value.format !== 'flow-operation-v1' || value.domain !== options.domain || value.run_id !== options.run || JSON.stringify(value.principal) !== principal || !['resume', 'cancel'].includes(value.action) || typeof value.idempotency !== 'string' || !/^[A-Za-z0-9._-]{1,128}$/.test(value.idempotency)) throw Error('flow recovery scope mismatch');
    revision(value.expected_revision); return value;
  };
  return {
    get state() { return { ...state }; },
    async restore() {
      const ticket = epoch, saved = await options.store.read(options.slot); requireCurrent(ticket);
      record = saved ? { ...saved, value: decode(saved) } : null;
      state = { phase: record ? 'unknown' : 'idle', knowledge: record ? 'unknown' : 'not_attempted', view: null, error: null, pending: Boolean(record) }; notify();
    },
    /** @param {string} action @param {any} value @param {string} idempotency */
    async begin(action, value, idempotency) {
      const ticket = epoch; requireCurrent(ticket);
      if (active || record) throw Error('resolve pending operation first');
      const snapshot = validateRunView(value);
      if (snapshot.run_id !== options.run || !['resume', 'cancel'].includes(action) || !/^[A-Za-z0-9._-]{1,128}$/.test(idempotency)) throw Error('invalid flow operation');
      const payload = { format: 'flow-operation-v1', domain: options.domain, principal: options.principal, run_id: options.run, action, expected_revision: snapshot.revision, idempotency };
      const next = { version: options.newVersion(), payload: JSON.stringify(payload) };
      if (!await options.store.compareExchange(options.slot, null, next)) throw Error('concurrent flow operation');
      requireCurrent(ticket); record = { ...next, value: payload };
      state = { phase: 'prepared', knowledge: 'not_attempted', view: null, error: null, pending: true }; notify();
    },
    async retry() {
      const ticket = epoch; requireCurrent(ticket);
      if (!record || active) throw Error('no retryable pending operation');
      active = true; state = { ...state, phase: 'submitting', knowledge: 'unknown', view: null }; notify();
      try {
        const intent = record.value;
        const value = validateRunView(await options.post(intent.action, { run_id: intent.run_id, expected_revision: intent.expected_revision, idempotency: intent.idempotency }));
        requireCurrent(ticket);
        if (value.run_id !== intent.run_id) throw Error('flow operation target mismatch');
        if (!await options.store.compareExchange(options.slot, record.version, null)) throw Error('concurrent recovery cleanup');
        requireCurrent(ticket); record = null;
        state = { phase: 'confirmed', knowledge: 'confirmed', view: value, error: null, pending: false }; notify(); return value;
      } catch (error) {
        requireCurrent(ticket); state = { ...state, phase: 'unknown', knowledge: 'unknown', view: null, error: (/** @type {any} */ (error))?.category ?? 'unknown_outcome' }; notify(); throw error;
      } finally { active = false; }
    },
    quarantine() { epoch++; quarantined = true; state = { ...state, phase: 'quarantined', view: null, error: null }; notify(); },
  };
}
