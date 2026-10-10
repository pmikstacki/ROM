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

for (const scenario of [
  { category: "outcome_unknown", status: 503, retain: true },
  { category: "denied", status: 403, retain: false },
]) {
  test(`upload ${scenario.category} preserves the correct recovery state`, async () => {
    const { RemoteError } = await import("../../src/lib/client/client.ts");
    let reserved = 0;
    const files: Blob[] = [];
    const client: AttachmentClient = {
      generation: 0,
      async reserveBlob() {
        reserved++;
        return view;
      },
      async uploadBlob(_id, file) {
        files.push(file);
        if (files.length === 1)
          throw new RemoteError(scenario.category, scenario.status);
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
      () => "frozen-upload-key",
      async () => "a".repeat(64),
    );
    const file = new Blob(["exact bytes"]);
    await controller.upload("file", "one", file);
    assert.equal(reserved, 1);
    assert.equal(files.length, 1, "no automatic retry");
    assert.equal(controller.state.phase, scenario.retain ? "unknown" : "error");
    const pending = controller.state.pending;
    if (scenario.retain) {
      assert(pending && pending.step === "upload");
      assert.equal(pending.file, files[0]);
      assert.equal(await pending.file.text(), await file.text());
      assert.equal(pending.reservation.idempotency, "frozen-upload-key");
      await controller.retry();
      assert.equal(reserved, 1, "retry must not reserve again");
      assert.equal(files.length, 2);
      assert.equal(files[1], files[0], "retry uses the frozen upload body");
      assert.equal(controller.state.phase, "success");
    } else {
      assert.equal(pending, null);
    }
    controller.dispose();
  });
}

for (const operation of ["reserve", "upload", "detach"] as const) {
  for (const rejection of [
    { category: "denied", status: 403 },
    { category: "not_committed", status: 503 },
  ]) {
    test(`${operation}: retry ${rejection.category} cannot resolve an earlier unknown outcome`, async () => {
      const { RemoteError } = await import("../../src/lib/client/client.ts");
      let attempts = 0;
      const reservations: BlobReservation[] = [];
      const files: Blob[] = [];
      const attempt = async () => {
        attempts++;
        if (attempts === 1) throw new RemoteError("outcome_unknown", 503);
        if (attempts === 2)
          throw new RemoteError(rejection.category, rejection.status);
        return view;
      };
      const client: AttachmentClient = {
        generation: 0,
        async reserveBlob(reservation) {
          reservations.push(reservation);
          return operation === "reserve" ? attempt() : view;
        },
        async uploadBlob(_id, file) {
          files.push(file);
          return operation === "upload" ? attempt() : view;
        },
        async detachBlob() { return attempt(); },
        async downloadBlob() { return new Uint8Array(); },
        async query() { return [view]; },
      };
      const controller = createAttachments(client, cap, () => "same-key", async () => "a".repeat(64));
      try {
        await controller.refresh();
        if (operation === "detach") await controller.detach("file");
        else await controller.upload("file", "one", new Blob(["exact"]));
        const pending = controller.state.pending;
        assert(pending);
        assert.equal(controller.state.phase, "unknown");
        await controller.retry();
        assert.equal(attempts, 2, "only explicit retries dispatch work");
        assert.deepEqual(controller.state.rows, [], "current refusal clears disclosed rows");
        assert.equal(controller.state.result, null);
        assert.equal(controller.state.pending, pending, "keep exact unresolved operation");
        assert.equal(controller.state.phase, "unknown");
        await assert.rejects(controller.detach("other"), /Resolve the current/);
        await controller.retry();
        assert.equal(attempts, 3);
        assert.equal(controller.state.phase, "success");
        assert.equal(controller.state.pending, null);
        if (operation === "reserve") {
          assert.equal(reservations.length, 3);
          assert(reservations.every((item) => item === reservations[0]));
        } else if (operation === "upload") {
          assert.equal(reservations.length, 1, "do not reserve again");
          assert.equal(files.length, 3);
          assert(files.every((file) => file === files[0]));
        }
      } finally { controller.dispose(); }
    });
  }
}
