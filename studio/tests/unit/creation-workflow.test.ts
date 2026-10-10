import test from "node:test";
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { createCreationWorkflow } from "../../src/lib/application/creation-workflow.ts";
import {
  fileStore,
  binding,
  principal,
} from "./mutation-recovery-support.test.ts";
import type { DurablePrincipal } from "../../src/recovery.ts";
import type { EditorDraft } from "../../src/lib/renderers/editor-draft.ts";

const form = {
  mode: "resource" as const,
  descriptor: "create-notes-v1",
  baseRevision: null,
  intents: { title: { mode: "value", value: "initial" } },
  editors: { title: { text: "initial" } },
};
const created = () =>
  new Response(
    '{"key":{"kind":"notes","id":"new-id"},"revision":1,"value":{"title":"initial"}}',
    { headers: { "content-type": "application/json" } },
  );
function fixture(stores = { intent: fileStore(), editor: fileStore() }) {
  let owner: DurablePrincipal | null = principal,
    allowed = true,
    lost = true;
  const bodies: string[] = [];
  let current = binding(async (_url, init) => {
    bodies.push(String(init?.body));
    if (lost) throw Error("acknowledgement lost");
    return created();
  });
  const workflow = createCreationWorkflow({
    kind: "notes",
    descriptor: "create-notes-v1",
    recovery: {
      namespace: "fixture",
      intentStore: stores.intent,
      editorStore: stores.editor,
      slot: () => {
        throw Error("Resource-target slot must not be used");
      },
      creationSlot: (p, kind, role) =>
        JSON.stringify(["fixture", role, p.authority, p.kind, p.subject, kind]),
      newVersion: randomUUID,
      newCommandKey: randomUUID,
      retryEpoch: () => 18446744073709551615n,
      maxBytes: 16384,
    },
    binding: () => ({ client: current.client, principal: owner }),
    allowed: () => allowed,
    publish() {},
  });
  return {
    workflow,
    stores,
    bodies,
    enable() {
      lost = false;
    },
    revoke() {
      allowed = false;
    },
    async rebind(next: DurablePrincipal | null) {
      owner = next;
      current = binding(async (_url, init) => {
        bodies.push(String(init?.body));
        return created();
      });
      await workflow.rebind({ client: current.client, principal: owner });
    },
  };
}
test("creation acknowledgement loss restores its original command without knowing the Resource ID", async () => {
  const first = fixture();
  await first.workflow.stage({ id: "new-id", form });
  await assert.rejects(
    first.workflow.submit("new-id", {
      type: "create",
      input: { title: "initial" },
    }),
  );
  assert.equal(first.workflow.state.recovery?.state.commitKnowledge, "unknown");
  assert.throws(() => first.workflow.guardNavigation(), /pending/);
  const restarted = fixture(first.stores);
  await restarted.workflow.restore();
  assert.equal(restarted.bodies.length, 0);
  assert.equal(restarted.workflow.state.recovery?.target.id, "new-id");
  restarted.enable();
  await restarted.workflow.retry();
  assert.equal(restarted.bodies[0], first.bodies[0]);
  assert.match(restarted.bodies[0], /"expected":null/);
  assert.match(restarted.bodies[0], /"retry_epoch":18446744073709551615/);
  assert.equal(restarted.workflow.state.editor.snapshot, null);
});
test("restart retry preserves later raw text even when its Resource ID and operation are unchanged", async () => {
  const first = fixture();
  await first.workflow.stage({ id: "new-id", form });
  await assert.rejects(
    first.workflow.submit("new-id", {
      type: "create",
      input: { title: "initial" },
    }),
  );
  await first.workflow.stage({
    id: "new-id",
    form: {
      ...form,
      editors: { title: { text: "initial ", invalid: "Later raw input" } },
    },
  });
  const restarted = fixture(first.stores);
  await restarted.workflow.restore();
  restarted.enable();
  await restarted.workflow.retry();
  assert.equal(
    restarted.workflow.state.editor.snapshot?.form.editors.title.text,
    "initial ",
  );
  assert.equal(restarted.bodies[0], first.bodies[0]);
});
test("a create invoked with different input must not acknowledge an unrelated editor draft", async () => {
  const first = fixture();
  await first.workflow.stage({ id: "new-id", form });
  await assert.rejects(
    first.workflow.submit("new-id", {
      type: "create",
      input: { title: "other" },
    }),
  );
  const restarted = fixture(first.stores);
  await restarted.workflow.restore();
  restarted.enable();
  await restarted.workflow.retry();
  assert.equal(
    restarted.workflow.state.editor.snapshot?.form.editors.title.text,
    "initial",
  );
});
for (const [name, editor] of Object.entries<EditorDraft>({
  child: {
    children: { a: { text: "invalid-child", invalid: "Invalid input" } },
  },
  errors: { errors: { a: "Invalid input" } },
  row: {
    rows: [
      {
        id: "row",
        identity: "item",
        value: "initial",
        error: "Invalid input",
        draft: { text: "invalid-row" },
      },
    ],
  },
}))
  test(`confirmation preserves ${name} invalid editor data on a direct create`, async () => {
    const first = fixture();
    const nested = { ...form, editors: { title: editor } };
    await first.workflow.stage({ id: "new-id", form: nested });
    const expected = first.workflow.state.editor.snapshot!.form.editors.title;
    await assert.rejects(
      first.workflow.submit("new-id", {
        type: "create",
        input: { title: "initial" },
      }),
    );
    const restarted = fixture(first.stores);
    await restarted.workflow.restore();
    restarted.enable();
    await restarted.workflow.retry();
    assert.deepEqual(
      restarted.workflow.state.editor.snapshot?.form.editors.title,
      expected,
    );
  });
test("later invalid creation edits survive confirmation of the earlier accepted command", async () => {
  const f = fixture();
  await f.workflow.stage({ id: "new-id", form });
  await assert.rejects(
    f.workflow.submit("new-id", {
      type: "create",
      input: { title: "initial" },
    }),
  );
  await f.workflow.stage({
    id: "different-invalid-id",
    form: {
      ...form,
      editors: { title: { text: "later", invalid: "Invalid" } },
    },
  });
  f.enable();
  await f.workflow.retry();
  assert.equal(f.workflow.state.editor.snapshot?.id, "different-invalid-id");
  assert.equal(
    f.workflow.state.editor.snapshot?.form.editors.title.text,
    "later",
  );
});
test("unknown creation prohibits a replacement operation and current permission gates retry", async () => {
  const f = fixture();
  await f.workflow.stage({ id: "new-id", form });
  await assert.rejects(
    f.workflow.submit("new-id", {
      type: "create",
      input: { title: "initial" },
    }),
  );
  await assert.rejects(
    f.workflow.submit("other", { type: "create", input: {} }),
    /pending/,
  );
  f.revoke();
  await assert.rejects(f.workflow.retry(), /authority/);
  assert.equal(f.bodies.length, 1);
});
test("another owner cannot recover or disclose the previous creation command", async () => {
  const f = fixture();
  await f.workflow.stage({ id: "new-id", form });
  await assert.rejects(
    f.workflow.submit("new-id", {
      type: "create",
      input: { title: "initial" },
    }),
  );
  await f.rebind({ ...principal, subject: "bob" });
  assert.equal(f.workflow.state.editor.snapshot, null);
  assert.equal(f.workflow.state.recovery, null);
  await assert.rejects(f.workflow.restore());
  assert.equal(f.bodies.length, 1);
});
