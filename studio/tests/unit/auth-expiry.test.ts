import test from "node:test";
import assert from "node:assert/strict";
import {
  createBrowserAuth,
  SessionExpiredError,
} from "../../src/lib/application/auth.ts";

test("an expired authenticated session cannot retain the previous CSRF authority", async () => {
  let expiry = Math.floor(Date.now() / 1000) + 3600;
  const auth = createBrowserAuth(
    "/rom-studio/",
    async () =>
      new Response(
        JSON.stringify({
          authenticated: true,
          generation: "session-a",
          csrf_token: "synthetic-csrf",
          user_id: "alice",
          expires_at: expiry,
        }),
        { headers: { "content-type": "application/json" } },
      ),
  );
  await auth.refresh();
  assert.equal(auth.csrf(), "synthetic-csrf");
  expiry = 1;
  await assert.rejects(auth.refresh(), SessionExpiredError);
  assert.equal(auth.csrf(), undefined);
});

test("an old expired response cannot clear replacement session authority", async () => {
  let calls = 0;
  let resolveOld!: (response: Response) => void;
  const response = (csrf: string, expires: number) =>
    new Response(
      JSON.stringify({
        authenticated: true,
        generation: csrf,
        csrf_token: csrf,
        user_id: "alice",
        expires_at: expires,
      }),
      { headers: { "content-type": "application/json" } },
    );
  const auth = createBrowserAuth("/rom-studio/", async (url) => {
    if (String(url).endsWith("logout"))
      return new Response(null, { status: 204 });
    calls++;
    if (calls === 2)
      return new Promise<Response>((resolve) => {
        resolveOld = resolve;
      });
    return response(
      calls === 1 ? "first" : "replacement",
      Math.floor(Date.now() / 1000) + 3600,
    );
  });
  await auth.refresh();
  const old = auth.refresh();
  await auth.logout();
  await auth.refresh();
  resolveOld(response("expired", 1));
  await assert.rejects(old, /Session changed/);
  assert.equal(auth.csrf(), "replacement");
});
