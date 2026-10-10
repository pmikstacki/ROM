import test from "node:test";
import assert from "node:assert/strict";
import { httpExecutor } from "./http-executor.mjs";
const session = { cookie: "rom_session=synthetic", csrf: "synthetic-csrf" };
const read = {
  method: "POST",
  path: "/rom-studio/api/read",
  body: { kind: "load-records", id: "load-00000" },
  measurement: "read",
};
const view = {
  key: { kind: "load-records", id: "load-00000" },
  revision: 1,
  value: { title: "example" },
};
test("actual HTTP shape verifies resource identity and does not retain payload", async () => {
  const http = httpExecutor(session, async (url, options) => {
    assert.equal(url, "https://127.0.0.1:44389/rom-studio/api/read");
    assert.equal(options.headers.cookie, session.cookie);
    return new Response(JSON.stringify(view));
  });
  assert.equal(
    await http.execute(read, new AbortController().signal),
    "success",
  );
  assert.equal(http.counts().requests, 1);
  assert.ok(!JSON.stringify(http.counts()).includes("synthetic"));
  const wrong = httpExecutor(
    session,
    async () =>
      new Response(
        JSON.stringify({ ...view, key: { ...view.key, id: "different" } }),
      ),
  );
  assert.equal(
    await wrong.execute(read, new AbortController().signal),
    "failure",
  );
});
test("denial and temporary failure are separate outcomes; absent mutation response is unknown", async () => {
  for (const [status, outcome] of [
    [401, "denied"],
    [403, "denied"],
    [429, "overloaded"],
    [503, "overloaded"],
  ]) {
    const http = httpExecutor(
      session,
      async () => new Response("{}", { status }),
    );
    assert.equal(
      await http.execute(read, new AbortController().signal),
      outcome,
    );
  }
  const http = httpExecutor(session, async () => {
    throw Error("transport failure");
  });
  assert.equal(
    await http.execute(
      { ...read, measurement: "commit" },
      new AbortController().signal,
    ),
    "unknown",
  );
});
test("binary output must match actual bounded attachment bytes", async () => {
  const http = httpExecutor(
    session,
    async () => new Response(Buffer.alloc(65536, 0x61)),
  );
  assert.equal(
    await http.execute(
      {
        method: "GET",
        path: "/rom-studio/blobs/attachment/load-attachment-0",
        expected_bytes: 65536,
        measurement: "download",
      },
      new AbortController().signal,
    ),
    "success",
  );
  const wrong = httpExecutor(
    session,
    async () => new Response(Buffer.alloc(65536, 0x62)),
  );
  assert.equal(
    await wrong.execute(
      {
        method: "GET",
        path: "/rom-studio/blobs/attachment/load-attachment-0",
        expected_bytes: 65536,
        measurement: "download",
      },
      new AbortController().signal,
    ),
    "failure",
  );
});
test("HTTP 503 outcome_unknown is not mislabeled as capacity overload", async () => {
  const http = httpExecutor(
    session,
    async () => new Response('{"error":"outcome_unknown"}', { status: 503 }),
  );
  assert.equal(
    await http.execute(
      { ...read, measurement: "commit" },
      new AbortController().signal,
    ),
    "unknown",
  );
});
test("all Studio API POSTs carry origin and CSRF, including reads and queries", async () => {
  const http = httpExecutor(session, async (_, options) => {
    assert.equal(options.headers.origin, "https://127.0.0.1:44389");
    assert.equal(options.headers["x-rom-csrf"], session.csrf);
    return new Response(JSON.stringify(view));
  });
  assert.equal(
    await http.execute(read, new AbortController().signal),
    "success",
  );
});
const workHandle = "a".repeat(64);
const workView = {
  protocol_version: 1,
  handle: workHandle,
  version: { generation: "fixture-generation", revision: 1 },
  category: "Reaction",
  definition: { name: "load-observation", version: 1 },
  state: "Done",
  attempts: 1,
  due: 0,
  delivery: null,
  source: { kind: "load-records", id: "load-00000" },
  target: null,
};
function workRequest(route, body = {}) {
  return {
    method: "POST",
    path: `/rom-studio/api/work/${route}`,
    body,
    measurement: "work",
    operator: true,
  };
}
test("Work capabilities/list/read use shared operator response semantics rather than Resource shape", async () => {
  for (const [request, response] of [
    [
      workRequest("capabilities"),
      { protocol_version: 1, inspect: true, retry: true, reconcile: true },
    ],
    [
      workRequest("list", { limit: 2 }),
      { protocol_version: 1, records: [workView], cursor: null },
    ],
    [workRequest("read", { handle: workHandle }), workView],
  ]) {
    const http = httpExecutor(
      session,
      async () => new Response(JSON.stringify(response)),
    );
    assert.equal(
      await http.execute(request, new AbortController().signal),
      "success",
    );
  }
});
test("Work invalid protocol, category, identity and duplicate handles reject", async () => {
  for (const [request, response] of [
    [
      workRequest("capabilities"),
      { protocol_version: 2, inspect: true, retry: true, reconcile: true },
    ],
    [
      workRequest("read", { handle: workHandle }),
      { ...workView, category: "made-up" },
    ],
    [workRequest("read", { handle: "b".repeat(64) }), workView],
    [
      workRequest("list", { limit: 2 }),
      { protocol_version: 1, records: [workView, workView], cursor: null },
    ],
  ]) {
    const http = httpExecutor(
      session,
      async () => new Response(JSON.stringify(response)),
    );
    assert.equal(
      await http.execute(request, new AbortController().signal),
      "failure",
    );
  }
});
test("Work controls require explicit operator scope and keep lost acknowledgement unknown", async () => {
  const http = httpExecutor(session, async () => {
    throw Error("synthetic secret");
  });
  const request = workRequest("control", {
    handle: workHandle,
    expected: workView.version,
    key: "fixture-control",
    retry_epoch: 0,
    operation: "Retry",
  });
  assert.equal(
    await http.execute(request, new AbortController().signal),
    "unknown",
  );
  await assert.rejects(
    http.execute({ ...request, operator: false }, new AbortController().signal),
    /operator/,
  );
});
test("Work control correspondence and uint64 validation reuse the public client parser", async () => {
  const request = workRequest("control", {
    handle: workHandle,
    expected: workView.version,
    key: "fixture-control",
    retry_epoch: 0,
    operation: "Retry",
  });
  const response = {
    protocol_version: 1,
    handle: workHandle,
    version: { ...workView.version, revision: 2 },
    key: "fixture-control",
    operation: "Retry",
    outcome: "Scheduled",
    replayed: false,
  };
  for (const [value, want] of [
    [response, "success"],
    [{ ...response, key: "other" }, "unknown"],
    [{ ...response, version: { ...response.version, revision: 3 } }, "unknown"],
  ]) {
    const http = httpExecutor(
      session,
      async () => new Response(JSON.stringify(value)),
    );
    assert.equal(
      await http.execute(request, new AbortController().signal),
      want,
    );
  }
});
const blobView={key:{kind:'blobs',id:'load-attachment-0'},revision:1,value:{store:'attachments'}};
const reserveRequest={method:'POST',path:'/rom-studio/blobs/reserve',body:{id:'load-attachment-0'},measurement:'reserve'};
const uploadRequest={method:'POST',path:'/rom-studio/blobs/upload?id=load-attachment-0',binary_bytes:65536,measurement:'upload'};
test('actual projected blob envelopes accept reserve and upload without changing identity checks',async()=>{for(const [request,status]of[[reserveRequest,'reserved'],[uploadRequest,'attached']]){const http=httpExecutor(session,async()=>new Response(JSON.stringify({status,resource:blobView})));assert.equal(await http.execute(request,new AbortController().signal),'success');}});
test('blob envelope rejects wrong status/kind/ID/missing resource and bare view',async()=>{for(const [request,response]of[[reserveRequest,{status:'attached',resource:blobView}],[uploadRequest,{status:'reserved',resource:blobView}],[reserveRequest,{status:'reserved',resource:{...blobView,key:{kind:'other',id:blobView.key.id}}}],[uploadRequest,{status:'attached',resource:{...blobView,key:{...blobView.key,id:'other'}}}],[reserveRequest,{status:'reserved'}],[reserveRequest,blobView]]){const http=httpExecutor(session,async()=>new Response(JSON.stringify(response)));assert.equal(await http.execute(request,new AbortController().signal),'failure');}});
test('bounded failure classification keeps no payload or raw error values',async()=>{const http=httpExecutor(session,async()=>new Response(JSON.stringify({status:'SECRET',resource:blobView})));assert.equal(await http.execute(reserveRequest,new AbortController().signal),'failure');assert.equal(http.counts().failure_classes['reserve:blob-envelope'],1);assert.ok(!JSON.stringify(http.counts()).includes('SECRET'));const capacity=httpExecutor(session,async()=>new Response('{}',{status:429}));assert.equal(await capacity.execute(reserveRequest,new AbortController().signal),'overloaded');assert.equal(capacity.counts().failure_classes['reserve:http-429'],1);});
test('closed HTTP backend code counter distinguishes overload closed and internal without secrets',async()=>{for(const [status,code,expected]of[[429,'overloaded','overloaded'],[503,'closed','closed'],[500,'internal','internal'],[503,'outcome_unknown','outcome_unknown'],[503,'SECRET','other']]){const h=httpExecutor(session,async()=>new Response(JSON.stringify({error:code,message:'SECRET'}),{status}));await h.execute(read,new AbortController().signal);assert.equal(h.counts().http_error_codes[expected],1);assert.ok(!JSON.stringify(h.counts()).includes('SECRET'));}});
