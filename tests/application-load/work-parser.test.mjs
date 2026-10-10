import test from "node:test";
import assert from "node:assert/strict";
import { loadWorkParser, captureWorkParserSources } from "./work-parser.mjs";
test("maintained Work parser has bounded exact source witness and rejects changed digest", async () => {
  const captured = captureWorkParserSources();
  assert.equal(captured.length, 5);
  const changed = structuredClone(captured);
  changed[0].sha256 = "0".repeat(64);
  await assert.rejects(loadWorkParser(changed), /source/);
  const parser = await loadWorkParser(captured);
  assert.equal(
    parser.workResponse(
      "capabilities",
      parser.parseWire(
        '{"protocol_version":1,"inspect":true,"retry":false,"reconcile":false}',
      ),
      {},
      2,
    ).inspect,
    true,
  );
});
test("actual public parser rejects invalid Work identity and cursor", async () => {
  const parser = await loadWorkParser();
  assert.throws(() =>
    parser.workResponse(
      "list",
      parser.parseWire('{"protocol_version":1,"records":[],"cursor":17}'),
      { limit: 2 },
      2,
    ),
  );
  assert.throws(() =>
    parser.workResponse(
      "read",
      parser.parseWire('{"protocol_version":1,"handle":"invalid"}'),
      { handle: "a".repeat(64) },
      2,
    ),
  );
});
import { readParserSource } from "./work-parser.mjs";
import {
  mkdtempSync,
  openSync,
  ftruncateSync,
  closeSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
test("parser source reads reject oversized and symbolic files before compile", () => {
  const dir = mkdtempSync(`${tmpdir()}/rom-mixed-parser-`),
    path = `${dir}/source.ts`,
    fd = openSync(path, "wx", 0o600);
  ftruncateSync(fd, 65537);
  closeSync(fd);
  assert.throws(() => readParserSource(path), /bounded/);
  symlinkSync(path, `${dir}/link.ts`);
  assert.throws(() => readParserSource(`${dir}/link.ts`), /bounded/);
  writeFileSync(`${dir}/small.ts`, "export const value=1;", {
    flag: "wx",
    mode: 0o600,
  });
  assert.equal(
    readParserSource(`${dir}/small.ts`).raw.toString(),
    "export const value=1;",
  );
});
