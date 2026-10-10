import test from "node:test";
import assert from "node:assert/strict";
import {
  createBrowserAuth,
  SessionExpiredError,
} from "../../src/lib/application/auth.ts";
import { createBrowserSessionTransport } from "../../src/lib/auth/browser-transport.ts";
import { SessionDeniedError } from "../../src/auth.ts";

const reply = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
const session = (generation = "one") => ({
  authenticated: true,
  generation,
  csrf_token: "fixture-secret",
  user_id: "alice",
  expires_at: 200,
});

for (const status of [401, 403]) {
  test(`legacy auth exposes confirmed ${status} denial separately from expiry and outage`, async () => {
    let revoked = false;
    const auth = createBrowserAuth("/mount/", async () =>
      revoked
        ? reply({ error: "denied" }, status)
        : reply({
            ...session(),
            expires_at: Math.floor(Date.now() / 1000) + 300,
          }),
    );
    await auth.refresh();
    assert.equal(auth.csrf(), "fixture-secret");
    revoked = true;
    await assert.rejects(auth.refresh(), (error: unknown) => {
      assert.ok(error instanceof Error);
      assert.equal(error.name, "SessionDeniedError");
      assert.ok(error instanceof SessionDeniedError);
      assert.equal(error instanceof SessionExpiredError, false);
      return true;
    });
    assert.equal(auth.csrf(), undefined);
  });
}

test("legacy auth keeps temporary 503 distinct from confirmed authority loss", async () => {
  let unavailable = false;
  const auth = createBrowserAuth("/mount/", async () =>
    unavailable
      ? reply({ error: "overloaded" }, 503)
      : reply({
          ...session(),
          expires_at: Math.floor(Date.now() / 1000) + 300,
        }),
  );
  await auth.refresh();
  unavailable = true;
  await assert.rejects(auth.refresh(), (error: unknown) => {
    assert.ok(error instanceof Error);
    assert.equal(error.name, "Error");
    assert.equal(error instanceof SessionExpiredError, false);
    return true;
  });
  assert.equal(auth.csrf(), "fixture-secret");
});
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

test("legacy refresh returns the exact validated anonymous generation", async () => {
  const auth = createBrowserAuth("/mount/", async () =>
    reply({ authenticated: false, generation: "anonymous-from-server" }),
  );
  assert.deepEqual(await auth.refresh(), {
    authenticated: false,
    generation: "anonymous-from-server",
  });
  assert.equal(auth.csrf(), undefined);
});

test("shared transport preserves expired versus anonymous outcomes without inventing generation", async () => {
  const transport = createBrowserSessionTransport({
    base: "/mount/",
    now: () => 100,
    fetch: async () => reply({ ...session(), expires_at: 1 }),
  });
  assert.deepEqual(await transport.refresh(new AbortController().signal), {
    status: "expired",
  });
  assert.equal(transport.csrf(), undefined);
});

test("one cancelled waiter cannot cancel another waiter's shared session acquisition", async () => {
  const held = deferred<Response>();
  let calls = 0,
    driverSignal: AbortSignal | undefined;
  const transport = createBrowserSessionTransport({
    base: "/mount/",
    now: () => 100,
    fetch: async (_url, init) => {
      calls++;
      driverSignal = init?.signal ?? undefined;
      return held.promise;
    },
  });
  const first = new AbortController(),
    second = new AbortController();
  const old = transport.refresh(first.signal),
    kept = transport.refresh(second.signal);
  first.abort();
  assert.deepEqual(await old, { status: "changed" });
  assert.equal(driverSignal?.aborted, false);
  held.resolve(reply(session()));
  assert.equal((await kept).status, "session");
  assert.equal(calls, 1);
  assert.equal(transport.csrf(), "fixture-secret");
});

test("all cancelled waiters abort the shared driver and cannot install late credentials", async () => {
  const held = deferred<Response>();
  let driverSignal: AbortSignal | undefined;
  const transport = createBrowserSessionTransport({
    base: "/mount/",
    now: () => 100,
    fetch: async (_url, init) => {
      driverSignal = init?.signal ?? undefined;
      return held.promise;
    },
  });
  const waiter = new AbortController();
  const result = transport.refresh(waiter.signal);
  await Promise.resolve();
  waiter.abort();
  assert.deepEqual(await result, { status: "changed" });
  assert.equal(driverSignal?.aborted, true);
  held.resolve(reply(session()));
  await Promise.resolve();
  assert.equal(transport.csrf(), undefined);
});

test("shared session results are independent copies for simultaneous consumers", async () => {
  const held = deferred<Response>();
  let calls = 0;
  const transport = createBrowserSessionTransport({
    base: "/mount/",
    now: () => 100,
    fetch: async () => {
      calls++;
      return held.promise;
    },
  });
  const first = transport.refresh(new AbortController().signal),
    second = transport.refresh(new AbortController().signal);
  held.resolve(reply(session()));
  const one = await first,
    two = await second;
  if (one.status === "session") one.session.generation = "mutated";
  assert.equal(two.status === "session" && two.session.generation, "one");
  assert.equal(calls, 1);
});

test("legacy refresh coalesces overlapping App and controller checks", async () => {
  const held = deferred<Response>();
  let calls = 0;
  const auth = createBrowserAuth("/mount/", async () => {
    calls++;
    return held.promise;
  });
  const first = auth.refresh(),
    second = auth.refresh();
  held.resolve(
    reply({ ...session(), expires_at: Math.floor(Date.now() / 1000) + 3600 }),
  );
  const one = await first,
    two = await second;
  one.generation = "mutated";
  assert.equal(two.generation, "one");
  assert.equal(calls, 1);
});

