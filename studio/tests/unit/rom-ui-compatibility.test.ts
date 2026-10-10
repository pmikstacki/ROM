import test from "node:test";
import assert from "node:assert/strict";
import * as studio from "../../src/ui.ts";
import * as shared from "rom-ui/ui";
test("Studio's stable UI entry uses the same shared implementation", () => {
 for (const name of ["createLatestRequest", "createSelection", "resolveSourceLink", "captureExportSnapshot", "attachFollowLatest", "validateLayout"] as const) assert.equal(studio[name], shared[name], name);
});
