import { test } from "node:test";
import assert from "node:assert/strict";
import { createClient } from "../../src/lib/client/client.ts";

const capabilities = {
  version: 1,
  resource_kind: "blobs",
  stores: ["folder"],
  limits: { blob_bytes: 1024, chunk_bytes: 16, chunks: 64 },
  operations: ["reserve", "upload", "download", "detach"],
};
const envelope = (status = "reserved", id = "a") => ({
  status,
  resource: {
    key: { kind: "blobs", id },
    revision: 1,
    value: { state: "reserved" },
  },
});
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });

test("blob capabilities and reserve bind the admitted kind, ID and exact integer body", async () => {
  const calls: { url: string; body: string }[] = [];
  const client = createClient({
    base: "/rom-studio/api",
    csrf: () => "token",
    fetch: async (url, init) => {
      calls.push({ url: String(url), body: String(init?.body ?? "") });
      if (String(url).endsWith("capabilities")) return json(capabilities);
      assert.equal(new Headers(init?.headers).get("x-rom-csrf"), "token");
      return json(envelope());
    },
  });
  assert.deepEqual(await client.blobCapabilities(), capabilities);
  assert.equal(
    (
      await client.reserveBlob({
        id: "a",
        store: "folder",
        digest: "a".repeat(64),
        bytes: 3n,
        idempotency: "stable",
      })
    ).key.id,
    "a",
  );
  assert.equal(calls[0].url, "/rom-studio/blobs/capabilities");
  assert.match(calls[1].body, /"bytes":3/);
});

test("blob uploads use immutable bytes, same-origin CSRF and bound projected responses", async () => {
  let wrong = false;
  const client = createClient({
    base: "/api",
    csrf: () => "token",
    fetch: async (url, init) => {
      if (String(url).endsWith("capabilities")) return json(capabilities);
      assert.equal(String(url), "/blobs/upload?id=a%2Fb");
      assert.equal(
        new Headers(init?.headers).get("content-type"),
        "application/octet-stream",
      );
      assert.equal(new Headers(init?.headers).get("x-rom-csrf"), "token");
      assert.equal(init?.credentials, "same-origin");
      assert.equal(await (init?.body as Blob).text(), "abc");
      return json(envelope("attached", wrong ? "other" : "a/b"));
    },
  });
  await client.blobCapabilities();
  assert.equal(
    (await client.uploadBlob("a/b", new Blob(["abc"]))).key.id,
    "a/b",
  );
  wrong = true;
  await assert.rejects(client.uploadBlob("a/b", new Blob(["abc"])), /identity/);
});

test("blob capability limits, operation and response status admission fail closed", async () => {
  let result: unknown = {
    ...capabilities,
    limits: { ...capabilities.limits, chunk_bytes: 1.5 },
  };
  const client = createClient({
    base: "/api",
    fetch: async () => json(result),
  });
  await assert.rejects(client.blobCapabilities(), /integer/);
  result = capabilities;
  await client.blobCapabilities();
  await assert.rejects(
    client.uploadBlob("a", new Blob([new Uint8Array(1025)])),
    /limit/,
  );
  result = envelope("detached");
  await assert.rejects(
    client.reserveBlob({
      id: "a",
      store: "folder",
      digest: "a".repeat(64),
      bytes: 3n,
      idempotency: "stable",
    }),
    /status/,
  );
});

test("blob generation and finite deadline protect binary requests and explicit recovery", async () => {
  let mode = "capabilities";
  const client = createClient({
    base: "/api",
    timeoutMs: 10,
    fetch: async () =>
      mode === "capabilities" ? json(capabilities) : new Promise(() => {}),
  });
  await client.blobCapabilities();
  mode = "upload";
  await assert.rejects(client.uploadBlob("a", new Blob(["a"])), /timeout/);
  client.invalidateSession();
  await assert.rejects(client.uploadBlob("a", new Blob(["a"])), /capabilit/);
});

test("download enforces admitted size and binary type without JSON decoding", async () => {
  let size = 3;
  const client = createClient({
    base: "/api",
    fetch: async (url) => {
      if (String(url).endsWith("capabilities")) return json(capabilities);
      assert.equal(String(url), "/blobs/attachment?id=a%2Fb");
      return new Response(new Uint8Array(size), {
        headers: { "content-type": "application/octet-stream" },
      });
    },
  });
  await client.blobCapabilities();
  assert.equal((await client.downloadBlob("a/b")).byteLength, 3);
  size = 1025;
  await assert.rejects(client.downloadBlob("a/b"), /limit/);
});

