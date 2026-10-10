import test from "node:test";
import assert from "node:assert/strict";
import { createApplication } from "../../src/lib/application/controller.ts";
import { createClient } from "../../src/client.ts";
import { stringifyWire, parseWire } from "../../src/lib/client/codec.ts";
import type {
  PendingIntentStore,
  StoredIntent,
  DurablePrincipal,
} from "../../src/recovery.ts";

const principal = {
  authority: "fixture",
  kind: "human" as const,
  subject: "alice",
};

test("managed App bridge requires explicit trusted profile and storage", async () => {
  const { createAppSession } =
    await import("../../src/lib/application/app-session.ts");
  assert.throws(
    () =>
      createAppSession({
        base: "/",
        profile: { authority: "", now: () => 0, recovery: recovery() },
      }),
    /profile|configuration/i,
  );
  assert.throws(
    () =>
      createAppSession({
        base: "/",
        profile: {
          authority: "fixture",
          now: () => 0,
          recovery: { ...recovery(), editorStore: undefined },
        },
      }),
    /profile|configuration/i,
  );
});

test("App bridge shares refresh through binding and preserves transient context", async () => {
  const { createAppSession } =
    await import("../../src/lib/application/app-session.ts");
  let checks = 0,
    temporary = false,
    generation = "one";
  const fetcher: typeof fetch = async (url) => {
    if (String(url).endsWith("auth/session")) {
      checks++;
      return new Response(
        temporary
          ? '{"error":"closed"}'
          : JSON.stringify({
              authenticated: true,
              generation,
              csrf_token: "fixture-token",
              user_id: "alice",
              expires_at: 100,
            }),
        {
          status: temporary ? 503 : 200,
          headers: { "content-type": "application/json" },
        },
      );
    }
    return new Response(
      String(url).endsWith("discover")
        ? '{"version":1,"resources":[{"kind":"task","version":1,"fields":[],"actions":[],"action_inputs":[]}]}'
        : String(url).endsWith("query")
          ? '[{"key":{"kind":"task","id":"one"},"revision":1,"value":{"done":false}}]'
          : '{"key":{"kind":"task","id":"one"},"revision":1,"value":{"done":false}}',
      { headers: { "content-type": "application/json" } },
    );
  };
  const session = createAppSession({
    base: "http://fixture/",
    profile: { authority: "fixture", now: () => 1, recovery: recovery() },
    fetch: fetcher,
  });
  try {
    const first = session.refresh(),
      concurrent = session.refresh();
    assert.equal(first, concurrent);
    await first;
    await session.controller.selectRow("one");
    temporary = true;
    await session.refresh();
    assert.equal(session.controller.state.selected?.key.id, "one");
    assert.equal(session.controller.state.session?.status, "transient");
    temporary = false;
    generation = "two";
    await session.refresh();
    assert.equal(session.controller.state.selected?.key.id, "one");
    assert.equal(session.controller.state.session?.status, "active");
    assert.equal(checks, 3);
  } finally {
    session.destroy();
  }
});

test("managed draft snapshots preserve decimal lexical categories", async () => {
  const client = fixture(),
    app = createApplication(client, undefined, undefined, {
      recovery: recovery(),
    });
  await app.rebindSession({ client, principal });
  const operation = parseWire(
    '{"type":"replace","input":{"decimal":1.0,"exponent":1e0,"negative_zero":-0.0,"integer":9007199254740993}}',
  );
  await app.stageDraft("one", operation);
  assert.equal(
    stringifyWire(app.state.recovery!.state.draft),
    stringifyWire(operation),
  );
  assert.equal(typeof app.state.page, "number");
  assert.equal(typeof app.state.query.limit, "number");
  assert.equal(typeof app.state.descriptors[0].version, "number");
  app.disconnect();
});

test("editor draft snapshots preserve decimal lexical categories", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  const config = recovery(),
    drafts = createEditorDrafts({
      recovery: config,
      binding: () => ({ client: fixture(), principal }),
      publish() {},
    });
  const intents = parseWire(
    '{"decimal":1.0,"exponent":1e0,"negative_zero":-0.0,"integer":9007199254740993}',
  );
  await drafts.stage(
    { kind: "task", id: "one" },
    {
      mode: "resource",
      descriptor: "task-v1",
      baseRevision: 1n,
      intents,
      editors: {},
    },
  );
  assert.equal(
    stringifyWire(drafts.state.snapshot!.intents),
    stringifyWire(intents),
  );
});
function store(): PendingIntentStore {
  const slots = new Map<string, StoredIntent>();
  return {
    async read(slot) {
      return structuredClone(slots.get(slot) ?? null);
    },
    async compareExchange(slot, expected, next) {
      if ((slots.get(slot)?.version ?? null) !== expected) return false;
      if (next) slots.set(slot, structuredClone(next));
      else slots.delete(slot);
      return true;
    },
  };
}
function fixture(
  onSubmit?: (body: string) => Promise<Response>,
  onRequest?: (route: string) => Promise<Response | null>,
) {
  const row =
    '{"key":{"kind":"task","id":"one"},"revision":1,"value":{"done":false}}';
  return createClient({
    base: "http://fixture/api",
    fetch: async (url, init) => {
      const custom = await onRequest?.(String(url).split("/").at(-1)!);
      return (
        custom ??
        (String(url).endsWith("invoke") && onSubmit
          ? onSubmit(String(init?.body))
          : new Response(
              String(url).endsWith("discover")
                ? '{"version":1,"resources":[{"kind":"task","version":1,"fields":[],"actions":[],"action_inputs":[]}]}'
                : String(url).endsWith("query")
                  ? `[${row}]`
                  : row,
              { headers: { "content-type": "application/json" } },
            ))
      );
    },
  });
}
function recovery() {
  let version = 0;
  return {
    namespace: "fixture",
    intentStore: store(),
    editorStore: store(),
    slot: (
      owner: DurablePrincipal,
      target: { kind: string; id: string },
      record: string,
    ) => `${owner.subject}/${target.kind}/${target.id}/${record}`,
    newVersion: () => String(++version),
    newCommandKey: () => "command-one",
    retryEpoch: () => 1n,
    maxBytes: 65536,
  };
}

