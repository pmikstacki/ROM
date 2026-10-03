import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/lib/client/client.ts";
const after = { kind: "task", generation: "history", position: 2n };
function batch() {
  return {
    cursor: { kind: "task", generation: "history", position: 5 },
    events: [
      {
        position: 3,
        view: { key: { kind: "task", id: "a" }, revision: 1, value: {} },
      },
      {
        position: 5,
        view: { key: { kind: "task", id: "b" }, revision: 2, value: null },
      },
    ],
  };
}
test("journal validates cursor generation, kind and strictly advancing positions", async () => {
  let value = batch();
  const c = createClient({
    base: "/api",
    fetch: async () => Response.json(value),
  });
  const result = await c.journal("task", after);
  assert.equal(
    (result as { cursor: { position: bigint } }).cursor.position,
    5n,
  );
  value = batch();
  value.cursor.generation = "other";
  await assert.rejects(c.journal("task", after), /generation/);
  value = batch();
  value.events[1].position = 3;
  await assert.rejects(c.journal("task", after), /position/);
  value = batch();
  value.events[0].view.key.kind = "other";
  await assert.rejects(c.journal("task", after), /identity/);
  value = batch();
  value.cursor.position = 2;
  await assert.rejects(c.journal("task", after), /position/);
});
