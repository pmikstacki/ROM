import { test } from "node:test";
import assert from "node:assert/strict";
import {
  admitReferenceResult,
  referenceSearchAllowed,
} from "../../src/lib/ui/components/reference-picker.ts";
test("reference candidate admission detaches exact IDs and bounds count and UTF8 bytes", () => {
  const candidates = [{ id: "  α/exact  ", title: "Private" }];
  const result = admitReferenceResult({
    status: "ready",
    candidates,
    limited: false,
  });
  candidates[0].title = "mutated";
  assert.deepEqual(result, {
    status: "ready",
    candidates: [{ id: "  α/exact  ", title: "Private" }],
    limited: false,
  });
  assert.throws(() =>
    admitReferenceResult({
      status: "ready",
      candidates: Array.from({ length: 21 }, (_, i) => ({
        id: String(i),
        title: "x",
      })),
      limited: false,
    }),
  );
  assert.throws(() =>
    admitReferenceResult({
      status: "ready",
      candidates: [{ id: "id", title: "α".repeat(32768) }],
      limited: false,
    }),
  );
});
test("reference admission rejects malformed/duplicate candidates and bounds search UTF8", () => {
  assert.throws(() =>
    admitReferenceResult({
      status: "ready",
      candidates: [
        { id: "id", title: "x" },
        { id: "id", title: "y" },
      ],
      limited: false,
    }),
  );
  assert.throws(() =>
    admitReferenceResult({
      status: "ready",
      candidates: [{ id: 42, title: "x" }],
      limited: false,
    }),
  );
  assert.equal(referenceSearchAllowed("α".repeat(512)), true);
  assert.equal(referenceSearchAllowed("α".repeat(513)), false);
});