test("managed pause and same-principal rebind retain selected target and query context", async () => {
  const client = fixture();
  const app = createApplication(client, undefined, undefined, {
    recovery: recovery(),
  });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  const selected = app.state.selected;
  app.pauseSession("transient");
  assert.equal(app.state.selected?.key.id, "one");
  assert.equal(app.state.session?.status, "transient");
  await app.rebindSession({ client, principal });
  assert.equal(app.state.selected?.key.id, selected?.key.id);
  assert.equal(app.state.kind, "task");
  assert.equal(app.state.session?.status, "active");
  app.disconnect();
});

for (const route of ["discover", "query", "read"]) {
  for (const failure of ["503", "network"]) {
    test(`same-principal ${route} ${failure} renewal keeps context stale and dispatch disabled`, async () => {
      const client = fixture(async () => {
        throw Error("lost acknowledgement");
      });
      const app = createApplication(client, undefined, undefined, {
        recovery: recovery(),
      });
      await app.rebindSession({ client, principal });
      await app.selectRow("one");
      await app.stageEditorDraft("one", {
        mode: "resource",
        descriptor: "task-v1",
        baseRevision: 1n,
        intents: {},
        editors: { count: { text: "invalid local text" } },
      });
      await assert.rejects(
        app.mutate("one", 1n, { type: "replace", input: { done: true } }),
      );
      const prior = app.state;
      const next = fixture(undefined, async (current) => {
        if (current !== route) return null;
        if (failure === "network") throw Error("network unavailable");
        return new Response('{"error":"closed"}', {
          status: 503,
          headers: { "content-type": "application/json" },
        });
      });
      next.invalidateSession();
      await app.rebindSession({ client: next, principal });
      assert.equal(app.state.selected?.key.id, "one");
      assert.deepEqual(app.state.rows, prior.rows);
      assert.deepEqual(app.state.query, prior.query);
      assert.equal(
        app.state.editor?.snapshot?.editors.count.text,
        "invalid local text",
      );
      assert.equal(app.state.recovery?.state.commitKnowledge, "unknown");
      assert.equal(app.state.session?.status, "transient");
      assert.equal(app.state.session?.stale, true);
      assert.equal(app.state.session?.mutationAllowed, false);
      await assert.rejects(app.retry(), /session|authority/i);
      await app.rebindSession({ client: fixture(), principal });
      assert.equal(app.state.session?.status, "active");
      assert.equal(app.state.selected?.key.id, "one");
      app.disconnect();
    });
  }
}

for (const [category, status] of [
  ["denied", 403],
  ["missing", 404],
] as const) {
  test(`confirmed renewal ${category} clears private projection but retains unknown knowledge`, async () => {
    const client = fixture(async () => {
      throw Error("lost acknowledgement");
    });
    const config = recovery();
    const app = createApplication(client, undefined, undefined, {
      recovery: config,
    });
    await app.rebindSession({ client, principal });
    await app.selectRow("one");
    await app.stageEditorDraft("one", {
      mode: "resource",
      descriptor: "task-v1",
      baseRevision: 1n,
      intents: {},
      editors: { count: { text: "private draft" } },
    });
    await assert.rejects(
      app.mutate("one", 1n, { type: "replace", input: { done: true } }),
    );
    const accepted = await config.intentStore.read("alice/task/one/intent");
    await app.rebindSession({
      client: fixture(undefined, async (route) =>
        route === "read"
          ? new Response(`{"error":"${category}"}`, {
              status,
              headers: { "content-type": "application/json" },
            })
          : null,
      ),
      principal,
    });
    assert.equal(app.state.selected, null);
    assert.equal(app.state.error, "");
    assert.deepEqual(app.state.rows, []);
    assert.equal(app.state.editor?.snapshot, null);
    assert.equal(app.state.session?.mutationAllowed, false);
    assert.equal(app.state.recovery?.state.commitKnowledge, "unknown");
    assert.equal(app.state.recovery?.state.draft, null);
    assert.equal(app.state.recovery?.state.result, null);
    await assert.rejects(
      app.stageDraft("one", {
        type: "replace",
        input: { secret: "later private text" },
      }),
      /denied|authority|session/i,
    );
    assert.equal(app.state.recovery?.state.draft, null);
    await assert.rejects(app.retry(), /session|authority/i);
    assert.deepEqual(
      await config.intentStore.read("alice/task/one/intent"),
      accepted,
    );
    app.disconnect();
  });
}

