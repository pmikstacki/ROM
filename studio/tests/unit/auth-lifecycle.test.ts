import test from "node:test";
import assert from "node:assert/strict";
import { createSessionLifecycle, type SessionCheck } from "../../src/auth.ts";

const alice = {
  principal: { authority: "fixture", kind: "human" as const, subject: "alice" },
  generation: "one",
  expiresAt: 200,
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

test("simultaneous session checks share acquisition and publish sanitized identity", async (context) => {
  const pending = deferred<SessionCheck>();
  let calls = 0;
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: () => {
        calls++;
        return pending.promise;
      },
      logout: async () => {},
    },
  });
  context.after(() => lifecycle.dispose());
  try {
    const first = lifecycle.refresh(),
      second = lifecycle.refresh();
    assert.equal(first, second);
    pending.resolve({ status: "authenticated", identity: alice });
    assert.equal((await first).status, "authenticated");
    assert.equal(calls, 1);
    assert.deepEqual(lifecycle.state.identity, alice);
    assert.equal(JSON.stringify(lifecycle.state).includes("csrf"), false);
  } finally {
    lifecycle.dispose();
  }
});

test("transient failure retains validated authority only until its original expiry", async (context) => {
  let now = 100;
  let result: SessionCheck = { status: "authenticated", identity: alice };
  let expire: (() => void) | undefined;
  const lifecycle = createSessionLifecycle({
    now: () => now,
    schedule: (task) => {
      expire = task;
      return () => {
        expire = undefined;
      };
    },
    driver: { check: async () => result, logout: async () => {} },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  result = { status: "transient", code: "unavailable" };
  await lifecycle.refresh();
  assert.equal(lifecycle.state.identity?.principal.subject, "alice");
  assert.equal(lifecycle.state.status, "transient");
  lifecycle.dismissTransient();
  assert.equal(lifecycle.state.status, "authenticated");
  assert.equal(lifecycle.state.identity?.expiresAt, 200);
  now = 201;
  assert.ok(expire);
  expire();
  assert.equal(lifecycle.state.identity, null);
  assert.equal(lifecycle.state.status, "anonymous");
  lifecycle.dispose();
});

test("logout invalidates a held check before calling the driver or publishing", async (context) => {
  const pending = deferred<SessionCheck>();
  let calls = 0;
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () =>
        ++calls === 1
          ? { status: "authenticated", identity: alice }
          : pending.promise,
      logout: async () => {
        assert.equal(lifecycle.state.identity, null);
      },
    },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  const old = lifecycle.refresh();
  await Promise.resolve();
  await lifecycle.logout();
  pending.resolve({ status: "authenticated", identity: alice });
  assert.equal((await old).status, "anonymous");
  assert.equal(lifecycle.state.identity, null);
  lifecycle.dispose();
});

test("authority is cleared before a transition hook and reentrant logout suppresses the candidate", async (context) => {
  let checks = 0;
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () => ({
        status: "authenticated",
        identity: { ...alice, generation: String(++checks) },
      }),
      logout: async () => {},
    },
    onTransition: async (transition) => {
      if (transition.kind === "renewed") {
        assert.equal(lifecycle.state.identity, null);
        await lifecycle.logout();
      }
    },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  assert.equal((await lifecycle.refresh()).status, "anonymous");
  assert.equal(lifecycle.state.identity, null);
  lifecycle.dispose();
});

test("listeners cannot disclose a late identity after a prior listener disposes", async (context) => {
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () => ({ status: "authenticated", identity: alice }),
      logout: async () => {},
    },
  });
  context.after(() => lifecycle.dispose());
  let leaked = false;
  lifecycle.subscribe((state) => {
    if (state.identity) lifecycle.dispose();
  });
  lifecycle.subscribe((state) => {
    if (state.identity) leaked = true;
  });
  assert.equal((await lifecycle.refresh()).status, "anonymous");
  assert.equal(leaked, false);
});

