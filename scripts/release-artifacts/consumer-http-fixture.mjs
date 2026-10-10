// Representative report bytes for static admission tests; no HTTP is executed.
export function recoveryHttpFixture(adapter = 'sqlite') {
  const receipt = { identity: 'original-identity' };
  const live = { counts: [1, 2, 2, 1], events_for_id: 2, row: { revision: 2, value: { title: 'A' } },
    receipt, expected_identity: receipt.identity, wire: '9007199254740993' };
  const deleted = { ...live, counts: [1, 3, 3, 1], events_for_id: 3, row: { revision: 3, value: null } };
  const accepted = '{"id":"http-one","expected":1,"value":9007199254740993}';
  return { adapter, counts: { committed: live, replayed: structuredClone(live), deleted,
    delete_replayed: structuredClone(deleted), retired: { ...deleted } },
    trace: { bodies: ['{"id":"http-one"}', accepted, accepted], dropped: [] },
    exact_first_and_replay: true, process_restart: true,
    pending_store_limit: 'single-process synchronous file CAS with file/directory fsync; no power-loss certification' };
}