test("principal clearing publishes no prior recovery draft or result", async () => {
  const client = fixture(
    async () =>
      new Response(
        '{"key":{"kind":"task","id":"one"},"revision":2,"value":{"done":true}}',
        { headers: { "content-type": "application/json" } },
      ),
  );
  const app = createApplication(client, undefined, undefined, {
    recovery: recovery(),
  });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  await app.mutate("one", 1n, { type: "replace", input: { done: true } });
  let leaked = false;
  const off = app.subscribe((state) => {
    if (
      state.phase === "disconnected" &&
      (state.recovery?.state.result || state.recovery?.state.draft)
    )
      leaked = true;
  });
  await app.rebindSession({ client, principal: null });
  assert.equal(leaked, false);
  assert.equal(app.state.selected, null);
  assert.equal(app.state.recovery?.state.result ?? null, null);
  off();
  app.disconnect();
});

for (const deniedRoute of ["discover", "read"]) {
  test(`confirmed ${deniedRoute} denial hides obsolete descriptor metadata in subsequent publications`, async () => {
    const descriptor = (kind: string) => ({
      kind,
      version: 1,
      fields: [],
      actions: [],
      action_inputs: [],
    });
    const client = fixture(undefined, async (route) =>
      route === "discover"
        ? new Response(
            JSON.stringify({
              version: 1,
              resources: [descriptor("task"), descriptor("private_kind")],
            }),
            { headers: { "content-type": "application/json" } },
          )
        : null,
    );
    const app = createApplication(client, undefined, undefined, {
      recovery: recovery(),
    });
    await app.rebindSession({ client, principal });
    await app.selectRow("one");
    assert.equal(
      app.state.descriptors.some((item) => item.kind === "private_kind"),
      true,
    );
    let freshDiscovery = false,
      leaked = false;
    const off = app.subscribe((state) => {
      if (
        freshDiscovery &&
        state.descriptors.some((item) => item.kind === "private_kind")
      )
        leaked = true;
      if (state.session?.status === "denied" && state.pending?.result)
        leaked = true;
    });
    const next = fixture(undefined, async (route) => {
      if (route === "discover") {
        freshDiscovery = true;
        return deniedRoute === "discover"
          ? new Response('{"error":"denied"}', {
              status: 403,
              headers: { "content-type": "application/json" },
            })
          : new Response(
              JSON.stringify({ version: 1, resources: [descriptor("task")] }),
              { headers: { "content-type": "application/json" } },
            );
      }
      if (route === "read")
        return new Response('{"error":"denied"}', {
          status: 403,
          headers: { "content-type": "application/json" },
        });
      return null;
    });
    await app.rebindSession({ client: next, principal });
    assert.equal(leaked, false);
    assert.deepEqual(app.state.descriptors, []);
    assert.equal(app.state.pending, null);
    app.pauseSession("transient");
    assert.deepEqual(app.state.descriptors, []);
    off();
    app.disconnect();
  });
}

test("reentrant target selection cannot stage an operation into a replacement lane", async () => {
  const { createMutationLane } =
    await import("../../src/lib/application/mutation-lane.ts");
  const config = recovery(),
    client = fixture();
  let switched = false,
    replacement: Promise<void> | undefined;
  const lanes = createMutationLane({
    recovery: config,
    binding: () => ({ client, principal }),
    allowed: () => true,
    publish(snapshot) {
      if (!switched && snapshot?.target.id === "one") {
        switched = true;
        replacement = lanes.stage(
          { kind: "task", id: "two" },
          { type: "replace", input: { origin: "two" } },
        );
      }
    },
  });
  await assert.rejects(
    lanes.stage(
      { kind: "task", id: "one" },
      { type: "replace", input: { origin: "one" } },
    ),
    /changed/i,
  );
  await replacement;
  const stored = await config.intentStore.read("alice/task/two/intent");
  assert.equal(stored!.payload.includes('\\"origin\\":\\"one\\"'), false);
  assert.equal(stored!.payload.includes('\\"origin\\":\\"two\\"'), true);
  lanes.dispose();
});

test("editor restore refuses to erase outstanding invalid local draft", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  const config = recovery();
  let release!: (success: boolean) => void;
  config.editorStore = {
    async read() {
      return null;
    },
    compareExchange() {
      return new Promise((resolve) => {
        release = resolve;
      });
    },
  };
  const drafts = createEditorDrafts({
    recovery: config,
    binding: () => ({ client: fixture(), principal }),
    publish() {},
  });
  const target = { kind: "task", id: "one" };
  const pending = drafts.stage(target, {
    mode: "resource",
    descriptor: "task-v1",
    baseRevision: 1n,
    intents: {},
    editors: { count: { text: "private invalid local draft" } },
  });
  await new Promise((resolve) => setImmediate(resolve));
  const restored = drafts.restore(target);
  void restored.catch(() => {});
  try {
    assert.equal(
      drafts.state.snapshot?.editors.count.text,
      "private invalid local draft",
    );
    await assert.rejects(restored, /pending|writing|draft/i);
  } finally {
    release(true);
    await pending;
  }
});

