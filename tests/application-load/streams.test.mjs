import test from "node:test";
import assert from "node:assert/strict";
import { openLoadStreams } from "./streams.mjs";
test("readable and intentionally unread response streams share bounded lifecycle", async () => {
  const session = { cookie: "rom_session=synthetic", csrf: "synthetic" },
    views = [
      {
        key: { kind: "load-records", id: "load-00000" },
        revision: 1,
        value: {},
      },
    ];
  let cancelled = 0;
  const streams = await openLoadStreams(
    session,
    { normal: 1, slow: 1 },
    async (_, options) => {
      assert.equal(options.headers["x-rom-csrf"], session.csrf);
      return new Response(
        new ReadableStream({
          start(c) {
            c.enqueue(
              new TextEncoder().encode(
                "event: data\ndata: " + JSON.stringify(views) + "\n\n",
              ),
            );
          },
          cancel() {
            cancelled++;
          },
        }),
        { headers: { "content-type": "text/event-stream" } },
      );
    },
  );
  await new Promise((resolve) => setTimeout(resolve, 1));
  assert.equal(streams.counts().data_events, 1);
  const proof = await streams.close();
  assert.equal(proof.readers_drained, true);
  assert.equal(cancelled, 2);
});
test("refused or non-SSE observations do not masquerade as active subscriptions", async () => {
  await assert.rejects(
    openLoadStreams(
      { cookie: "synthetic", csrf: "synthetic" },
      { normal: 1, slow: 0 },
      async () => new Response("{}", { status: 401 }),
    ),
    /stream rejected/,
  );
});
test("premature EOF is a stream error rather than an active observation", async () => {
  const streams = await openLoadStreams(
    { cookie: "synthetic", csrf: "synthetic" },
    { normal: 4, slow: 1 },
    async () =>
      new Response(
        new ReadableStream({
          start(controller) {
            controller.close();
          },
        }),
        { headers: { "content-type": "text/event-stream" } },
      ),
  );
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(streams.counts().error_events, 4);
  assert.equal((await streams.close()).readers_drained, true);
});
test('server terminal SSE code and subsequent early EOF are separately diagnosed',async()=>{for(const code of['overloaded','closed','internal','SECRET']){const s=await openLoadStreams({cookie:'synthetic',csrf:'synthetic'},{normal:1,slow:0},async()=>new Response(new ReadableStream({start(c){c.enqueue(new TextEncoder().encode(`event: error\ndata: ${code}\n\n`));c.close();}}),{headers:{'content-type':'text/event-stream'}}));await new Promise(r=>setImmediate(r));const x=await s.close();assert.equal(x.error_events,2);assert.equal(x.error_classes['server-'+(code==='SECRET'?'other':code)],1);assert.equal(x.error_classes['early-eof'],1);assert.ok(!JSON.stringify(x).includes('SECRET'));}});
