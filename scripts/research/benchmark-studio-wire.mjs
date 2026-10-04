// Finite Node parser experiment. This does not measure browser rendering or I/O.
import { readFile, writeFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";
import {
  parseWire,
  stringifyWire,
} from "../../studio/src/lib/client/serialization.ts";

const directory = resolve(
  process.argv[2] ?? "docs/research/evidence/rom-0.0.2/wire-parser",
);
const capture = JSON.parse(
  await readFile(join(directory, "capture.json"), "utf8"),
);
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const batches = 31,
  iterations = 100;
function measure(operation) {
  for (let i = 0; i < 100; i++) operation();
  const times = [];
  for (let batch = 0; batch < batches; batch++) {
    const start = performance.now();
    for (let i = 0; i < iterations; i++) operation();
    times.push(((performance.now() - start) * 1000) / iterations);
  }
  times.sort((a, b) => a - b);
  return { medianMicroseconds: times[15], p95Microseconds: times[29] };
}
const results = [];
for (const fixture of capture.fixtures) {
  const bytes = await readFile(join(directory, fixture.file));
  if (hash(bytes) !== fixture.sha256 || bytes.length !== fixture.bytes)
    throw Error("fixture identity mismatch");
  const text = bytes.toString("utf8");
  const value = parseWire(text);
  const encoded = stringifyWire(value);
  const reparsed = parseWire(encoded);
  if (stringifyWire(reparsed) !== encoded)
    throw Error("wire roundtrip mismatch");
  const builtinValue = JSON.parse(text);
  const builtin = JSON.stringify(builtinValue);
  const equivalent = stringifyWire(parseWire(builtin)) === encoded;
  results.push({
    ...fixture,
    builtinEquivalent: equivalent,
    parseWire: measure(() => parseWire(text)),
    stringifyWire: measure(() => stringifyWire(value)),
    jsonParse: measure(() => JSON.parse(text)),
    jsonStringify: measure(() => JSON.stringify(builtinValue)),
  });
}
const source = await readFile(
  resolve("studio/src/lib/client/serialization.ts"),
);
const result = {
  recordedAt: new Date().toISOString(),
  node: process.version,
  platform: process.platform,
  arch: process.arch,
  sourceSha256: hash(source),
  binarySha256: capture.binarySha256,
  batches,
  iterations,
  jsonStringifyIncludesParse: false,
  scope:
    "retained native seeded fixtures; Node wall-clock; no network, database, DOM or heap claim",
  results,
};
await writeFile(
  join(directory, "benchmark.json"),
  JSON.stringify(result, null, 2) + "\n",
);
console.log(
  JSON.stringify({
    fixtures: results.length,
    builtinNonEquivalent: results.filter((r) => !r.builtinEquivalent).length,
  }),
);