test("reentrant editor publication cannot write an older draft after the newer draft", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  const config = recovery(),
    target = { kind: "task", id: "one" };
  const snapshot = {
    mode: "resource" as const,
    descriptor: "task-v1",
    baseRevision: 1n,
    intents: parseWire('{"decimal":1.0,"exponent":1e0,"negative_zero":-0.0}'),
    editors: { count: { text: "older" } },
  };
  let changed = false,
    later: Promise<void> | undefined;
  const drafts = createEditorDrafts({
    recovery: config,
    binding: () => ({ client: fixture(), principal }),
    publish(state) {
      if (!changed && state.status === "writing") {
        changed = true;
        later = drafts.stage(target, {
          ...snapshot,
          editors: { count: { text: "newer" } },
        });
      }
    },
  });
  await drafts.stage(target, snapshot);
  await later;
  assert.equal(drafts.state.snapshot?.editors.count.text, "newer");
  const reopened = createEditorDrafts({
    recovery: config,
    binding: () => ({ client: fixture(), principal }),
    publish() {},
  });
  await reopened.restore(target);
  assert.equal(reopened.state.snapshot?.editors.count.text, "newer");
  assert.equal(
    stringifyWire(reopened.state.snapshot!.intents),
    stringifyWire(snapshot.intents),
  );
});

test("held editor restore cannot publish over a newer staged draft", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  const config = recovery(),
    underlying = config.editorStore;
  let release!: (stored: StoredIntent | null) => void;
  config.editorStore = {
    read() {
      return new Promise((resolve) => {
        release = resolve;
      });
    },
    compareExchange: underlying.compareExchange,
  };
  const drafts = createEditorDrafts({
    recovery: config,
    binding: () => ({ client: fixture(), principal }),
    publish() {},
  });
  const target = { kind: "task", id: "one" },
    restoring = drafts.restore(target);
  void restoring.catch(() => {});
  await new Promise((resolve) => setImmediate(resolve));
  const later = drafts.stage(target, {
    mode: "resource",
    descriptor: "task-v1",
    baseRevision: 1n,
    intents: {},
    editors: { count: { text: "newer invalid" } },
  });
  release(null);
  await assert.rejects(restoring, /changed|draft/i);
  await later;
  assert.equal(drafts.state.snapshot?.editors.count.text, "newer invalid");
});

test("same-owner rebind does not permit restore while prior draft durability is outstanding", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  const config = recovery();
  let release!: (success: boolean) => void;
  config.editorStore = {
    async read() {
      return null;
    },
    compareExchange() {
      return new Promise((resolve) => {
        release = resolve;
      });
    },
  };
  const drafts = createEditorDrafts({
      recovery: config,
      binding: () => ({ client: fixture(), principal }),
      publish() {},
    }),
    target = { kind: "task", id: "one" };
  const pending = drafts.stage(target, {
    mode: "resource",
    descriptor: "task-v1",
    baseRevision: 1n,
    intents: {},
    editors: { count: { text: "held invalid draft" } },
  });
  void pending.catch(() => {});
  await new Promise((resolve) => setImmediate(resolve));
  drafts.rebind(principal);
  const restored = drafts.restore(target);
  void restored.catch(() => {});
  try {
    assert.equal(
      drafts.state.snapshot?.editors.count.text,
      "held invalid draft",
    );
    await assert.rejects(restored, /pending|draft/i);
  } finally {
    release(true);
    await assert.rejects(pending, /changed/i);
  }
});

test("reentrant pause during renewal publication cannot publish active authority", async () => {
  const client = fixture(),
    app = createApplication(client, undefined, undefined, {
      recovery: recovery(),
    });
  let paused = false;
  const off = app.subscribe((state) => {
    if (!paused && state.session?.status === "renewing") {
      paused = true;
      app.pauseSession("transient");
    }
  });
  await app.rebindSession({ client, principal });
  assert.equal(app.state.session?.status, "transient");
  assert.equal(app.state.session?.mutationAllowed, false);
  off();
  app.disconnect();
});

test("reentrant principal replacement during clearing cannot be overwritten by old binding", async () => {
  const client = fixture(),
    app = createApplication(client, undefined, undefined, {
      recovery: recovery(),
    });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  let replacement: Promise<void> | undefined,
    started = false;
  const off = app.subscribe((state) => {
    if (state.phase === "disconnected" && !started) {
      started = true;
      replacement = app.rebindSession({
        client: fixture(),
        principal: { ...principal, subject: "bob" },
      });
    }
  });
  await app.rebindSession({ client, principal: null });
  await replacement;
  await app.stageEditorDraft("one", {
    mode: "resource",
    descriptor: "task-v1",
    baseRevision: 1n,
    intents: {},
    editors: {},
  });
  assert.equal(app.state.session?.status, "active");
  off();
  app.disconnect();
});

test("identity invalidation precedes reentrant mutation from clearing subscribers", async () => {
  let attempts = 0;
  const client = fixture(async () => {
    attempts++;
    throw Error("unexpected dispatch");
  });
  const app = createApplication(client, undefined, undefined, {
    recovery: recovery(),
  });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  let invocation: Promise<void> | undefined;
  const off = app.subscribe((state) => {
    if (state.phase === "disconnected" && !invocation)
      invocation = app.mutate("one", 1n, {
        type: "replace",
        input: { done: true },
      });
  });
  await app.rebindSession({ client, principal: null });
  await assert.rejects(invocation!, /session|principal/i);
  assert.equal(attempts, 0);
  off();
  app.disconnect();
});

