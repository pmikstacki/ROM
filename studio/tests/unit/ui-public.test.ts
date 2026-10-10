import test from "node:test";
import assert from "node:assert/strict";
import { createLatestRequest } from "../../src/ui.ts";

test("the UI entry exposes disposable computations without application imports", async () => {
  let result: string | null = null;
  const request = createLatestRequest({
    clone: (value: string) => value,
    classifyError: () => "Unavailable",
    onState: (state) => {
      result = state.value;
    },
  });
  await request.run("public-consumer", async () => "ready");
  assert.equal(result, "ready");
  request.dispose();
});

test("public composition combines exact selection with validated display links", async () => {
  const { createSelection, resolveSourceLink } =
    await import("../../src/ui.ts");
  const selection = createSelection({
    ids: ['["local","human","alice"]'],
    initial: [],
    multiple: false,
  });
  selection.toggle('["local","human","alice"]');
  assert.deepEqual(selection.selected, ['["local","human","alice"]']);
  assert.deepEqual(
    resolveSourceLink("/resources/alice", {
      base: "https://studio.example/",
      internal: (path) => path.startsWith("/resources/"),
      externalOrigins: [],
    }),
    { href: "https://studio.example/resources/alice", external: false },
  );
});

test("headless package entry validates host layouts without importing Svelte", async () => {
  const ui = await import("rom-studio/ui");
  assert.equal(typeof ui.validateLayout, "function");
  assert.deepEqual(
    ui.validateLayout([], {
      columns: 4,
      maxRows: 10,
      maxItems: 2,
      catalog: [],
    }),
    { valid: true, layout: [] },
  );
});
