import { test } from "node:test";
import assert from "node:assert/strict";
import { createBrowserAuth } from "../../src/lib/application/auth.ts";
function reply(value: unknown) {
  return new Response(JSON.stringify(value), {
    headers: { "content-type": "application/json" },
  });
}
test("browser session supplies CSRF only after verified authenticated response", async () => {
  const auth = createBrowserAuth("/rom-studio/", async () =>
    reply({
      authenticated: true,
      generation: "g",
      csrf_token: "csrf",
      user_id: "human",
      expires_at: Math.floor(Date.now() / 1000) + 60,
    }),
  );
  assert.equal(auth.csrf(), undefined);
  await auth.refresh();
  assert.equal(auth.csrf(), "csrf");
});
test("unauthenticated authority and expired session fail closed", async () => {
  for (const value of [
    { authenticated: false, generation: "g", csrf_token: "leak" },
    {
      authenticated: true,
      generation: "g",
      csrf_token: "csrf",
      user_id: "human",
      expires_at: 1,
    },
  ]) {
    const auth = createBrowserAuth("/rom-studio/", async () => reply(value));
    await assert.rejects(auth.refresh());
    assert.equal(auth.csrf(), undefined);
  }
});
test("logout includes last CSRF and clears it before request returns", async () => {
  let sent = "";
  const auth = createBrowserAuth("/rom-studio/", async (url, options) => {
    if (String(url).endsWith("/logout")) {
      sent = new Headers(options?.headers).get("x-rom-csrf") ?? "";
      assert.equal(auth.csrf(), undefined);
      return new Response(null, { status: 204 });
    }
    return reply({
      authenticated: true,
      generation: "g",
      csrf_token: "csrf",
      user_id: "human",
      expires_at: Math.floor(Date.now() / 1000) + 60,
    });
  });
  await auth.refresh();
  await auth.logout();
  assert.equal(sent, "csrf");
});
test("primary provider must exist; provider URL is encoded", async () => {
  const auth = createBrowserAuth("/mount/", async () =>
    reply({ providers: [{ id: "a/b", label: "label" }], primary: "missing" }),
  );
  await assert.rejects(auth.providers());
  assert.equal(auth.loginUrl("a/b"), "/mount/auth/login/a%2Fb");
});

test("delayed session response cannot restore authority after logout", async () => {
  let resolve!: (value: Response) => void;
  const auth = createBrowserAuth(
    "/rom-studio/",
    async () => await new Promise((r) => (resolve = r)),
  );
  const refresh = auth.refresh();
  await auth.logout();
  resolve(
    reply({
      authenticated: true,
      generation: "old",
      csrf_token: "old-csrf",
      user_id: "human",
      expires_at: Math.floor(Date.now() / 1000) + 60,
    }),
  );
  await assert.rejects(refresh, /Session changed/);
  assert.equal(auth.csrf(), undefined);
});
