import test from "node:test";
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { createApplication } from "../../src/lib/application/controller.ts";
import { createClient } from "../../src/client.ts";
import { fileStore, principal } from "./mutation-recovery-support.test.ts";
import { resourceDraftIdentity } from "../../src/lib/resources/form-draft.ts";
const descriptor = {
  kind: "notes",
  version: 1,
  fields: [],
  actions: [],
  action_inputs: [],
};
test("managed application routes create through stable recovery and guards navigation until exact retry", async () => {
  const bodies: string[] = [];
  let lose = true;
  const client = createClient({
    base: "http://fixture/api",
    fetch: async (url, init) => {
      if (String(url).endsWith("discover"))
        return Response.json({ version: 1, resources: [descriptor] });
      if (String(url).endsWith("query")) return Response.json([]);
      bodies.push(String(init?.body));
      if (lose) throw Error("ack lost");
      return Response.json({
        key: { kind: "notes", id: "new" },
        revision: 1,
        value: {},
      });
    },
  });
  const app = createApplication(client, randomUUID, undefined, {
    recovery: {
      namespace: "fixture",
      intentStore: fileStore(),
      editorStore: fileStore(),
      maxBytes: 16384,
      slot: () => "selected-row",
      creationSlot: (p, k, r) =>
        JSON.stringify(["fixture", r, p.authority, p.kind, p.subject, k]),
      newVersion: randomUUID,
      newCommandKey: randomUUID,
      retryEpoch: () => 1n,
    },
  });
  await app.rebindSession({ client, principal });
  const { resourceDraftIdentity } =
    await import("../../src/lib/resources/form-draft.ts");
  await app.createCreationWriter().stage({
    id: "new",
    form: {
      mode: "resource",
      descriptor: resourceDraftIdentity(descriptor, "create"),
      baseRevision: null,
      intents: {},
      editors: {},
    },
  });
  await assert.rejects(app.mutate("new", null, { type: "create", input: {} }));
  assert.equal(app.state.creation?.recovery?.target.id, "new");
  await assert.rejects(app.selectKind("notes"), /pending/);
  lose = false;
  await app.retryCreation();
  assert.equal(bodies.length, 2);
  assert.equal(bodies[0], bodies[1]);
  assert.equal(app.state.selected?.key.id, "new");
});
function renewingCreation() {
  let version = 1;
  const client = createClient({
    base: "http://fixture/api",
    fetch: async (url) =>
      String(url).endsWith("discover")
        ? Response.json({ version: 1, resources: [{ ...descriptor, version }] })
        : Response.json([]),
  });
  const app = createApplication(client, randomUUID, undefined, {
    recovery: {
      namespace: "renewal",
      intentStore: fileStore(),
      editorStore: fileStore(),
      maxBytes: 16384,
      slot: () => "row",
      creationSlot: (p, k, r) =>
        JSON.stringify([p.authority, p.kind, p.subject, k, r]),
      newVersion: randomUUID,
      newCommandKey: randomUUID,
      retryEpoch: () => 1n,
    },
  });
  const draft = (revision: number) => ({
    id: "new",
    form: {
      mode: "resource" as const,
      descriptor: resourceDraftIdentity(
        { ...descriptor, version: revision },
        "create",
      ),
      baseRevision: null,
      intents: {},
      editors: {},
    },
  });
  return {
    app,
    client,
    draft,
    async renew() {
      version = 2;
      await app.rebindSession({ client, principal });
    },
  };
}
test("same-kind descriptor renewal creates a current writer when there is no retained draft", async () => {
  const f = renewingCreation();
  await f.app.rebindSession({ client: f.client, principal });
  f.app.createCreationWriter();
  await f.renew();
  await f.app.createCreationWriter().stage(f.draft(2));
  assert.equal(
    f.app.state.creation?.editor.snapshot?.form.descriptor,
    f.draft(2).form.descriptor,
  );
});
test("descriptor renewal preserves the old draft until explicit discard permits current editing", async () => {
  const f = renewingCreation();
  await f.app.rebindSession({ client: f.client, principal });
  const old = f.app.createCreationWriter();
  await old.stage(f.draft(1));
  await f.renew();
  const current = f.app.createCreationWriter();
  await assert.rejects(current.stage(f.draft(2)), /definition|identity/i);
  assert.equal(
    f.app.state.creation?.editor.snapshot?.form.descriptor,
    f.draft(1).form.descriptor,
  );
  await f.app.discardCreation();
  await current.stage(f.draft(2));
  assert.equal(
    f.app.state.creation?.editor.snapshot?.form.descriptor,
    f.draft(2).form.descriptor,
  );
  await assert.rejects(old.stage(f.draft(1)), /definition|identity|changed/i);
});