test("later local draft can be persisted while transient session blocks dispatch", async () => {
  const client = fixture(async () => {
    throw Error("lost acknowledgement");
  });
  const app = createApplication(client, undefined, undefined, {
    recovery: recovery(),
  });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  await assert.rejects(
    app.mutate("one", 1n, { type: "replace", input: { done: true } }),
  );
  app.pauseSession("transient");
  await app.stageDraft("one", { type: "replace", input: { done: false } });
  assert.equal(app.state.recovery?.state.commitKnowledge, "unknown");
  assert.equal(
    stringifyWire(app.state.recovery!.state.draft),
    '{"type":"replace","input":{"done":false}}',
  );
  app.disconnect();
});

test("paused managed authority cannot dispatch a mutation", async () => {
  let dispatched = 0;
  const client = fixture(async () => {
    dispatched++;
    throw Error("lost acknowledgement");
  });
  const app = createApplication(client, undefined, undefined, {
    recovery: recovery(),
  });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  app.pauseSession("transient");
  await assert.rejects(
    app.mutate("one", 1n, { type: "replace", input: { done: true } }),
    /session|authority/i,
  );
  assert.equal(dispatched, 0);
  app.disconnect();
});

test("managed state snapshots cannot change mutation authority", async () => {
  let attempts = 0;
  const client = fixture(async () => {
    attempts++;
    throw Error("unexpected dispatch");
  });
  const app = createApplication(client, undefined, undefined, {
    recovery: recovery(),
  });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  app.pauseSession("transient");
  app.state.session!.mutationAllowed = true;
  await assert.rejects(
    app.mutate("one", 1n, { type: "replace", input: { done: true } }),
    /session/i,
  );
  assert.equal(attempts, 0);
  app.disconnect();
});

test("editor CAS preserves invalid text and exact base revision, then refuses stale overwrite", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  const config = recovery(),
    client = fixture();
  const options = {
    recovery: config,
    binding: () => ({ client, principal }),
    publish: () => {},
  };
  const first = createEditorDrafts(options),
    stale = createEditorDrafts(options);
  const target = { kind: "task", id: "one" };
  const snapshot = {
    mode: "resource" as const,
    descriptor: "task-v1",
    baseRevision: 9007199254740993n,
    intents: { done: false },
    editors: { count: { text: "invalid number", invalid: "Enter a number." } },
  };
  await first.stage(target, snapshot);
  assert.equal(first.state.status, "saved");
  await stale.restore(target);
  await first.stage(target, {
    ...snapshot,
    editors: { count: { text: "later invalid" } },
  });
  await assert.rejects(stale.stage(target, snapshot), /storage|conflict/i);
  assert.equal(stale.state.status, "error");
  assert.throws(() => stale.guardNavigation(), /draft|storage/i);
  const reopened = createEditorDrafts(options);
  await reopened.restore(target);
  assert.equal(reopened.state.snapshot?.baseRevision, 9007199254740993n);
  assert.equal(reopened.state.snapshot?.editors.count.text, "later invalid");
});

test("held editor persistence blocks navigation and cannot republish after principal clearing", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  let release!: (result: boolean) => void;
  const config = recovery();
  config.editorStore = {
    async read() {
      return null;
    },
    compareExchange() {
      return new Promise((resolve) => {
        release = resolve;
      });
    },
  };
  let owner: DurablePrincipal | null = principal;
  const drafts = createEditorDrafts({
    recovery: config,
    binding: () => ({ client: fixture(), principal: owner }),
    publish: () => {},
  });
  const write = drafts.stage(
    { kind: "task", id: "one" },
    {
      mode: "resource",
      descriptor: "task-v1",
      baseRevision: 1n,
      intents: {},
      editors: { count: { text: "private invalid" } },
    },
  );
  await new Promise((resolve) => setImmediate(resolve));
  assert.throws(() => drafts.guardNavigation(), /draft|storage/i);
  owner = null;
  drafts.rebind(null);
  release(true);
  await assert.rejects(write, /changed|principal/i);
  assert.equal(drafts.state.snapshot, null);
  assert.equal(drafts.state.status, "idle");
});

test("controller retains selected view when editor CAS fails", async () => {
  const config = recovery();
  config.editorStore = {
    async read() {
      return null;
    },
    async compareExchange() {
      return false;
    },
  };
  const client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  await assert.rejects(
    app.stageEditorDraft("one", {
      mode: "resource",
      descriptor: "task-v1",
      baseRevision: 1n,
      intents: {},
      editors: { count: { text: "invalid" } },
    }),
    /storage|conflict/i,
  );
  await assert.rejects(app.selectRow("other"), /draft|storage/i);
  assert.equal(app.state.selected?.key.id, "one");
  app.disconnect();
});

