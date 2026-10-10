import test from "node:test";
import assert from "node:assert/strict";
import {
  exploratoryProfile,
  requireSeedProof,
  requireLedgerProof,
} from "./profile.mjs";
test("load configuration separates page size from complete candidates and retained work", () => {
  const p = exploratoryProfile();
  assert.equal(p.seed_resources, 10000);
  assert.equal(p.query.page_rows, 50);
  assert.equal(p.query.candidate_rows, 12000);
  assert.equal(p.query.candidate_bytes, 32 * 1024 ** 2);
  assert.equal(p.work.records, 12000);
  assert.equal(p.work.bytes, 16 * 1024 ** 2);
  assert.equal(p.maximum_reaction_records, 12000);
  assert.throws(
    () => exploratoryProfile({ seed_resources: 20000 }),
    /closed profile/,
  );
});
test("seed admission needs actual candidate bytes and completed reaction proof", () => {
  const p = exploratoryProfile();
  const proof = {
    resources: 10000,
    candidate_bytes: 2000000,
    seed_work: 10000,
    done_work: 10000,
    work_bytes: 7000000,
    drain_ms: 1000,
  };
  assert.doesNotThrow(() => requireSeedProof(p, proof));
  assert.throws(
    () => requireSeedProof(p, { ...proof, done_work: 9999 }),
    /seed drain/,
  );
  assert.throws(
    () =>
      requireSeedProof(p, {
        ...proof,
        candidate_bytes: p.query.candidate_bytes + 1,
      }),
    /candidate bytes/,
  );
  assert.throws(
    () => requireSeedProof(p, { ...proof, work_bytes: p.work.bytes + 1 }),
    /ledger bytes/,
  );
  assert.throws(
    () => requireSeedProof(p, { ...proof, secret: "not admissible" }),
    /seed proof/,
  );
});
test("retained Done work still counts towards finite ledger admission", () => {
  const p = exploratoryProfile();
  assert.doesNotThrow(() =>
    requireLedgerProof(p, { records: 12000, bytes: 10000000 }),
  );
  assert.throws(
    () => requireLedgerProof(p, { records: 12001, bytes: 10000000 }),
    /ledger records/,
  );
  assert.throws(
    () => requireLedgerProof(p, { records: 11000, bytes: p.work.bytes + 1 }),
    /ledger bytes/,
  );
});