test("expiry cancels a check whose driver ignores abort without waiting for a response", async (context) => {
  let now = 100;
  let expire: (() => void) | undefined;
  let calls = 0;
  const lifecycle = createSessionLifecycle({
    now: () => now,
    schedule: (task) => {
      expire = task;
      return () => {
        expire = undefined;
      };
    },
    driver: {
      check: async () =>
        ++calls === 1
          ? { status: "authenticated", identity: alice }
          : new Promise<SessionCheck>(() => {}),
      logout: async () => {},
    },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  const held = lifecycle.refresh();
  await Promise.resolve();
  now = 201;
  expire?.();
  const result = await Promise.race([
    held,
    new Promise<string>((resolve) => setTimeout(() => resolve("hung"), 20)),
  ]);
  assert.deepEqual(result, { status: "anonymous" });
  assert.equal(lifecycle.state.identity, null);
  lifecycle.dispose();
});

test("state and transition snapshots are isolated from host mutations", async (context) => {
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () => ({ status: "authenticated", identity: alice }),
      logout: async () => {},
    },
    onTransition: (transition) => {
      if (transition.next) transition.next.principal.subject = "changed";
    },
  });
  context.after(() => lifecycle.dispose());
  const result = await lifecycle.refresh();
  if (result.status === "authenticated")
    result.identity.principal.subject = "result-mutated";
  const state = lifecycle.state;
  if (state.identity) state.identity.principal.subject = "snapshot-mutated";
  assert.equal(lifecycle.state.identity?.principal.subject, "alice");
  lifecycle.dispose();
});

test("browser driver binds validated human subject to host authority and keeps CSRF private", async (context) => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  const driver = createBrowserSessionDriver({
    base: "/rom-studio/",
    authority: "host-a",
    now: () => 100,
    fetch: async () =>
      new Response(
        '{"authenticated":true,"generation":"new","csrf_token":"fixture-secret","user_id":"alice","expires_at":200}',
        { headers: { "content-type": "application/json" } },
      ),
  });
  const result = await driver.check(new AbortController().signal);
  assert.deepEqual(result, {
    status: "authenticated",
    identity: {
      principal: { authority: "host-a", kind: "human", subject: "alice" },
      generation: "new",
      expiresAt: 200,
    },
  });
  assert.equal(JSON.stringify(result).includes("fixture-secret"), false);
  assert.equal(driver.csrf(), "fixture-secret");
});

test("browser driver distinguishes structured denial from temporary or malformed failures", async (context) => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  let response = () =>
    new Response(
      '{"authenticated":true,"generation":"one","csrf_token":"fixture-secret","user_id":"alice","expires_at":200}',
      { headers: { "content-type": "application/json" } },
    );
  const driver = createBrowserSessionDriver({
    base: "/rom-studio/",
    authority: "fixture",
    now: () => 100,
    fetch: async () => response(),
  });
  const signal = new AbortController().signal;
  await driver.check(signal);
  response = () =>
    new Response('{"error":"overloaded"}', {
      status: 503,
      headers: { "content-type": "application/json" },
    });
  assert.deepEqual(await driver.check(signal), {
    status: "transient",
    code: "overloaded",
  });
  assert.equal(driver.csrf(), "fixture-secret");
  response = () => new Response("unknown proxy failure", { status: 401 });
  assert.deepEqual(await driver.check(signal), {
    status: "transient",
    code: "invalid_response",
  });
  assert.equal(driver.csrf(), "fixture-secret");
  response = () =>
    new Response('{"error":"denied"}', {
      status: 401,
      headers: { "content-type": "application/json" },
    });
  assert.deepEqual(await driver.check(signal), { status: "denied" });
  assert.equal(driver.csrf(), undefined);
});

test("malformed sessions cannot renew authority and validated anonymous sessions clear it", async (context) => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  let now = 100;
  let body =
    '{"authenticated":true,"generation":"one","csrf_token":"fixture-secret","user_id":"alice","expires_at":200}';
  const driver = createBrowserSessionDriver({
    base: "/rom-studio/",
    authority: "fixture",
    now: () => now,
    fetch: async () =>
      new Response(body, { headers: { "content-type": "application/json" } }),
  });
  const signal = new AbortController().signal;
  await driver.check(signal);
  body =
    '{"authenticated":true,"generation":"bad","csrf_token":"new-secret","user_id":"alice","expires_at":9007199254740993}';
  assert.deepEqual(await driver.check(signal), {
    status: "transient",
    code: "invalid_response",
  });
  assert.equal(driver.csrf(), "fixture-secret");
  now = 201;
  assert.equal(driver.csrf(), undefined);
  body = '{"authenticated":false,"generation":"anonymous","csrf_token":"leak"}';
  assert.deepEqual(await driver.check(signal), {
    status: "transient",
    code: "invalid_response",
  });
  body = '{"authenticated":false,"generation":"anonymous"}';
  assert.deepEqual(await driver.check(signal), { status: "anonymous" });
});

