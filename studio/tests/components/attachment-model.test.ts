import { test } from "node:test";
import assert from "node:assert/strict";
import {
  createAttachments,
  type AttachmentClient,
} from "../../src/lib/attachments/controller.ts";
import type {
  BlobCapabilities,
  ProjectedView,
  BlobReservation,
} from "../../src/lib/client/types.ts";
const cap: BlobCapabilities = {
  version: 1,
  resource_kind: "fixture-files",
  stores: ["one"],
  limits: { blob_bytes: 16, chunk_bytes: 8, chunks: 2 },
  operations: ["reserve", "upload", "download", "detach"],
};
const view: ProjectedView = {
  key: { kind: "fixture-files", id: "file" },
  revision: 2n,
  value: { state: "ready" },
};
test("unknown reserve repeats the frozen key/file then upload without Resource kind switches", async () => {
  const reservations: BlobReservation[] = [];
  const contents: string[] = [];
  let calls = 0;
  const client: AttachmentClient = {
    generation: 0,
    async reserveBlob(request) {
      reservations.push(request);
      if (calls++ === 0) throw Error("lost acknowledgment");
      return view;
    },
    async uploadBlob(_id, file) {
      contents.push(await file.text());
      return view;
    },
    async detachBlob() {
      return view;
    },
    async downloadBlob() {
      return new Uint8Array();
    },
    async query() {
      return [view];
    },
  };
  const controller = createAttachments(
    client,
    cap,
    () => "same-key",
    async () => "a".repeat(64),
  );
  await controller.upload("file", "one", new Blob(["initial"]));
  assert.equal(controller.state.phase, "unknown");
  await controller.retry();
  assert.equal(controller.state.phase, "success");
  assert.deepEqual(reservations[0], reservations[1]);
  assert.equal(reservations[1].idempotency, "same-key");
  assert.deepEqual(contents, ["initial"]);
});
test("unknown upload retries publication only and disposal clears retained content", async () => {
  let reserved = 0,
    uploaded = 0;
  const client: AttachmentClient = {
    generation: 0,
    async reserveBlob() {
      reserved++;
      return view;
    },
    async uploadBlob() {
      if (uploaded++ === 0) throw Error("lost upload response");
      return view;
    },
    async detachBlob() {
      return view;
    },
    async downloadBlob() {
      return new Uint8Array();
    },
    async query() {
      return [view];
    },
  };
  const controller = createAttachments(
    client,
    cap,
    () => "key",
    async () => "a".repeat(64),
  );
  await controller.upload("file", "one", new Blob(["content"]));
  assert.equal(controller.state.phase, "unknown");
  await controller.retry();
  assert.equal(reserved, 1);
  assert.equal(uploaded, 2);
  assert.equal(controller.state.phase, "success");
  controller.dispose();
  assert.equal(controller.state.pending, null);
  assert.equal(controller.state.result, null);
});
test("byte limit rejects before mutation", async () => {
  let called = 0;
  const client: AttachmentClient = {
    generation: 0,
    async reserveBlob() {
      called++;
      return view;
    },
    async uploadBlob() {
      return view;
    },
    async detachBlob() {
      return view;
    },
    async downloadBlob() {
      return new Uint8Array();
    },
    async query() {
      return [view];
    },
  };
  const controller = createAttachments(
    client,
    cap,
    () => "key",
    async () => "a".repeat(64),
  );
  await controller.upload("file", "one", new Blob(["x".repeat(17)]));
  assert.equal(called, 0);
  assert.equal(controller.state.phase, "error");
});

test("a result from the old session generation never populates the new view", async () => {
  let generation = 0;
  let release: (rows: ProjectedView[]) => void = () => {};
  const waiting = new Promise<ProjectedView[]>(
    (resolve) => (release = resolve),
  );
  const client: AttachmentClient = {
    get generation() {
      return generation;
    },
    async reserveBlob() {
      return view;
    },
    async uploadBlob() {
      return view;
    },
    async detachBlob() {
      return view;
    },
    async downloadBlob() {
      return new Uint8Array();
    },
    async query() {
      return waiting;
    },
  };
  const controller = createAttachments(client, cap);
  const refreshing = controller.refresh();
  generation++;
  release([view]);
  await refreshing;
  assert.deepEqual(controller.state.rows, []);
});
test("disposal denies a late completed download", async () => {
  let resolve = (_bytes: Uint8Array) => {};
  const result = new Promise<Uint8Array>((done) => {
    resolve = done;
  });
  const client: AttachmentClient = {
    generation: 0,
    async reserveBlob() {
      return view;
    },
    async uploadBlob() {
      return view;
    },
    async detachBlob() {
      return view;
    },
    async query() {
      return [view];
    },
    async downloadBlob() {
      return result;
    },
  };
  const controller = createAttachments(client, cap);
  const downloading = controller.download("file");
  controller.dispose();
  resolve(new Uint8Array([1]));
  await assert.rejects(downloading, /no longer active/);
});
test("HTTP 500 retains the exact pending reservation until explicit retry", async () => {
  const { RemoteError } = await import("../../src/lib/client/client.ts");
  const reservations: BlobReservation[] = [];
  const client: AttachmentClient = {
    generation: 0,
    async reserveBlob(request) {
      reservations.push(request);
      if (reservations.length === 1) throw new RemoteError("internal", 500);
      return view;
    },
    async uploadBlob() {
      return view;
    },
    async detachBlob() {
      return view;
    },
    async query() {
      return [view];
    },
    async downloadBlob() {
      return new Uint8Array();
    },
  };
  const controller = createAttachments(
    client,
    cap,
    () => "frozen-key",
    async () => "a".repeat(64),
  );
  await controller.upload("file", "one", new Blob(["exact"]));
  assert.equal(controller.state.phase, "unknown");
  assert.notEqual(controller.state.pending, null);
  await controller.retry();
  assert.deepEqual(reservations[0], reservations[1]);
});
