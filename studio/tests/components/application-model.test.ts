import { test } from "node:test";
import assert from "node:assert/strict";
import { createApplication } from "../../src/lib/application/controller.ts";
import type {
  RomClient,
  ProjectedView,
  PendingMutation,
  QuerySpec,
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
    async anchor(query, last) {
      return {
        kind: last.key.kind,
        id: last.key.id,
        schema_version: 1,
        canonical: "{}",
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let settled = false;
  const promise = new Promise<T>(
    (accept) =>
      (resolve = (value) => {
        settled = true;
        accept(value);
      }),
  );
  return {
    promise,
    resolve,
    get settled() {
      return settled;
    },
  };
}

test("query Apply retains the selected key continuously and reads its latest authorized revision", async () => {
  let reads = 0;
  const fresh: ProjectedView = { ...row, revision: 2n, value: { done: true } };
  const app = createApplication(
    fixture({
      async read() {
        return ++reads === 1 ? row : fresh;
      },
      async query(_kind, query) {
        return query.filters ? [] : [row];
      },
    }),
  );
  await app.connect();
  await app.selectRow("one");
  const selections: (ProjectedView | null)[] = [];
  const unsubscribe = app.subscribe((state) => selections.push(state.selected));
  await app.applyQuery({
    limit: 50,
    filters: [{ field: "done", value: false }],
  });
  unsubscribe();
  assert.equal(app.state.rows.length, 0);
  assert.equal(app.state.selected?.revision, 2n);
  assert.deepEqual(app.state.selected?.value, { done: true });
  assert.ok(
    selections.every(
      (selected) => selected?.key.kind === "task" && selected.key.id === "one",
    ),
  );
});

test("query Apply clears a currently denied selection even when the query contains its old row", async () => {
  const { RemoteError } = await import("../../src/lib/client/client.ts");
  let reads = 0;
  const app = createApplication(
    fixture({
      async read() {
        if (++reads > 1) throw new RemoteError("denied", 403);
        return row;
      },
    }),
  );
  await app.connect();
  await app.selectRow("one");
  await app.applyQuery({ limit: 25 });
  assert.equal(app.state.selected, null);
});

test("query Apply resets moving anchors and history while preserving live choice", async () => {
  let streams = 0;
  const queries: QuerySpec[] = [];
  const app = createApplication(
    fixture({
      async query(_kind, query) {
        queries.push(query);
        return [row];
      },
      async *observe(_kind, _query, signal) {
        streams++;
        yield [row];
        await new Promise<void>((resolve) => {
          if (signal.aborted) resolve();
          else
            signal.addEventListener("abort", () => resolve(), { once: true });
        });
      },
    }),
  );
  await app.connect();
  await app.selectKind("task", { limit: 1 });
  await app.nextPage();
  const observing = app.observe();
  try {
    await new Promise((resolve) => setImmediate(resolve));
    assert.equal(app.state.live, true);
    await app.applyQuery({
      limit: 25,
      after: app.state.query.after,
      after_id: "old",
      filters: [{ field: "done", value: false }],
    });
    await new Promise((resolve) => setImmediate(resolve));
    assert.equal(app.state.page, 1);
    assert.equal(app.state.hasPrevious, false);
    assert.deepEqual(queries.at(-1), {
      limit: 25,
      filters: [{ field: "done", value: false }],
    });
    assert.equal(app.state.live, true);
    assert.equal(streams, 2);
    await app.previousPage();
    assert.equal(app.state.page, 1);
  } finally {
    app.disconnect();
    await observing;
  }
});

for (const transition of ["disconnect", "navigation", "selection"] as const) {
  test(
    `late query selection read cannot undo ${transition}`,
    { timeout: 2000 },
    async () => {
      const entered = deferred<void>(),
        late = deferred<ProjectedView>();
      const other: ProjectedView = { ...row, key: { kind: "task", id: "two" } };
      let reads = 0;
      const app = createApplication(
        fixture({
          async query(kind) {
            return kind === "other" ? [] : [row];
          },
          async read(_kind, id) {
            if (id === "two") return other;
            if (++reads === 1) return row;
            entered.resolve();
            return late.promise;
          },
        }),
      );
      await app.connect();
      await app.selectRow("one");
      const applying = app.applyQuery({ limit: 25 });
      await entered.promise;
      if (transition === "disconnect") app.disconnect();
      else if (transition === "navigation") await app.selectKind("other");
      else await app.selectRow("two");
      late.resolve({ ...row, revision: 9n });
      await applying;
      if (transition === "selection")
        assert.equal(app.state.selected?.key.id, "two");
      else assert.equal(app.state.selected, null);
      if (transition === "disconnect")
        assert.equal(app.state.phase, "disconnected");
      if (transition === "navigation") assert.equal(app.state.kind, "other");
    },
  );
}

test("live filter exclusion retains a currently readable Resource at its latest revision", async () => {
  let reads = 0;
  const fresh: ProjectedView = { ...row, revision: 2n, value: { done: true } };
  const app = createApplication(
    fixture({
      async read() {
        return ++reads === 1 ? row : fresh;
      },
      async *observe() {
        yield [];
      },
    }),
  );
  await app.connect();
  await app.selectRow("one");
  await app.observe();
  assert.equal(app.state.rows.length, 0);
  assert.equal(app.state.selected?.revision, 2n);
  assert.deepEqual(app.state.selected?.value, { done: true });
});

test("live exclusion clears a selection when its current read is denied", async () => {
  const { RemoteError } = await import("../../src/lib/client/client.ts");
  let reads = 0;
  const app = createApplication(
    fixture({
      async read() {
        if (++reads > 1) throw new RemoteError("denied", 403);
        return row;
      },
      async *observe() {
        yield [];
      },
    }),
  );
  await app.connect();
  await app.selectRow("one");
  await app.observe();
  assert.equal(app.state.selected, null);
});

for (const transition of [
  "disconnect",
  "navigation",
  "stop",
  "selection",
] as const) {
  test(
    `late excluded live selection read cannot undo ${transition}`,
    { timeout: 2000 },
    async () => {
      const entered = deferred<void>(),
        late = deferred<ProjectedView>();
      const other: ProjectedView = { ...row, key: { kind: "task", id: "two" } };
      let reads = 0;
      const app = createApplication(
        fixture({
          async read(_kind, id) {
            if (id === "two") return other;
            if (++reads === 1) return row;
            entered.resolve();
            return late.promise;
          },
          async *observe() {
            yield [];
          },
        }),
      );
      await app.connect();
      await app.selectRow("one");
      const observing = app.observe();
      await new Promise((resolve) => setImmediate(resolve));
      assert.ok(
        entered.settled,
        "Excluded selection must be re-read under current authority",
      );
      if (transition === "disconnect") app.disconnect();
      else if (transition === "navigation") await app.selectKind("other");
      else if (transition === "selection") await app.selectRow("two");
      else app.stopLive();
      late.resolve({ ...row, revision: 9n });
      await observing;
      if (transition === "selection")
        assert.equal(app.state.selected?.key.id, "two");
      else if (transition === "stop")
        assert.equal(app.state.selected?.revision, 1n);
      else assert.equal(app.state.selected, null);
    },
  );
}
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

test("moving pagination uses server envelope, refetches previous and resets on new query", async () => {
  let anchors = 0,
    queries: any[] = [];
  const envelope = {
    kind: "task",
    id: "one",
    schema_version: 1,
    canonical: '{"version":1,"opaque_fixture":true}',
  };
  const app = createApplication(
    fixture({
      async anchor(query, last) {
        anchors++;
        assert.equal(last, row);
        return envelope;
      },
      async query(kind, query) {
        queries.push(query);
        return [row];
      },
    }),
  );
  await app.connect();
  await app.selectKind("task", { limit: 1 });
  await app.nextPage();
  assert.equal(app.state.page, 2);
  assert.equal(app.state.query.after, envelope);
  assert.equal(anchors, 1);
  await app.previousPage();
  assert.equal(app.state.page, 1);
  assert.equal(app.state.query.after, undefined);
  await app.nextPage();
  await app.selectKind("task", {
    limit: 1,
    filters: [{ field: "done", value: false }],
  });
  assert.equal(app.state.page, 1);
  assert.equal(app.state.hasPrevious, false);
  assert.equal(app.state.query.after, undefined);
  assert.equal(queries.length, 6);
});
test("delayed anchor cannot paginate a different Resource", async () => {
  let resolve!: (value: any) => void;
  const app = createApplication(
    fixture({
      async anchor() {
        return await new Promise((r) => (resolve = r));
      },
    }),
  );
  await app.connect();
  await app.selectKind("task", { limit: 1 });
  const next = app.nextPage();
  await app.selectKind("other", { limit: 1 });
  resolve({ kind: "task", id: "one", schema_version: 1, canonical: "{}" });
  await next;
  assert.equal(app.state.kind, "other");
  assert.equal(app.state.page, 1);
  assert.equal(app.state.query.after, undefined);
});

test("pagination history is bounded and first page remains available", async () => {
  const app = createApplication(fixture());
  await app.connect();
  await app.selectKind("task", { limit: 1 });
  for (let i = 0; i < 140; i++) await app.nextPage();
  assert.equal(app.state.page, 141);
  for (let i = 0; i < 128; i++) await app.previousPage();
  assert.equal(app.state.page, 13);
  assert.equal(app.state.hasPrevious, false);
  await app.firstPage();
  assert.equal(app.state.page, 1);
  assert.equal(app.state.query.after, undefined);
});

test("moving page changes cancel the old live stream and open a new one", async () => {
  let streams = 0,
    aborts = 0;
  const app = createApplication(
    fixture({
      async *observe(kind, query, signal) {
        streams++;
        yield [row];
        await new Promise<void>((resolve) => {
          if (signal.aborted) {
            aborts++;
            resolve();
          } else
            signal.addEventListener(
              "abort",
              () => {
                aborts++;
                resolve();
              },
              { once: true },
            );
        });
      },
    }),
  );
  await app.connect();
  await app.selectKind("task", { limit: 1 });
  const observing = app.observe();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(app.state.live, true);
  await app.nextPage();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(streams, 2);
  assert.equal(aborts, 1);
  assert.equal(app.state.page, 2);
  app.disconnect();
  await observing;
  assert.equal(aborts, 2);
});

test("query Apply preserves live table choice when only the selected Resource read is denied", async () => {
  const { RemoteError } = await import("../../src/lib/client/client.ts");
  let reads = 0;
  const app = createApplication(
    fixture({
      async read() {
        if (++reads > 1) throw new RemoteError("denied", 403);
        return row;
      },
      async *observe(_kind, _query, signal) {
        yield [row];
        await new Promise<void>((resolve) => {
          if (signal.aborted) resolve();
          else
            signal.addEventListener("abort", () => resolve(), { once: true });
        });
      },
    }),
  );
  await app.connect();
  await app.selectRow("one");
  const observing = app.observe();
  try {
    await new Promise((resolve) => setImmediate(resolve));
    await app.applyQuery({ limit: 25 });
    await new Promise((resolve) => setImmediate(resolve));
    assert.equal(app.state.selected, null);
    assert.equal(app.state.live, true);
  } finally {
    app.disconnect();
    await observing;
  }
});
