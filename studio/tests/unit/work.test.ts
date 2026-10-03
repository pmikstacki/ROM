import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/lib/client/client.ts";
const handle = "a".repeat(64),
  other = "b".repeat(64);
function view() {
  return {
    protocol_version: 1,
    handle,
    version: { generation: "history", revision: 2 },
    category: "Reaction",
    definition: { name: "follow", version: 1 },
    state: "Pending",
    attempts: 0,
    due: 10,
    delivery: null,
    source: null,
    target: null,
  };
}
test("work reads bind handles and validate protocol, state and counters", async () => {
  let value = view();
  const c = createClient({
    base: "/api",
    fetch: async () => Response.json(value),
  });
  await c.work("read", { handle });
  value.handle = other;
  await assert.rejects(c.work("read", { handle }), /identity/);
  value = view();
  value.protocol_version = 2;
  await assert.rejects(c.work("read", { handle }), /version/);
  value = view();
  value.state = "Unknown";
  await assert.rejects(c.work("read", { handle }), /state/);
  value = view();
  value.attempts = -1;
  await assert.rejects(c.work("read", { handle }), /integer/);
});
test("operator control binds key, operation, history and coherent revision", async () => {
  const request = {
    handle,
    expected: { generation: "history", revision: 2n },
    key: "retry-1",
    retry_epoch: 0n,
    operation: "Retry",
  };
  let value = {
    protocol_version: 1,
    handle,
    version: { generation: "history", revision: 3 },
    key: "retry-1",
    operation: "Retry",
    outcome: "Scheduled",
    replayed: false,
  };
  const c = createClient({
    base: "/api",
    fetch: async () => Response.json(value),
  });
  await c.work("control", request);
  value.key = "other";
  await assert.rejects(c.work("control", request), /correspondence/);
  value.key = "retry-1";
  value.version.generation = "other";
  await assert.rejects(c.work("control", request), /correspondence/);
  value.version.generation = "history";
  value.version.revision = 2;
  await assert.rejects(c.work("control", request), /revision/);
});
test("work capabilities and lists reject invalid booleans, handles, duplicate rows and limits", async () => {
  let value: unknown = {
    protocol_version: 1,
    inspect: true,
    retry: false,
    reconcile: false,
  };
  const c = createClient({
    base: "/api",
    maxRows: 1,
    fetch: async () => Response.json(value),
  });
  await c.work("capabilities", {});
  value = {
    protocol_version: 1,
    inspect: "yes",
    retry: false,
    reconcile: false,
  };
  await assert.rejects(c.work("capabilities", {}), /boolean/);
  value = { protocol_version: 1, records: [view(), view()], cursor: null };
  await assert.rejects(c.work("list", { limit: 1 }), /limit/);
  value = {
    protocol_version: 1,
    records: [{ ...view(), handle: "invalid" }],
    cursor: null,
  };
  await assert.rejects(c.work("list", { limit: 1 }), /handle/);
});

test("response is bound to the original submitted control despite caller edits", async () => {
  let resolve!: (response: Response) => void;
  const client = createClient({
    base: "/api",
    fetch: async () => new Promise((r) => (resolve = r)),
  });
  const request = {
    handle,
    expected: { generation: "history", revision: 2n },
    key: "retry-1",
    retry_epoch: 0n,
    operation: "Retry",
  };
  const pending = client.work("control", request);
  request.key = "changed";
  request.expected.revision = 20n;
  resolve(
    Response.json({
      protocol_version: 1,
      handle,
      version: { generation: "history", revision: 3 },
      key: "retry-1",
      operation: "Retry",
      outcome: "Scheduled",
      replayed: false,
    }),
  );
  await pending;
});
