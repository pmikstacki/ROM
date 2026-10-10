import test from "node:test";
import assert from "node:assert/strict";
import { createSelection } from "../../src/lib/ui/selection.ts";

test("selection preserves exact opaque IDs and returns detached snapshots", () => {
  const input = ['["local","human","alice"]', "other"];
  const selection = createSelection({
    ids: input,
    initial: [input[0]],
    multiple: true,
  });
  input[0] = "changed";
  const snapshot = selection.selected;
  assert.deepEqual(snapshot, ['["local","human","alice"]']);
  assert.throws(() => {
    (snapshot as string[]).push("other");
  });
  selection.toggle("other");
  assert.equal(snapshot.length, 1);
  assert.equal(selection.selected.length, 2);
});

test("single selection toggles and replaces without inferring labels", () => {
  const selection = createSelection({
    ids: ["a", "b"],
    initial: [],
    multiple: false,
  });
  selection.toggle("a");
  selection.toggle("b");
  assert.deepEqual(selection.selected, ["b"]);
  selection.toggle("b");
  assert.deepEqual(selection.selected, []);
});

test("removing available IDs prunes selection and clear empties it", () => {
  const selection = createSelection({
    ids: ["a", "b"],
    initial: ["a", "b"],
    multiple: true,
  });
  selection.replaceAvailable(["b", "c"]);
  assert.deepEqual(selection.selected, ["b"]);
  assert.throws(() => selection.toggle("a"), /unavailable/i);
  selection.clear();
  assert.deepEqual(selection.selected, []);
});

test("invalid catalogs and initial selection fail explicitly", () => {
  assert.throws(
    () => createSelection({ ids: ["a", "a"], initial: [], multiple: true }),
    /duplicate/i,
  );
  assert.throws(
    () => createSelection({ ids: ["a"], initial: ["b"], multiple: true }),
    /unavailable/i,
  );
  assert.throws(
    () =>
      createSelection({
        ids: ["a", "b"],
        initial: ["a", "b"],
        multiple: false,
      }),
    /single/i,
  );
  assert.throws(
    () => createSelection({ ids: ["a"], initial: ["a", "a"], multiple: true }),
    /duplicate/i,
  );
});

test("invalid replacement is atomic and does not discard current selection", () => {
  const selection = createSelection({
    ids: ["a"],
    initial: ["a"],
    multiple: true,
  });
  assert.throws(() => selection.replaceAvailable(["b", "b"]), /duplicate/i);
  assert.deepEqual(selection.selected, ["a"]);
  selection.toggle("a");
  assert.deepEqual(selection.selected, []);
});
