import { test } from "node:test";
import assert from "node:assert/strict";
import {
  resourceTitle,
  resourceLabel,
  settingsSections,
} from "../../src/lib/presentation/resource-presentation.ts";
import type {
  ResourceDescriptor,
  ProjectedView,
} from "../../src/lib/client/types.ts";
const user: ResourceDescriptor = {
  kind: "users",
  version: 1,
  fields: [{ name: "display_name", shape: { type: "string" } }],
  actions: [],
  action_inputs: [],
  presentation: { label: "User", title_field: "display_name" },
};
const row: ProjectedView = {
  key: { kind: "users", id: '["local","human","alice"]' },
  revision: 1n,
  value: { display_name: "Alice" },
};
test("titles use declared authorized fields rather than interpreting identity", () => {
  assert.equal(resourceLabel(user), "User");
  assert.equal(resourceTitle(user, row), "Alice");
  assert.equal(row.key.id, '["local","human","alice"]');
});
test("missing titles fall back without guessing JSON identity", () => {
  const title = resourceTitle(user, { ...row, value: {} });
  assert.notEqual(title, "Alice");
  assert.notEqual(title, "alice");
  assert.ok(title.startsWith("User"));
});
test("hidden undeclared title fields are not resolved", () => {
  assert.notEqual(resourceTitle({ ...user, fields: [] }, row), "Alice");
});
test("settings catalog includes classified descriptors only and groups plugins", () => {
  const plugin: ResourceDescriptor = {
    ...user,
    kind: "mail-settings",
    presentation: {
      label: "Mail settings",
      settings: { group: "mail", label: "Mail plugin" },
    },
  };
  const sections = settingsSections([user, plugin]);
  assert.equal(sections.length, 1);
  assert.equal(sections[0].label, "Mail plugin");
  assert.deepEqual(
    sections[0].resources.map((r) => r.kind),
    ["mail-settings"],
  );
});
