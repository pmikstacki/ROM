import { test } from "node:test";
import assert from "node:assert/strict";
import { createApplication } from "../../src/lib/application/controller.ts";
import type {
  RomClient,
  ProjectedView,
  PendingMutation,
} from "../../src/lib/client/types.ts";
const row: ProjectedView = {
  key: { kind: "task", id: "one" },
  revision: 1n,
  value: { done: false },
};
function fixture(overrides: Partial<RomClient> = {}): RomClient {
  return {
    generation: 0,
    invalidateSession() {},
    async discover() {
      return {
        version: 1,
        resources: [
          {
            kind: "task",
            version: 1,
            fields: [],
            actions: [],
            action_inputs: [],
          },
        ],
      };
    },
    async query() {
      return [row];
    },
    async read() {
      return row;
    },
    prepare(request) {
      return { request, fingerprint: "exact", generation: 0, state: "pending" };
    },
    async submit(mutation) {
      mutation.state = "succeeded";
      return row;
    },
    async *observe() {},
    async journal() {
      return null;
    },
    async work() {
      return {};
    },
    ...overrides,
  };
}
test("connect exposes discovered resources and selected rows", async () => {
  const app = createApplication(fixture());
  await app.connect();
  assert.equal(app.state.phase, "ready");
  assert.equal(app.state.descriptors[0].kind, "task");
  assert.equal(app.state.rows[0], row);
});
test("late query cannot replace another navigation", async () => {
  let release!: (v: ProjectedView[]) => void;
  const app = createApplication(
    fixture({
      query: async (kind) =>
        kind === "slow"
          ? await new Promise((resolve) => (release = resolve))
          : [row],
    }),
  );
  const slow = app.selectKind("slow");
  await app.selectKind("task");
  release([]);
  await slow;
  assert.equal(app.state.kind, "task");
  assert.equal(app.state.rows.length, 1);
});
test("unknown outcome blocks another mutation and exact retry uses same object", async () => {
  let attempts = 0;
  const seen: PendingMutation[] = [];
  const app = createApplication(
    fixture({
      submit: async (m) => {
        seen.push(m);
        if (!attempts++) {
          m.state = "unknown";
          throw Error("lost reply");
        }
        m.state = "succeeded";
        return row;
      },
    }),
  );
  await app.connect();
  await assert.rejects(app.mutate("one", 1n, { type: "delete" }));
  assert.equal(app.state.pending?.state, "unknown");
  await assert.rejects(app.mutate("two", 1n, { type: "delete" }), /Resolve/);
  await app.retry();
  assert.equal(seen[0], seen[1]);
  assert.equal(app.state.pending?.state, "succeeded");
});
test("disconnect aborts and suppresses delayed discovery", async () => {
  let release!: (v: any) => void;
  let invalidated = false;
  const app = createApplication(
    fixture({
      discover: async () => await new Promise((r) => (release = r)),
      invalidateSession() {
        invalidated = true;
      },
    }),
  );
  const connecting = app.connect();
  app.disconnect();
  release({ version: 1, resources: [] });
  await connecting;
  assert.equal(app.state.phase, "disconnected");
  assert.equal(invalidated, true);
});

test("finite lease revalidation reopens once with fresh snapshot", async () => {
  const { RemoteError } = await import("../../src/lib/client/client.ts");
  let streams = 0,
    checks = 0;
  const app = createApplication(
    fixture({
      async *observe() {
        streams++;
        if (streams === 1) throw new RemoteError("identity_expired", 0);
        yield [row];
      },
    }),
    () => "key",
    async () => {
      checks++;
      return true;
    },
  );
  await app.connect();
  await app.observe();
  assert.equal(checks, 1);
  assert.equal(streams, 2);
  assert.equal(app.state.rows.length, 1);
});
test("repeated finite lease failure stays bounded and clears projections", async () => {
  const { RemoteError } = await import("../../src/lib/client/client.ts");
  let streams = 0;
  const app = createApplication(
    fixture({
      async *observe() {
        streams++;
        throw new RemoteError("identity_expired", 0);
      },
    }),
    () => "key",
    async () => true,
  );
  await app.connect();
  await app.selectRow("one");
  await app.observe();
  assert.equal(streams, 2);
  assert.equal(app.state.rows.length, 0);
  assert.equal(app.state.selected, null);
  assert.match(app.state.error, /Live query stopped/);
});

test("separated normal lease expiries renew after each accepted snapshot", async () => {
  const { RemoteError } = await import("../../src/lib/client/client.ts");
  let streams = 0,
    checks = 0;
  const app = createApplication(
    fixture({
      async *observe() {
        streams++;
        yield [row];
        if (streams < 4) throw new RemoteError("identity_expired", 0);
      },
    }),
    () => "key",
    async () => {
      checks++;
      return true;
    },
  );
  await app.connect();
  await app.observe();
  assert.equal(checks, 3);
  assert.equal(streams, 4);
  assert.equal(app.state.rows.length, 1);
  assert.equal(app.state.error, "");
});
