import test from "node:test";
import assert from "node:assert/strict";
import { requireTokenWindow, withLoadValidity } from "./provider-validity.mjs";

function control({ patchFailure = false, restoreFailure = false } = {}) {
  let validity = "minutes=5";
  const calls = [];
  return {
    calls,
    async request(path, method = "GET", body) {
      assert.equal(path, "/api/v3/providers/oauth2/1/");
      calls.push({
        method,
        ...(body ? { validity: body.access_token_validity } : {}),
      });
      if (method === "PATCH") {
        validity = body.access_token_validity;
        if (validity === "minutes=20" && patchFailure)
          throw Error("patch response lost");
        if (validity === "minutes=5" && restoreFailure)
          throw Error("restore response lost");
      }
      return { access_token_validity: validity, client_secret: "not evidence" };
    },
  };
}
test("actual token timing needs traffic plus cleanup budget, distinct from expiry fault", () => {
  const timing = { iat: 100, exp: 400, received_at_ms: 100100 };
  assert.equal(
    requireTokenWindow(timing, 110000, 150000, 45000, "exploratory")
      .remaining_ms,
    290000,
  );
  assert.throws(
    () => requireTokenWindow(timing, 210000, 150000, 45000, "exploratory"),
    /fresh login/,
  );
  assert.throws(
    () =>
      requireTokenWindow(
        { ...timing, exp: 1300 },
        110000,
        150000,
        45000,
        "exploratory",
      ),
    /token timing/,
  );
  assert.doesNotThrow(() =>
    requireTokenWindow({ ...timing, exp: 1300 }, 110000, 840000, 45000, "full"),
  );
  for (const extra of [
    { token: "secret" },
    { exp: Infinity },
    { iat: 500 },
    { received_at_ms: 400000 },
  ])
    assert.throws(() =>
      requireTokenWindow(
        { ...timing, ...extra },
        110000,
        150000,
        45000,
        "exploratory",
      ),
    );
});
test("full load temporarily sets exact synthetic validity and verifies restoration", async () => {
  const api = control(),
    evidence = [];
  assert.equal(
    await withLoadValidity(api, "full", async () => 42, evidence),
    42,
  );
  assert.equal(api.calls.length, 5);
  assert.equal(api.calls[1].validity, "minutes=20");
  assert.equal(api.calls[3].validity, "minutes=5");
  assert.equal(evidence.at(-1).restored, true);
  assert.ok(!JSON.stringify(evidence).includes("client_secret"));
});
test("restoration runs after traffic failure and uncertain patch response", async () => {
  for (const patchFailure of [false, true]) {
    const api = control({ patchFailure }),
      evidence = [];
    await assert.rejects(
      withLoadValidity(
        api,
        "full",
        async () => {
          throw Error("traffic failed");
        },
        evidence,
      ),
    );
    assert.equal(api.calls.at(-2).validity, "minutes=5");
    assert.equal(evidence.at(-1).restored, true);
  }
});
test("restoration failure is explicit and cannot be accepted as successful load", async () => {
  const api = control({ restoreFailure: true }),
    evidence = [];
  await assert.rejects(
    withLoadValidity(api, "full", async () => 42, evidence),
    /restoration failed/,
  );
  assert.equal(evidence.at(-1).restored, false);
});
test("exploratory run reads existing five-minute setting without changing it", async () => {
  const api = control(),
    evidence = [];
  await withLoadValidity(api, "exploratory", async () => {}, evidence);
  assert.equal(api.calls.length, 1);
  assert.equal(api.calls[0].method, "GET");
  await assert.rejects(
    withLoadValidity(api, "unbounded", async () => {}, []),
    /closed load profile/,
  );
});
test("body and restoration failures are both preserved", async () => {
  const api = control({ restoreFailure: true });
  await assert.rejects(
    withLoadValidity(
      api,
      "full",
      async () => {
        throw Error("traffic failed");
      },
      [],
    ),
    (error) =>
      error instanceof AggregateError &&
      error.errors.length === 2 &&
      error.errors[0].message === "traffic failed",
  );
});