test("queued editor writes keep navigation blocked until latest draft is durable", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  const releases: ((result: boolean) => void)[] = [],
    config = recovery();
  config.editorStore = {
    async read() {
      return null;
    },
    compareExchange() {
      return new Promise((resolve) => releases.push(resolve));
    },
  };
  let escaped = false;
  const drafts = createEditorDrafts({
    recovery: config,
    binding: () => ({ client: fixture(), principal }),
    publish(state) {
      if (
        state.status === "saved" &&
        state.snapshot?.editors.count.text === "first"
      )
        escaped = true;
    },
  });
  const target = { kind: "task", id: "one" },
    snapshot = {
      mode: "resource" as const,
      descriptor: "task-v1",
      baseRevision: 1n,
      intents: {},
      editors: { count: { text: "first" } },
    };
  const first = drafts.stage(target, snapshot),
    second = drafts.stage(target, {
      ...snapshot,
      editors: { count: { text: "latest" } },
    });
  await new Promise((resolve) => setImmediate(resolve));
  releases[0](true);
  await first;
  assert.throws(() => drafts.guardNavigation(), /draft|storage/i);
  await new Promise((resolve) => setImmediate(resolve));
  releases[1](true);
  await second;
  assert.equal(escaped, false);
  assert.equal(drafts.state.snapshot?.editors.count.text, "latest");
});

test("malformed editor records fail closed without restoring private text", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  for (const malformed of [
    { intents: [] },
    { editors: "private text" },
    { extra: "unknown metadata" },
  ]) {
    const config = recovery(),
      target = { kind: "task", id: "one" };
    const record = {
      format: "rom-editor-draft-v1",
      namespace: "fixture",
      principal,
      target,
      mode: "resource",
      descriptor: "task-v1",
      baseRevision: 1n,
      intents: {},
      editors: {},
      ...malformed,
    };
    config.editorStore = {
      async read() {
        return { version: "stored", payload: stringifyWire(record) };
      },
      async compareExchange() {
        return true;
      },
    };
    const drafts = createEditorDrafts({
      recovery: config,
      binding: () => ({ client: fixture(), principal }),
      publish: () => {},
    });
    await assert.rejects(drafts.restore(target), /invalid|record/i);
    assert.equal(drafts.state.snapshot, null);
    assert.equal(drafts.state.status, "error");
  }
});

test("wrong-owner or wrong-target editor records cannot be restored", async () => {
  const { createEditorDrafts } =
    await import("../../src/lib/application/editor-drafts.ts");
  for (const mismatch of [
    { principal: { ...principal, subject: "bob" } },
    { target: { kind: "task", id: "other" } },
  ]) {
    const config = recovery(),
      target = { kind: "task", id: "one" };
    const record = {
      format: "rom-editor-draft-v1",
      namespace: "fixture",
      principal,
      target,
      mode: "resource",
      descriptor: "task-v1",
      baseRevision: 1n,
      intents: {},
      editors: {},
      ...mismatch,
    };
    config.editorStore = {
      async read() {
        return { version: "stored", payload: stringifyWire(record) };
      },
      async compareExchange() {
        return true;
      },
    };
    const drafts = createEditorDrafts({
      recovery: config,
      binding: () => ({ client: fixture(), principal }),
      publish: () => {},
    });
    await assert.rejects(drafts.restore(target), /mismatch/i);
    assert.equal(drafts.state.snapshot, null);
  }
});

test("explicit restore cannot load replacement-owner editor records after reentrant identity change", async () => {
  const config = recovery(),
    reads: string[] = [];
  config.editorStore = {
    async read(slot) {
      reads.push(slot);
      return null;
    },
    async compareExchange() {
      return true;
    },
  };
  const client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  let changed = false,
    replacement: Promise<void> | undefined;
  const off = app.subscribe((state) => {
    if (!changed && state.recovery) {
      changed = true;
      replacement = app.rebindSession({
        client: fixture(),
        principal: { ...principal, subject: "bob" },
      });
    }
  });
  await assert.rejects(
    app.restoreSelectedIntent(),
    /changed|binding|principal/i,
  );
  await replacement;
  assert.equal(
    reads.some((slot) => slot.startsWith("bob/")),
    false,
  );
  off();
  app.disconnect();
});

test("unknown managed command blocks navigation and retries exact accepted wire after renewal", async () => {
  const bodies: string[] = [];
  const first = fixture(async (body) => {
    bodies.push(body);
    throw Error("lost acknowledgement");
  });
  const app = createApplication(first, undefined, undefined, {
    recovery: recovery(),
  });
  await app.rebindSession({ client: first, principal });
  await app.selectRow("one");
  await assert.rejects(
    app.mutate("one", 1n, {
      type: "replace",
      input: { done: true, large: 9007199254740993n },
    }),
  );
  assert.equal(app.state.recovery?.state.commitKnowledge, "unknown");
  await assert.rejects(app.selectRow("other"), /pending|resolve/i);
  assert.equal(app.state.selected?.key.id, "one");
  await app.stageDraft("one", { type: "replace", input: { done: false } });
  app.pauseSession("renewing");
  const next = fixture(async (body) => {
    bodies.push(body);
    return new Response(
      '{"key":{"kind":"task","id":"one"},"revision":2,"value":{"done":true}}',
      { headers: { "content-type": "application/json" } },
    );
  });
  await app.rebindSession({ client: next, principal });
  await app.retry();
  assert.equal(bodies.length, 2);
  assert.equal(bodies[1], bodies[0]);
  assert.equal(app.state.recovery?.state.commitKnowledge, "committed");
  assert.equal(
    stringifyWire(app.state.recovery!.state.draft),
    '{"type":"replace","input":{"done":false}}',
  );
  app.disconnect();
});

