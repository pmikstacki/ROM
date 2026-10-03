#!/usr/bin/env node
// Summarize raw query samples. This does not establish statistical significance.
import { readFileSync } from 'node:fs';

const paths = process.argv.slice(2);
if (!paths.length) throw new Error('Usage: summarize-query-measurements.mjs JSONL...');
const groups = new Map();
for (const path of paths) {
  let label = path;
  for (const line of readFileSync(path, 'utf8').trim().split('\n')) {
    const row = JSON.parse(line);
    if (row.record === 'run') label = row.label;
    if (!['query', 'production_query', 'heap'].includes(row.record)) continue;
    const dimensions = {
      label, record: row.record, distribution: row.dataset.distribution,
      seed: row.dataset.seed, size: row.size, case: row.case, mode: row.mode,
    };
    const key = JSON.stringify(dimensions);
    if (!groups.has(key)) groups.set(key, { dimensions, rows: [] });
    groups.get(key).rows.push(row);
  }
}
function distribution(values) {
  const sorted = values.sort((a, b) => a - b);
  const n = sorted.length;
  return {
    count: n, min: sorted[0],
    median: n % 2 ? sorted[Math.floor(n / 2)] : (sorted[n / 2 - 1] + sorted[n / 2]) / 2,
    p95_nearest_rank: sorted[Math.ceil(0.95 * n) - 1], max: sorted[n - 1],
  };
}
const summaries = [];
for (const { dimensions, rows } of groups.values()) {
  const values = {};
  for (const field of [
    'elapsed_ns', 'storage_elapsed_ns', 'decoded_rows', 'decoded_bytes',
    'materialization_vm_steps', 'result_count', 'match_count',
    'probe_rows', 'probe_statements', 'probe_vm_steps', 'probe_elapsed_ns',
  ]) {
    const numbers = rows.map(row => row[field]).filter(value => typeof value === 'number');
    if (numbers.length) values[field] = distribution(numbers);
  }
  for (const field of [
    'allocations', 'total_allocated_bytes', 'peak_tracked_bytes', 'tracked_bytes_at_return',
  ]) {
    const numbers = rows.map(row => row.heap?.[field]).filter(value => typeof value === 'number');
    if (numbers.length) values[field] = distribution(numbers);
  }
  summaries.push({ ...dimensions, samples: rows.length,
    actual_strategies: [...new Set(rows.map(row => row.actual_strategy).filter(Boolean))], values });
}
process.stdout.write(`${JSON.stringify(summaries, null, 2)}\n`);
