import { test } from "node:test";
import assert from "node:assert/strict";
import { createApplication } from "../../src/lib/application/controller.ts";
import { createClient } from "../../src/lib/client/client.ts";
import { parseWire, stringifyWire } from "../../src/lib/client/codec.ts";
import type {
  WireValue,
  ResourceDescriptor,
} from "../../src/lib/client/types.ts";

const people: ResourceDescriptor = {
  kind: "people",
  version: 1,
  fields: [{ name: "name", shape: { type: "string" } }],
  actions: [],
  action_inputs: [],
  presentation: { label: "Person", title_field: "name" },
};
const tasks: ResourceDescriptor = {
  kind: "tasks",
  version: 1,
  fields: [],
  actions: [],
  action_inputs: [],
};
function row(id: string, name: string) {
  return {
    key: { kind: "people", id },
    revision: 1n,
    value: { name, hidden: "withheld text" },
  };
}
function response(value: WireValue) {
  return new Response(stringifyWire(value), {
    headers: { "Content-Type": "application/json" },
  });
}
function fixture(
  answer: (
    kind: string,
    query: Record<string, WireValue>,
    signal: AbortSignal | null | undefined,
  ) => Promise<Response> = async (kind) =>
    response(kind === "people" ? [row(" id with spaces ", "Ada")] : []),
  descriptor = people,
) {
  const calls: { kind: string; query: Record<string, WireValue> }[] = [];
  const client = createClient({
    base: "https://studio.invalid/api",
    fetch: async (url, init) => {
      if (String(url).endsWith("/discover"))
        return response({
          version: 1,
          resources: [tasks, descriptor],
        } as unknown as WireValue);
      const body = parseWire(String(init?.body)) as Record<string, WireValue>;
      const kind = body.kind as string,
        query = body.query as Record<string, WireValue>;
      calls.push({ kind, query });
      return answer(kind, query, init?.signal);
    },
  });
  return { app: createApplication(client), client, calls };
}
test("reference lookup bounds authorized candidates, preserves exact IDs and leaves navigation untouched", async () => {
  const { app, calls } = fixture();
  await app.connect();
  const state = app.state;
  const result = await app.lookupResources("people", "Ada");
  assert.deepEqual(result, {
    status: "ready",
    candidates: [{ id: " id with spaces ", title: "Ada" }],
    limited: false,
  });
  assert.equal(app.state, state);
  assert.equal(calls.at(-1)?.query.limit, 20);
  assert.equal(calls.at(-1)?.kind, "people");
  assert.equal(app.state.kind, "tasks");
});
test("hidden kinds and disconnected sessions perform no reference request", async () => {
  const { app, calls } = fixture();
  await app.connect();
  const before = calls.length;
  assert.equal((await app.lookupResources("hidden")).status, "unavailable");
  assert.equal(calls.length, before);
  app.disconnect();
  assert.equal((await app.lookupResources("people")).status, "unavailable");
  assert.equal(calls.length, before);
});
test("candidate titles cannot read a withheld title field", async () => {
  const { app } = fixture(undefined, {
    ...people,
    fields: [],
    presentation: { label: "Person" },
  });
  await app.connect();
  const result = await app.lookupResources("people");
  assert.deepEqual(result, {
    status: "ready",
    candidates: [{ id: " id with spaces ", title: "Person  id with spaces " }],
    limited: false,
  });
});
test("denied lookup is distinct from no matching authorized candidates", async () => {
  const { app } = fixture(async (kind) =>
    kind === "people"
      ? new Response('{"error":"denied"}', {
          status: 403,
          headers: { "Content-Type": "application/json" },
        })
      : response([]),
  );
  await app.connect();
  const result = await app.lookupResources("people");
  assert.equal(result.status, "denied");
  const empty = fixture();
  await empty.app.connect();
  assert.deepEqual(await empty.app.lookupResources("people", "nobody"), {
    status: "ready",
    candidates: [],
    limited: false,
  });
});
test("session changes and superseded requests discard delayed reference replies", async () => {
  let release!: (response: Response) => void;
  const { app, client } = fixture(async (kind) =>
    kind === "people"
      ? new Promise<Response>((resolve) => {
          release = resolve;
        })
      : response([]),
  );
  await app.connect();
  const controller = new AbortController();
  const pending = app.lookupResources("people", "", controller.signal);
  controller.abort();
  release(response([row("old", "Stale person")]));
  assert.equal((await pending).status, "cancelled");
  const again = app.lookupResources("people");
  client.invalidateSession();
  release(response([row("old", "Stale person")]));
  assert.equal((await again).status, "cancelled");
});
test("reference candidate admission enforces bytes and row limits", async () => {
  for (const rows of [
    [row("large", "é".repeat(40000))],
    Array.from({ length: 21 }, (_, i) => row(String(i), "Name")),
  ]) {
    const { app } = fixture(async (kind) =>
      response(kind === "people" ? rows : []),
    );
    await app.connect();
    assert.equal((await app.lookupResources("people")).status, "error");
  }
});
test("a full bounded page is marked limited without traversing subsequent pages", async () => {
  const { app, calls } = fixture(async (kind) =>
    response(
      kind === "people"
        ? Array.from({ length: 20 }, (_, i) => row(String(i), "Name"))
        : [],
    ),
  );
  await app.connect();
  const result = await app.lookupResources("people", "missing");
  assert.deepEqual(result, { status: "ready", candidates: [], limited: true });
  assert.equal(calls.length, 2);
});

test("lookup bounds simultaneous requests and rejects oversized search without fetching", async () => {
  const releases: ((response: Response) => void)[] = [];
  const { app, calls } = fixture(async (kind) =>
    kind === "people"
      ? new Promise<Response>((resolve) => releases.push(resolve))
      : response([]),
  );
  await app.connect();
  const before = calls.length;
  assert.equal(
    (await app.lookupResources("people", "é".repeat(513))).status,
    "error",
  );
  assert.equal(calls.length, before);
  const pending = Array.from({ length: 4 }, () =>
    app.lookupResources("people"),
  );
  assert.equal((await app.lookupResources("people")).status, "error");
  assert.equal(calls.length, before + 4);
  releases.forEach((release) => release(response([row("one", "Name")])));
  assert.ok(
    (await Promise.all(pending)).every((result) => result.status === "ready"),
  );
});

test("already cancelled lookup and tombstone candidates do not disclose selectable results", async () => {
  const { app, calls } = fixture(async (kind) =>
    response(
      kind === "people"
        ? [
            {
              key: { kind: "people", id: "deleted" },
              revision: 1n,
              value: null,
            },
          ]
        : [],
    ),
  );
  await app.connect();
  const before = calls.length;
  assert.equal(
    (await app.lookupResources("people", "", AbortSignal.abort())).status,
    "cancelled",
  );
  assert.equal(calls.length, before);
  assert.deepEqual(await app.lookupResources("people"), {
    status: "ready",
    candidates: [],
    limited: false,
  });
});