function writerSnapshot(text = "private-draft") {
  return {
    mode: "resource" as const,
    descriptor: "task-v1",
    baseRevision: 1n,
    intents: {},
    editors: { count: { text } },
  };
}
for (const change of ["principal", "clear", "denied", "target"] as const) {
  test(`bound draft writer rejects stale ${change} context before persistence`, async () => {
    let denied = false,
      target = "one",
      writes = 0;
    const config = recovery(),
      original = config.editorStore.compareExchange;
    config.editorStore.compareExchange = async (...args) => {
      writes++;
      return original(...args);
    };
    const client = fixture(undefined, async (route) =>
      route === "read"
        ? new Response(
            denied
              ? '{"error":"denied"}'
              : `{"key":{"kind":"task","id":"${target}"},"revision":1,"value":{"done":false}}`,
            {
              status: denied ? 403 : 200,
              headers: { "content-type": "application/json" },
            },
          )
        : null,
    );
    const app = createApplication(client, undefined, undefined, {
      recovery: config,
    });
    await app.rebindSession({ client, principal });
    await app.selectRow("one");
    const writer = app.createDraftWriter("one");
    if (change === "principal")
      await app.rebindSession({
        client,
        principal: { ...principal, subject: "bob" },
      });
    if (change === "clear")
      await app.rebindSession({ client, principal: null });
    if (change === "denied") {
      denied = true;
      await app.rebindSession({ client, principal });
    }
    if (change === "target") {
      target = "two";
      await app.selectRow("two");
    }
    await assert.rejects(
      writer.stage(writerSnapshot(), {
        type: "replace",
        input: { done: true },
      }),
      /owner|principal|target|denied|binding/i,
    );
    assert.equal(writes, 0);
    assert.equal(app.state.recovery?.state.draft ?? null, null);
    app.disconnect();
  });
}
test("bound draft writer remains valid after same-owner renewal and stores invalid raw text", async () => {
  const config = recovery(),
    client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  const writer = app.createDraftWriter("one");
  app.pauseSession("transient");
  await writer.stage(writerSnapshot("invalid-offline"), null);
  await app.rebindSession({ client: fixture(), principal });
  await writer.stage(writerSnapshot("invalid-renewed"), null);
  assert.equal(
    app.state.editor?.snapshot?.editors.count.text,
    "invalid-renewed",
  );
  assert.equal(app.state.recovery?.state.draft, null);
  app.disconnect();
});
test("reentrant principal change after editor CAS cannot stage old input in the new owner lane", async () => {
  const config = recovery(),
    client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  const writer = app.createDraftWriter("one");
  let replacement: Promise<void> | undefined,
    armed = true;
  const stop = app.subscribe((state) => {
    if (armed && state.editor?.status === "saved") {
      armed = false;
      replacement = app.rebindSession({
        client,
        principal: { ...principal, subject: "bob" },
      });
    }
  });
  await assert.rejects(
    writer.stage(writerSnapshot(), { type: "replace", input: { done: true } }),
    /owner|principal|binding|target/i,
  );
  await replacement;
  assert.equal(app.state.editor?.snapshot, null);
  assert.equal(app.state.recovery?.state.draft ?? null, null);
  stop();
  app.disconnect();
});

test("simultaneous bound frame updaters merge from current owner cache without losing a form", async () => {
  const { mergeFormFrames, readFormFrames } =
    await import("../../src/lib/resources/form-draft.ts");
  const definition = {
      descriptor: "parent-v1",
      resource: "task-v1",
      actions: { save: "save-v1" },
    },
    config = recovery(),
    client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  const writer = app.createDraftWriter("one");
  const resource = {
    ...writerSnapshot("invalid-resource"),
    baseRevision: 9007199254740993n,
  };
  const action = {
    ...writerSnapshot("invalid-action"),
    mode: "action" as const,
    descriptor: "save-v1",
    baseRevision: 9007199254740993n,
  };
  await Promise.all([
    writer.stage(
      (current) =>
        mergeFormFrames(current, { type: "resource" }, resource, definition),
      null,
    ),
    writer.stage(
      (current) =>
        mergeFormFrames(
          current,
          { type: "action", name: "save" },
          action,
          definition,
        ),
      null,
    ),
  ]);
  const frames = readFormFrames(app.state.editor!.snapshot!, definition);
  assert.equal(frames.resource?.editors.count.text, "invalid-resource");
  assert.equal(frames.actions.save.editors.count.text, "invalid-action");
  app.disconnect();
});
test("bound frame updater reentrancy cannot write old draft after principal replacement", async () => {
  const config = recovery(),
    client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  const writer = app.createDraftWriter("one");
  let replacement: Promise<void> | undefined;
  await assert.rejects(
    writer.stage(() => {
      replacement = app.rebindSession({
        client,
        principal: { ...principal, subject: "bob" },
      });
      return writerSnapshot();
    }, null),
    /owner|principal|binding/i,
  );
  await replacement;
  assert.equal(app.state.editor?.snapshot, null);
  assert.equal(app.state.recovery?.state.draft ?? null, null);
  app.disconnect();
});
test("failed form composition blocks navigation and does not dispatch or store partial frames", async () => {
  const config = recovery(),
    client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  const writer = app.createDraftWriter("one");
  await assert.rejects(
    writer.stage(() => {
      throw Error("frame bound exceeded");
    }, null),
    /frame bound/i,
  );
  assert.equal(app.state.editor?.status, "error");
  await assert.rejects(app.selectKind("task"), /storage|draft|resolve/i);
  assert.equal(app.state.recovery?.state.draft ?? null, null);
  app.disconnect();
});

