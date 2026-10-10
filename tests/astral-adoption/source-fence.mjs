// Complete caller inputs and best-effort final observations for the isolated trial.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

export function auditSourceInputs(witness) {
  if (!Array.isArray(witness?.source_inputs) || witness.source_inputs.length < 1 ||
      witness.source_inputs.length > 32) throw Error('complete source closure required');
  for (const input of witness.source_inputs) {
    const digest = createHash('sha256').update(readFileSync(input.path)).digest('hex');
    if (digest !== input.sha256) throw Error('consumer source closure changed');
  }
}

export function finalObservation(record, stage, observe) {
  try { return observe(); } catch (error) {
    record.status = 'failed';
    (record.final_failures ??= []).push({ stage, category: error?.name ?? 'Error' });
    return undefined;
  }
}