test("shared provider validation rejects duplicates and missing primaries", async () => {
  for (const body of [
    {
      providers: [
        { id: "one", label: "A" },
        { id: "one", label: "B" },
      ],
      primary: "one",
    },
    { providers: [{ id: "one", label: "A" }], primary: "missing" },
    {
      providers: Array.from({ length: 101 }, (_, i) => ({
        id: String(i),
        label: "A",
      })),
      primary: null,
    },
  ]) {
    const transport = createBrowserSessionTransport({
      base: "/mount/",
      now: () => 100,
      fetch: async () => reply(body),
    });
    await assert.rejects(transport.providers(new AbortController().signal));
  }
  const transport = createBrowserSessionTransport({
    base: "/mount/",
    now: () => 100,
    fetch: async () =>
      reply({ providers: [{ id: "a/b", label: "Provider" }], primary: "a/b" }),
  });
  assert.deepEqual(await transport.providers(new AbortController().signal), {
    providers: [{ id: "a/b", label: "Provider" }],
    primary: "a/b",
  });
});

test("legacy rejects invalid anonymous and authenticated protocol without fabricating defaults", async () => {
  for (const value of [
    { authenticated: false },
    { authenticated: false, generation: "wire", user_id: "forbidden" },
    { authenticated: false, generation: "wire", expires_at: 200 },
    {
      authenticated: true,
      generation: "wire",
      csrf_token: "secret",
      user_id: "alice",
      expires_at: "200",
    },
    {
      authenticated: true,
      generation: "wire",
      csrf_token: "secret",
      user_id: "alice",
      expires_at: 9007199254740992,
    },
    { authenticated: "true", generation: "wire" },
  ]) {
    const auth = createBrowserAuth("/mount/", async () => reply(value));
    await assert.rejects(auth.refresh(), /Invalid session response/);
    assert.equal(auth.csrf(), undefined);
  }
});

test("legacy expiry remains the exact exported typed error", async () => {
  const auth = createBrowserAuth("/mount/", async () =>
    reply({ ...session(), expires_at: 1 }),
  );
  await assert.rejects(
    auth.refresh(),
    (error) =>
      error instanceof SessionExpiredError &&
      error.name === "SessionExpiredError" &&
      error.message === "Session expired.",
  );
  assert.equal(auth.csrf(), undefined);
});

test("shared provider responses held across invalidation cannot republish old choices", async () => {
  const held = deferred<Response>();
  const transport = createBrowserSessionTransport({
    base: "/mount/",
    now: () => 100,
    fetch: async () => held.promise,
  });
  const old = transport.providers(new AbortController().signal);
  transport.invalidate();
  held.resolve(
    reply({ providers: [{ id: "old", label: "Old" }], primary: "old" }),
  );
  await assert.rejects(old, /Session changed/);
});

test("a cancelled flight leaves its slot available for a bounded replacement", async () => {
  let calls = 0,
    oldSignal: AbortSignal | undefined;
  const oldResponse = deferred<Response>();
  const transport = createBrowserSessionTransport({
    base: "/mount/",
    now: () => 100,
    timeoutMs: 50,
    fetch: async (_url, init) => {
      calls++;
      if (calls === 1) {
        oldSignal = init?.signal ?? undefined;
        return oldResponse.promise;
      }
      return reply(session("replacement"));
    },
  });
  const aborted = new AbortController();
  const old = transport.refresh(aborted.signal);
  aborted.abort();
  assert.deepEqual(await old, { status: "changed" });
  const next = await transport.refresh(new AbortController().signal);
  assert.equal(
    next.status === "session" && next.session.generation,
    "replacement",
  );
  assert.equal(oldSignal?.aborted, true);
  oldResponse.resolve(reply(session("old")));
  await Promise.resolve();
  assert.equal(transport.csrf(), "fixture-secret");
});

test("public driver suppresses protocol results invalidated before its adapter continuation", async () => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  const driver = createBrowserSessionDriver({
    base: "/mount/",
    authority: "fixture",
    now: () => 100,
    fetch: async () => reply(session()),
  });
  const pending = driver.check(new AbortController().signal);
  for (let tick = 0; tick < 50 && !driver.csrf(); tick++)
    await Promise.resolve();
  assert.equal(driver.csrf(), "fixture-secret");
  await Promise.resolve();
  await Promise.resolve();
  driver.invalidate?.();
  assert.deepEqual(await pending, { status: "anonymous" });
  assert.equal(driver.csrf(), undefined);
});

test("legacy adapter preserves Session changed after logout between transport and adapter completion", async () => {
  const auth = createBrowserAuth("/mount/", async (url) =>
    String(url).endsWith("logout")
      ? new Response(null, { status: 204 })
      : reply({
          ...session(),
          expires_at: Math.floor(Date.now() / 1000) + 3600,
        }),
  );
  const pending = auth.refresh();
  for (let tick = 0; tick < 50 && !auth.csrf(); tick++) await Promise.resolve();
  assert.equal(auth.csrf(), "fixture-secret");
  await Promise.resolve();
  await Promise.resolve();
  const stopped = auth.logout();
  await assert.rejects(pending, /Session changed/);
  await stopped;
  assert.equal(auth.csrf(), undefined);
});

test("the shared provider bound accepts exactly one hundred unique choices and preserves encoded login identity", async () => {
  const providers = Array.from({ length: 100 }, (_, i) => ({
    id: `provider-${i}`,
    label: `Provider ${i}`,
  }));
  const auth = createBrowserAuth("/mount/", async () =>
    reply({ providers, primary: "provider-99" }),
  );
  assert.deepEqual(await auth.providers(), {
    providers,
    primary: "provider-99",
  });
  assert.equal(auth.loginUrl("a/b ?#"), "/mount/auth/login/a%2Fb%20%3F%23");
});
