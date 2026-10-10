// Exploratory fixture limits. This is not a supported production throughput envelope.
export function exploratoryProfile(options) {
  if (options !== undefined) throw Error("closed profile");
  return Object.freeze({
    seed_resources: 10000,
    query: Object.freeze({
      page_rows: 50,
      candidate_rows: 12000,
      candidate_bytes: 32 * 1024 ** 2,
    }),
    work: Object.freeze({ records: 12000, bytes: 16 * 1024 ** 2 }),
    maximum_reaction_records: 12000,
    traffic: Object.freeze({
      scheduled_groups: 12000,
      mutations: 1500,
      notification_intents: 500,
      uploads: 640,
    }),
    seeding: Object.freeze({ batch: 100, deadline_ms: 120000 }),
    phases: Object.freeze({
      warm_ms: 30000,
      measure_ms: 90000,
      drain_ms: 30000,
    }),
  });
}
function closedIntegerProof(proof, fields, name) {
  if (
    !proof ||
    Object.keys(proof).length !== fields.length ||
    Object.keys(proof).some((k) => !fields.includes(k))
  )
    throw Error(name);
  for (const field of fields)
    if (!Number.isSafeInteger(proof[field]) || proof[field] < 0)
      throw Error(name);
}
export function requireLedgerProof(profile, proof) {
  closedIntegerProof(proof, ["records", "bytes"], "ledger proof");
  if (proof.records > profile.work.records) throw Error("ledger records");
  if (proof.bytes > profile.work.bytes) throw Error("ledger bytes");
}
export function requireSeedProof(profile, proof) {
  closedIntegerProof(
    proof,
    [
      "resources",
      "candidate_bytes",
      "seed_work",
      "done_work",
      "work_bytes",
      "drain_ms",
    ],
    "seed proof",
  );
  if (
    proof.resources !== profile.seed_resources ||
    proof.seed_work !== profile.seed_resources ||
    proof.done_work !== proof.seed_work ||
    proof.drain_ms > profile.seeding.deadline_ms
  )
    throw Error("seed drain");
  if (proof.candidate_bytes > profile.query.candidate_bytes)
    throw Error("candidate bytes");
  requireLedgerProof(profile, {
    records: proof.seed_work,
    bytes: proof.work_bytes,
  });
}
