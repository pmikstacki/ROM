import { test } from "node:test";
import assert from "node:assert/strict";
import { proposeLayoutChange } from "../../src/lib/ui/components/layout-controls.ts";
const options = {
  columns: 4,
  maxRows: 4,
  maxItems: 1,
  catalog: [
    { id: " exact/id ", minWidth: 1, maxWidth: 3, minHeight: 1, maxHeight: 3 },
  ],
};
const item = {
  id: " exact/id ",
  x: 0,
  y: 0,
  width: 2,
  height: 2,
  visible: true,
};
test("layout proposals retain exact IDs and validate move/size/visibility without mutation", () => {
  const result = proposeLayoutChange([item], options, {
    id: item.id,
    action: "right",
  });
  assert.equal(result.valid, true);
  if (result.valid) {
    assert.equal(result.layout[0].x, 1);
    assert.equal(result.layout[0].id, item.id);
    assert.equal(Object.isFrozen(result.layout[0]), true);
  }
  assert.equal(item.x, 0);
  assert.equal(
    proposeLayoutChange([item], options, { id: item.id, action: "left" }).valid,
    false,
  );
  assert.equal(
    proposeLayoutChange([item], options, { id: "unknown", action: "right" })
      .valid,
    false,
  );
  const hide = proposeLayoutChange([item], options, {
    id: item.id,
    action: "hide",
  });
  if (hide.valid) assert.equal(hide.layout[0].visible, false);
});
test("invalid commands and corrupt layouts never propose a fallback", () => {
  assert.equal(
    proposeLayoutChange(null, options, { id: item.id, action: "right" }).valid,
    false,
  );
  assert.equal(
    proposeLayoutChange([item], options, { id: item.id, action: "teleport" })
      .valid,
    false,
  );
});