test("malformed host checks cannot establish an unrecognized lifecycle state", async (context) => {
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () =>
        JSON.parse('{"status":"surprise","identity":{"csrf_token":"private"}}'),
      logout: async () => {},
    },
  });
  context.after(() => lifecycle.dispose());
  assert.deepEqual(await lifecycle.refresh(), {
    status: "transient",
    code: "invalid_response",
  });
  assert.equal(lifecycle.state.identity, null);
  lifecycle.dispose();
});

test("a refused logout cannot leak a driver error after replacement authority", async (context) => {
  const pending = deferred<void>();
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () => ({ status: "authenticated", identity: alice }),
      logout: () => pending.promise,
    },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  const stopped = lifecycle.logout();
  await lifecycle.refresh();
  pending.reject(Error("private old owner detail"));
  await stopped;
  assert.equal(lifecycle.state.identity?.principal.subject, "alice");
  lifecycle.dispose();
});

test("a hook failure clears browser credential authority as well as observable identity", async (context) => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  const driver = createBrowserSessionDriver({
    base: "/rom-studio/",
    authority: "fixture",
    now: () => 100,
    fetch: async () =>
      new Response(
        '{"authenticated":true,"generation":"one","csrf_token":"fixture-secret","user_id":"alice","expires_at":200}',
        { headers: { "content-type": "application/json" } },
      ),
  });
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver,
    onTransition: () => {
      throw Error("private host detail");
    },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  assert.equal(lifecycle.state.identity, null);
  assert.equal(lifecycle.state.transientCode, "transition_failed");
  assert.equal(driver.csrf(), undefined);
  lifecycle.dispose();
});

test("an invalid clock cannot establish or retain browser authority", async (context) => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  let now = 100;
  const driver = createBrowserSessionDriver({
    base: "/rom-studio/",
    authority: "fixture",
    now: () => now,
    fetch: async () =>
      new Response(
        '{"authenticated":true,"generation":"one","csrf_token":"fixture-secret","user_id":"alice","expires_at":200}',
        { headers: { "content-type": "application/json" } },
      ),
  });
  const lifecycle = createSessionLifecycle({ now: () => now, driver });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  now = NaN;
  assert.equal(lifecycle.state.identity, null);
  assert.equal(driver.csrf(), undefined);
  assert.notEqual((await lifecycle.refresh()).status, "authenticated");
});

test("browser acquisition has a finite deadline even when fetch ignores cancellation", async () => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  const driver = createBrowserSessionDriver({
    base: "/rom-studio/",
    authority: "fixture",
    now: () => 100,
    timeoutMs: 5,
    fetch: () => new Promise<Response>(() => {}),
  });
  assert.deepEqual(await driver.check(new AbortController().signal), {
    status: "transient",
    code: "unavailable",
  });
});

test("an expiry reached inside an asynchronous hook prevents authority publication", async (context) => {
  let now = 100;
  const lifecycle = createSessionLifecycle({
    now: () => now,
    driver: {
      check: async () => ({ status: "authenticated", identity: alice }),
      logout: async () => {},
    },
    onTransition: async (transition) => {
      if (transition.next) {
        await Promise.resolve();
        now = 201;
      }
    },
  });
  context.after(() => lifecycle.dispose());
  assert.equal((await lifecycle.refresh()).status, "anonymous");
  assert.equal(lifecycle.state.identity, null);
});

test("credential invalidation cannot throw past disposal or retain observable private state", async (context) => {
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () => ({ status: "authenticated", identity: alice }),
      logout: async () => {},
      invalidate: () => {
        throw Error("private invalidation detail");
      },
    },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  assert.doesNotThrow(() => lifecycle.dispose());
  assert.equal(lifecycle.state.identity, null);
  assert.equal(lifecycle.state.status, "disposed");
});

test("a reentrant acquisition during credential invalidation supersedes the clearing hook", async (context) => {
  let result: SessionCheck = { status: "authenticated", identity: alice };
  let replaced: Promise<SessionCheck> | undefined;
  const hooks: string[] = [];
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () => result,
      logout: async () => {},
      invalidate: () => {
        if (result.status === "anonymous") {
          result = {
            status: "authenticated",
            identity: { ...alice, generation: "replacement" },
          };
          replaced = lifecycle.refresh();
        }
      },
    },
    onTransition: (transition) => {
      hooks.push(transition.kind);
    },
  });
  context.after(() => lifecycle.dispose());
  await lifecycle.refresh();
  hooks.length = 0;
  result = { status: "anonymous" };
  await lifecycle.refresh();
  await replaced;
  assert.deepEqual(hooks, ["changed"]);
  assert.equal(lifecycle.state.identity?.generation, "replacement");
});

