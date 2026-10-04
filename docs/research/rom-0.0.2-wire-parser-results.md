# Studio wire parser: native fixture measurements

## Question and method

Measure the adopted parser on actual native response bodies. Keep correctness separate from timing. This experiment ran after adoption, not before it.

The capture script started the accepted native demo for SQLite and redb. Chromium completed login against the actual local OIDC provider fixture. The browser read seeded Task and Inventory Resources. The script retained only successful discovery, query and read bodies. It did not retain cookies, tokens, auth responses or request headers. The identity provider uses a development memory adapter; it is not a production provider acceptance claim.

The immutable binary SHA-256 is `60576302fec8f9d2439b3da34cd23ea8411c9277ce575c2bad9c2386986b9d7c`. The parser SHA-256 is `acbfa47fa28e9edef94220a297da5a2c1755f3ce5aa00a5c8b13aa8ee2e03d6c`. The capture manifest records each response body hash, byte count, backend and request. The benchmark rejects an identity mismatch.

Each operation had 100 warm-up iterations, then 31 batches of 100 iterations. Values are microseconds per operation. Median is the middle batch; p95 is the 30th sorted batch. Parse and encode timings are separate. Both serializers receive an already parsed value.

## Results

| Native fixture | Bytes | Wire parse median | Wire parse p95 | Wire encode median | JSON.parse median | JSON.stringify median |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| redb-1-discover.json | 2981 | 56.55 | 71.37 | 314.16 | 14.41 | 4.96 |
| redb-2-query.json | 2 | 0.24 | 0.44 | 0.51 | 0.06 | 0.05 |
| redb-3-query.json | 297 | 6.71 | 10.51 | 31.46 | 1.31 | 0.54 |
| redb-4-read.json | 97 | 2.19 | 3.54 | 8.53 | 0.46 | 0.21 |
| redb-5-query.json | 102 | 2.46 | 3.09 | 8.84 | 0.50 | 0.23 |
| redb-6-read.json | 100 | 2.35 | 2.99 | 8.36 | 0.46 | 0.22 |
| sqlite-1-discover.json | 2981 | 54.96 | 58.99 | 314.10 | 14.12 | 4.83 |
| sqlite-2-query.json | 2 | 0.46 | 1.03 | 0.89 | 0.05 | 0.04 |
| sqlite-3-query.json | 297 | 6.02 | 7.17 | 27.26 | 1.31 | 0.54 |
| sqlite-4-read.json | 97 | 2.22 | 7.82 | 8.43 | 0.46 | 0.21 |
| sqlite-5-query.json | 102 | 2.53 | 4.34 | 8.81 | 0.50 | 0.23 |
| sqlite-6-read.json | 100 | 2.42 | 4.89 | 8.32 | 0.46 | 0.22 |

All twelve seeded responses passed parse/encode/parse stability. The built-in JSON path was equivalent for these small samples. These samples do not contain unsafe integer boundaries. Existing codec tests cover those boundaries; these measurements do not replace them.

The strict parser has measurable overhead relative to JSON.parse. These finite fixture costs do not justify replacing its integer, duplicate-key, depth, Unicode or byte-limit guarantees. No parser optimization was accepted from this experiment.

## Limits and reproduction

This run used Node v22.16.0, linux/x64. It measures parser wall-clock time in Node. It does not measure browser rendering, database work, network time, sustained stream retention or heap bounds. The bodies contain 2–2981 bytes; no maximum-payload performance claim follows. Native response serialization and JSON parsing are not interchangeable correctness contracts.

From the repository root, capture new fixture bodies with the accepted immutable binary and built assets:

```sh
ROM_STUDIO_DEMO_BINARY=/var/tmp/rom-studio-secure-files-acceptance/rom-demo \
ROM_STUDIO_ASSETS="$PWD/studio/dist" \
node scripts/research/capture-studio-wire.mjs
node --experimental-strip-types scripts/research/benchmark-studio-wire.mjs
```

Evidence: [capture manifest](evidence/rom-0.0.2/wire-parser/capture.json), [measured results](evidence/rom-0.0.2/wire-parser/benchmark.json), [parser source](../../studio/src/lib/client/serialization.ts), [codec tests](../../studio/tests/unit/codec.test.ts). The final complete producer must still verify the final source. These measurements do not establish release completion.