test("download IDs cannot normalize into another route and rejected headers cancel unused bytes", async () => {
  let cancelled = 0,
    malformed = false;
  const client = createClient({
    base: "/rom-studio/api",
    fetch: async (url) => {
      if (String(url).endsWith("capabilities")) return json(capabilities);
      assert.equal(String(url), "/rom-studio/blobs/attachment?id=..");
      const resolved = new URL(String(url), "https://example.test");
      assert.equal(resolved.pathname, "/rom-studio/blobs/attachment");
      assert.equal(resolved.searchParams.get("id"), "..");
      return new Response(
        new ReadableStream({
          cancel() {
            cancelled++;
          },
        }),
        {
          headers: malformed
            ? { "content-type": "text/html" }
            : {
                "content-type": "application/octet-stream",
                "content-length": "1025",
              },
        },
      );
    },
  });
  await client.blobCapabilities();
  await assert.rejects(client.downloadBlob(".."));
  assert.equal(cancelled, 1);
  malformed = true;
  await assert.rejects(client.downloadBlob(".."), /content type/);
  assert.equal(cancelled, 2);
});

test("returned capabilities cannot widen private admission and reserve input is frozen before waiting", async () => {
  let started!: () => void, finish!: (response: Response) => void;
  const submitted = new Promise<void>((resolve) => {
    started = resolve;
  });
  let body = "";
  const client = createClient({
    base: "/api",
    fetch: async (url, init) => {
      if (String(url).endsWith("capabilities")) return json(capabilities);
      body = String(init?.body);
      started();
      return new Promise((resolve) => {
        finish = resolve;
      });
    },
  });
  const exposed = await client.blobCapabilities();
  exposed.stores.push("secret");
  exposed.limits.blob_bytes = 2000;
  await assert.rejects(
    client.reserveBlob({
      id: "a",
      store: "secret",
      digest: "a".repeat(64),
      bytes: 1n,
      idempotency: "key",
    }),
    /store/,
  );
  await assert.rejects(
    client.uploadBlob("a", new Blob([new Uint8Array(1025)])),
    /limit/,
  );
  const reservation = {
    id: "a",
    store: "folder",
    digest: "a".repeat(64),
    bytes: 3n,
    idempotency: "stable",
  };
  const pending = client.reserveBlob(reservation);
  await submitted;
  reservation.id = "other";
  reservation.bytes = 100n;
  reservation.idempotency = "changed";
  finish(json(envelope()));
  assert.equal((await pending).key.id, "a");
  assert.match(body, /"idempotency":"stable"/);
  assert.match(body, /"bytes":3/);
});

test("binary request pre-abort reaches no transport and failed capability refresh closes admission", async () => {
  let count = 0,
    denied = false;
  const client = createClient({
    base: "/api",
    fetch: async () => {
      count++;
      return denied ? json({ error: "denied" }, 403) : json(capabilities);
    },
  });
  await client.blobCapabilities();
  const abort = new AbortController();
  abort.abort(Error("caller stopped"));
  await assert.rejects(
    client.uploadBlob("a", new Blob(["x"]), abort.signal),
    /stopped/,
  );
  assert.equal(count, 1);
  denied = true;
  await assert.rejects(client.blobCapabilities(), /denied/);
  await assert.rejects(client.detachBlob("a"), /capabilit/);
  assert.equal(count, 2);
});

test("capability completion cannot stamp an old response with a new session generation", async () => {
  for (let steps = 0; steps < 40; steps++) {
    let calls = 0;
    const client = createClient({
      base: "/api",
      fetch: async () => {
        calls++;
        return json(capabilities);
      },
    });
    const pending = client.blobCapabilities().catch(() => undefined);
    let tick = Promise.resolve();
    for (let step = 0; step < steps; step++) tick = tick.then(() => {});
    const invalidation = tick.then(() => client.invalidateSession());
    await pending;
    await invalidation;
    await assert.rejects(
      client.reserveBlob({
        id: "a",
        store: "folder",
        digest: "a".repeat(64),
        bytes: 1n,
        idempotency: "key",
      }),
      /capabilit/,
    );
    assert.equal(calls, 1, `stale capability admission at microtask ${steps}`);
  }
});

test("a superseded capability request cannot reopen admission after a newer denial", async () => {
  let finish!: (response: Response) => void,
    calls = 0;
  const client = createClient({
    base: "/api",
    fetch: async () => {
      calls++;
      if (calls === 1)
        return new Promise((resolve) => {
          finish = resolve;
        });
      return json({ error: "denied" }, 403);
    },
  });
  const original = client.blobCapabilities();
  await assert.rejects(client.blobCapabilities(), /denied/);
  finish(json(capabilities));
  await assert.rejects(original, /superseded/);
  await assert.rejects(
    client.reserveBlob({
      id: "a",
      store: "folder",
      digest: "a".repeat(64),
      bytes: 1n,
      idempotency: "key",
    }),
    /capabilit/,
  );
  assert.equal(calls, 2);
});