test("browser driver captures its host profile rather than trusting later option mutations", async () => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  const options = {
    base: "/rom-studio/",
    authority: "original",
    now: () => 100,
    fetch: async () =>
      new Response(
        '{"authenticated":true,"generation":"one","csrf_token":"fixture-secret","user_id":"alice","expires_at":200}',
        { headers: { "content-type": "application/json" } },
      ),
  };
  const driver = createBrowserSessionDriver(options);
  options.authority = "changed";
  const result = await driver.check(new AbortController().signal);
  assert.equal(
    result.status === "authenticated" && result.identity.principal.authority,
    "original",
  );
});

test("a browser response held across logout cannot replace fresh credential authority", async () => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  const held = deferred<Response>();
  let calls = 0;
  const response = (token: string) =>
    new Response(
      JSON.stringify({
        authenticated: true,
        generation: token,
        csrf_token: token,
        user_id: "alice",
        expires_at: 200,
      }),
      { headers: { "content-type": "application/json" } },
    );
  const driver = createBrowserSessionDriver({
    base: "/rom-studio/",
    authority: "fixture",
    now: () => 100,
    fetch: async (url, init) => {
      if (String(url).endsWith("logout")) {
        assert.equal(new Headers(init?.headers).get("x-rom-csrf"), "first");
        return new Response(null, { status: 204 });
      }
      return ++calls === 2
        ? held.promise
        : response(calls === 1 ? "first" : "replacement");
    },
  });
  const signal = new AbortController().signal;
  await driver.check(signal);
  const old = driver.check(signal);
  await driver.logout(signal);
  await driver.check(signal);
  held.resolve(response("old"));
  assert.deepEqual(await old, { status: "anonymous" });
  assert.equal(driver.csrf(), "replacement");
});

test("oversized and invalid UTF-8 browser responses cannot establish authority", async () => {
  const { createBrowserSessionDriver } = await import("../../src/auth.ts");
  for (const bytes of [new Uint8Array(65537), new Uint8Array([255])]) {
    const driver = createBrowserSessionDriver({
      base: "/rom-studio/",
      authority: "fixture",
      now: () => 100,
      fetch: async () =>
        new Response(bytes, {
          headers: { "content-type": "application/json" },
        }),
    });
    assert.deepEqual(await driver.check(new AbortController().signal), {
      status: "transient",
      code: "invalid_response",
    });
    assert.equal(driver.csrf(), undefined);
  }
});

test("concurrent refresh remains single-flight while authority rebinding awaits its hook", async (context) => {
  const entered = deferred<void>(),
    release = deferred<void>();
  let calls = 0;
  const lifecycle = createSessionLifecycle({
    now: () => 100,
    driver: {
      check: async () => {
        calls++;
        return { status: "authenticated", identity: alice };
      },
      logout: async () => {},
    },
    onTransition: async () => {
      entered.resolve();
      await release.promise;
    },
  });
  context.after(() => {
    release.resolve();
    lifecycle.dispose();
  });
  const first = lifecycle.refresh();
  await entered.promise;
  const second = lifecycle.refresh();
  assert.equal(first, second);
  release.resolve();
  await first;
  assert.equal(calls, 1);
});

for (const failure of ["throw", "reject"])
  test(`logout captures transport credentials then invalidates them before hooks when the driver ${failure}s`, async (context) => {
    let credential: string | undefined = "fixture-secret";
    const order: string[] = [];
    const lifecycle = createSessionLifecycle({
      now: () => 100,
      driver: {
        check: async () => ({ status: "authenticated", identity: alice }),
        logout: () => {
          assert.equal(credential, "fixture-secret");
          order.push("captured");
          if (failure === "reject")
            return Promise.reject(Error("private logout failure"));
          throw Error("private logout failure");
        },
        invalidate: () => {
          credential = undefined;
          order.push("invalidated");
        },
      },
      onTransition: (transition) => {
        if (transition.kind === "cleared") {
          assert.equal(credential, undefined);
          assert.equal(lifecycle.state.identity, null);
          order.push("hook");
        }
      },
    });
    context.after(() => lifecycle.dispose());
    await lifecycle.refresh();
    await assert.rejects(lifecycle.logout(), /^Error: logout_unconfirmed$/);
    assert.equal(credential, undefined);
    assert.deepEqual(order, ["captured", "invalidated", "hook"]);
    assert.equal(lifecycle.state.identity, null);
  });