for (const accepted of [false, true]) {
  for (const subject of ["alice", "bob"]) {
    test(`${accepted ? "accepted unknown" : "draft-only"} owner clear permits fresh ${subject} authority without automatic old-owner restoration`, async () => {
      const config = recovery(),
        bodies: string[] = [];
      const first = fixture(async (body) => {
        bodies.push(body);
        throw Error("lost acknowledgement");
      });
      const app = createApplication(first, undefined, undefined, {
        recovery: config,
      });
      await app.rebindSession({ client: first, principal });
      await app.selectRow("one");
      await app.createDraftWriter("one").stage(
        {
          mode: "resource",
          descriptor: "task-v1",
          baseRevision: 1n,
          intents: {},
          editors: { title: { text: "private-invalid" } },
        },
        { type: "replace", input: { done: true } },
      );
      if (accepted)
        await assert.rejects(
          app.mutate("one", 1n, { type: "replace", input: { done: true } }),
        );
      const slot = config.slot(
        principal,
        { kind: "task", id: "one" },
        "intent",
      );
      const original = await config.intentStore.read(slot);
      await app.rebindSession({ client: first, principal: null });
      assert.equal(app.state.selected, null);
      assert.equal(app.state.editor?.snapshot, null);
      assert.equal(
        app.state.recovery,
        null,
        "quarantined prior-owner lane is not a current private projection",
      );
      const next = fixture(async (body) => {
        bodies.push(body);
        return new Response(
          '{"key":{"kind":"task","id":"one"},"revision":2,"value":{"done":true}}',
          { headers: { "content-type": "application/json" } },
        );
      });
      await app.rebindSession({
        client: next,
        principal: { ...principal, subject },
      });
      assert.equal(app.state.session?.status, "active");
      assert.equal(app.state.rows.length, 1);
      assert.equal(bodies.length, accepted ? 1 : 0);
      await app.selectRow("one");
      assert.equal(
        app.state.editor?.snapshot,
        null,
        "fresh selection must not load durable draft implicitly",
      );
      const restoreReads: string[] = [];
      for (const storage of [config.intentStore, config.editorStore]) {
        const read = storage.read;
        storage.read = async (slot) => {
          restoreReads.push(slot);
          return read(slot);
        };
      }
      await app.restoreSelectedIntent();
      if (subject === "bob")
        assert.ok(
          restoreReads.every((slot) => slot.startsWith("bob/")),
          "other owner reads only explicitly selected own slots",
        );
      assert.equal(
        bodies.length,
        accepted ? 1 : 0,
        "explicit restore must not dispatch",
      );
      if (subject === "alice") {
        assert.equal(
          app.state.editor?.snapshot?.editors.title.text,
          "private-invalid",
        );
        if (accepted) {
          await app.retry();
          assert.equal(bodies[1], bodies[0]);
        }
      } else {
        assert.equal(app.state.editor?.snapshot, null);
        assert.equal(app.state.recovery?.state.draft, null);
        assert.equal(app.state.recovery?.state.hasUnresolvedIntent, false);
        assert.deepEqual(
          await config.intentStore.read(slot),
          original,
          "other owner never alters original durable command",
        );
      }
      app.disconnect();
    });
  }
}

test("outstanding draft CAS cannot republish old-owner bytes after clear and other-owner reacquisition", async () => {
  const config = recovery(),
    storage = config.intentStore,
    originalCas = storage.compareExchange;
  let release!: () => void, reached!: () => void;
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  const started = new Promise<void>((resolve) => {
    reached = resolve;
  });
  storage.compareExchange = async (...args) => {
    reached();
    await held;
    return originalCas(...args);
  };
  const client = fixture(),
    app = createApplication(client, undefined, undefined, { recovery: config });
  await app.rebindSession({ client, principal });
  await app.selectRow("one");
  const staged = app
    .createDraftWriter("one")
    .stage(
      {
        mode: "resource",
        descriptor: "task-v1",
        baseRevision: 1n,
        intents: {},
        editors: { title: { text: "private-pending" } },
      },
      { type: "replace", input: { done: true } },
    );
  const rejected = assert.rejects(staged, /binding|owner|changed/i);
  await started;
  await app.rebindSession({ client, principal: null });
  await app.rebindSession({
    client: fixture(),
    principal: { ...principal, subject: "bob" },
  });
  assert.equal(app.state.session?.status, "active");
  await app.selectRow("one");
  release();
  await rejected;
  assert.equal(app.state.editor?.snapshot, null);
  assert.equal(app.state.recovery, null);
  app.disconnect();
});
