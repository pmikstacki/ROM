// Reproduce the compact table from the preserved raw JSONL; no external packages.
import fs from 'node:fs';
const rows = fs.readFileSync(process.argv[2] ?? 'results/benchmark-ext4.jsonl', 'utf8').trim().split('\n').map(JSON.parse);
const median = values => [...values].sort((a,b)=>a-b)[Math.floor(values.length/2)];
const groups = Map.groupBy(rows, r => `${r.workload}|${r.payload_bytes}|${r.mode}`);
console.log('| Workload | Bytes | Mode | Median calls/s (min–max) | Median p50/p95/p99 µs | DB attempts | New commits |');
console.log('|---|---:|---|---:|---|---|---|');
for (const [group, r] of groups) {
 const [w,b,m]=group.split('|');const rate=r.map(v=>v.throughput_per_s);
 console.log(`| ${w} | ${b} | ${m} | ${median(rate).toFixed(0)} (${Math.min(...rate).toFixed(0)}–${Math.max(...rate).toFixed(0)}) | ${['p50_us','p95_us','p99_us'].map(k=>median(r.map(v=>v[k])).toFixed(2)).join('/')} | ${[...new Set(r.map(v=>v.db_attempts))].sort((a,b)=>a-b).join(',')} | ${[...new Set(r.map(v=>v.actual_commits))].sort((a,b)=>a-b).join(',')} |`);
}
