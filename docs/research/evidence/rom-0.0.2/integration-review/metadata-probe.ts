import assert from 'node:assert/strict';
import { discovery } from '../../../../../studio/src/lib/client/discovery.ts';
import { parseWire, stringifyWire, objectInput } from '../../../../../studio/src/lib/client/codec.ts';

const catalog = (shape: unknown, input: unknown = { type: 'unit' }, codec?: unknown) => parseWire(JSON.stringify({ version: 1, resources: [{ kind: 'items', version: 1, fields: [{ name: 'field', shape, ...(codec ? { codec } : {}) }], actions: ['run'], action_inputs: [{ name: 'run', version: 1, input }] }] }));
const hidden = parseWire('{"version":1,"resources":[{"kind":"items","version":1,"fields":[],"actions":["visible-name"],"action_inputs":[]}]}');
assert.equal(discovery(hidden).resources[0].action_inputs.length, 0);
console.log(JSON.stringify({ probe: 'hidden-input-subset-correction', accepted: true }));

for (const [name, shape, input, codec] of [
  ['empty-enum', { type: 'enum', value: [] }, { type: 'unit' }],
  ['enum-over256', { type: 'enum', value: Array.from({ length: 257 }, (_, index) => String(index)) }, { type: 'unit' }],
  ['nested-optional', { type: 'list', value: { type: 'optional', value: { type: 'string' } } }, { type: 'unit' }],
  ['nested-nullable', { type: 'nullable', value: { type: 'nullable', value: { type: 'string' } } }, { type: 'unit' }],
  ['scalar-optional', { type: 'string' }, { type: 'scalar', value: { shape: { type: 'optional', value: { type: 'string' } } } }],
  ['codec-name-over256bytes', { type: 'string' }, { type: 'unit' }, { name: 'é'.repeat(129), version: 1 }],
] as const) {
  let accepted = true;
  try { discovery(catalog(shape, input, codec)); } catch { accepted = false; }
  console.log(JSON.stringify({ probe: 'native-impossible-metadata', name, accepted }));
}

const parsed = parseWire('{"rate":0.0}');
assert.equal(stringifyWire(parsed), '{"rate":0.0}');
const normalized = objectInput([{ name: 'rate', shape: { type: 'f64' } }], { rate: { mode: 'value', value: 0 } });
assert.equal(stringifyWire(normalized), '{"rate":0}');
console.log(JSON.stringify({ probe: 'float-authoring-category', raw_replay: stringifyWire(parsed), normalized_form: stringifyWire(normalized), value_equal: normalized.rate === 0 }));
