import test from "node:test";
import assert from "node:assert/strict";
import { privateLoadSession } from "./session.mjs";
const session = {
    authenticated: true,
    user_id: "fixture-user",
    csrf_token: "actual-csrf",
  },
  cookie = {
    name: "rom_session",
    domain: "127.0.0.1",
    secure: true,
    path: "/rom-studio/",
    value: "actual-cookie",
  };
test("only actual linked synthetic session and secure exact-origin cookie enter executor", () => {
  assert.deepEqual(privateLoadSession(session, [cookie]), {
    cookie: "rom_session=actual-cookie",
    csrf: "actual-csrf",
  });
  for (const variant of [
    { ...session, user_id: "other" },
    { ...session, authenticated: false },
    { ...session, csrf_token: "bad\nheader" },
  ])
    assert.throws(() => privateLoadSession(variant, [cookie]));
  for (const cookies of [
    [],
    [cookie, cookie],
    [{ ...cookie, secure: false }],
    [{ ...cookie, domain: "other" }],
  ])
    assert.throws(() => privateLoadSession(session, cookies));
});
