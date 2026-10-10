import { stringifyWire } from "../client/codec.ts";
import { unsigned } from "../client/validation.ts";
import type { FieldIntent, WireObject, WireValue } from "../client/types.ts";
import type { EditorDraft } from "../renderers/editor-draft.ts";
import type { EditorSnapshot } from "../application/editor-drafts.ts";
import { copyEditorSnapshot } from "../application/snapshot.ts";
export interface FormDraftIdentity {
  mode: "resource" | "action";
  descriptor: string;
  baseRevision: bigint | null;
}
export interface FormDraft extends FormDraftIdentity {
  intents: Record<string, FieldIntent>;
  editors: Record<string, EditorDraft>;
}
function intent(value: WireValue): FieldIntent {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw Error("Invalid form intent.");
  const mode = value.mode;
  if (mode === "value") {
    if (
      !Object.hasOwn(value, "value") ||
      Object.keys(value).some((key) => key !== "mode" && key !== "value")
    )
      throw Error("Invalid form intent value.");
    return { mode, value: value.value };
  }
  if (
    !["omit", "null", "remove"].includes(String(mode)) ||
    Object.keys(value).length !== 1
  )
    throw Error("Invalid form intent mode.");
  return { mode: mode as "omit" | "null" | "remove" };
}
/** Wire input and raw editor text remain distinct. Storage and authority belong to the host. */
export function captureFormDraft(
  value: FormDraft & { maxBytes?: number },
): EditorSnapshot {
  const intents: WireObject = {};
  for (const [key, entry] of Object.entries(value.intents)) {
    const wire: WireValue =
      entry.mode === "value"
        ? { mode: entry.mode, value: entry.value }
        : { mode: entry.mode };
    intent(wire);
    Object.defineProperty(intents, key, {
      value: wire,
      enumerable: true,
      configurable: true,
      writable: true,
    });
  }
  return copyEditorSnapshot(
    {
      mode: value.mode,
      descriptor: value.descriptor,
      baseRevision: value.baseRevision,
      intents,
      editors: value.editors,
    },
    value.maxBytes ?? 1048576,
  );
}
export function restoreFormDraft(
  snapshot: EditorSnapshot,
  identity: FormDraftIdentity,
  maxBytes = 1048576,
): FormDraft {
  if (
    snapshot.mode !== identity.mode ||
    snapshot.descriptor !== identity.descriptor
  )
    throw Error("Form draft identity changed.");
  const copy = copyEditorSnapshot(snapshot, maxBytes),
    intents: Record<string, FieldIntent> = {};
  for (const [key, value] of Object.entries(copy.intents))
    Object.defineProperty(intents, key, {
      value: intent(value),
      enumerable: true,
      configurable: true,
      writable: true,
    });
  return {
    mode: copy.mode,
    descriptor: copy.descriptor,
    baseRevision: copy.baseRevision,
    intents,
    editors: copy.editors,
  };
}

export interface FormFrameDefinition {
  descriptor: string;
  resource: string;
  actions: Record<string, string>;
}
export interface FormFrameBounds {
  maxBytes: number;
  maxFrames: number;
  maxChildren: number;
  maxDepth: number;
}
export type FormFrame = { type: "resource" } | { type: "action"; name: string };
const defaultBounds: FormFrameBounds = {
  maxBytes: 1048576,
  maxFrames: 64,
  maxChildren: 4096,
  maxDepth: 32,
};
function frameBounds(options?: Partial<FormFrameBounds>) {
  const bounds = { ...defaultBounds, ...options };
  if (
    Object.values(bounds).some(
      (value) => !Number.isSafeInteger(value) || value < 1,
    )
  )
    throw Error("Invalid form frame bounds.");
  return bounds;
}
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw Error("Invalid form frame object.");
  return value as Record<string, unknown>;
}
function keys(value: Record<string, unknown>, allowed: string[]) {
  if (Object.keys(value).some((key) => !allowed.includes(key)))
    throw Error("Extra form frame properties.");
}
function boundedFrames(snapshot: EditorSnapshot, bounds: FormFrameBounds) {
  const baseRevision =
    snapshot.baseRevision === null
      ? null
      : unsigned(snapshot.baseRevision as WireValue, snapshot, "baseRevision");
  const copy = copyEditorSnapshot(snapshot, bounds.maxBytes);
  copy.baseRevision = baseRevision;
  let children = 0;
  function visit(value: unknown, depth: number) {
    if (depth > bounds.maxDepth)
      throw Error("Form frame depth bound exceeded.");
    if (value && typeof value === "object") {
      const entries = Object.values(value);
      children += entries.length;
      if (children > bounds.maxChildren)
        throw Error("Form frame child bound exceeded.");
      for (const child of entries) visit(child, depth + 1);
    }
  }
  visit(copy, 0);
  return copy;
}
function frameLeaf(
  value: unknown,
  mode: "resource" | "action",
  descriptor: string,
  baseRevision: bigint | null,
  maxBytes: number,
) {
  const leaf = record(value);
  keys(leaf, ["mode", "descriptor", "baseRevision", "intents", "editors"]);
  if (leaf.mode !== mode || leaf.descriptor !== descriptor)
    throw Error("Form frame identity changed.");
  const leafRevision =
    leaf.baseRevision === null
      ? null
      : unsigned(leaf.baseRevision as WireValue, leaf, "baseRevision");
  if (leafRevision !== baseRevision)
    throw Error("Form frame revision changed.");
  record(leaf.intents);
  record(leaf.editors);
  const normalized = {
    ...leaf,
    baseRevision: leafRevision,
  } as unknown as EditorSnapshot;
  restoreFormDraft(normalized, { mode, descriptor, baseRevision }, maxBytes);
  return normalized;
}
/** One bounded editor record contains each visible form; accepted command identity stays in the R4 lane. */
export function readFormFrames(
  snapshot: EditorSnapshot,
  definition: FormFrameDefinition,
  options?: Partial<FormFrameBounds>,
) {
  const bounds = frameBounds(options),
    copy = boundedFrames(snapshot, bounds);
  if (copy.mode !== "resource" || copy.descriptor !== definition.descriptor)
    throw Error("Form frame set identity changed.");
  keys(record(copy), [
    "mode",
    "descriptor",
    "baseRevision",
    "intents",
    "editors",
  ]);
  if (Object.keys(record(copy.intents)).length)
    throw Error("Extra form frame intents.");
  const editors = record(copy.editors);
  keys(editors, ["resource", "actions"]);
  let resource: EditorSnapshot | null = null,
    count = 0;
  const actions: Record<string, EditorSnapshot> = {};
  if (Object.hasOwn(editors, "resource")) {
    const frame = record(editors.resource);
    keys(frame, ["state"]);
    const state = record(frame.state);
    keys(state, ["snapshot"]);
    resource = frameLeaf(
      state.snapshot,
      "resource",
      definition.resource,
      copy.baseRevision,
      bounds.maxBytes,
    );
    count++;
  }
  if (Object.hasOwn(editors, "actions")) {
    const frame = record(editors.actions);
    keys(frame, ["children"]);
    const frames = record(frame.children);
    for (const [name, value] of Object.entries(frames)) {
      if (!Object.hasOwn(definition.actions, name))
        throw Error("Unknown action form frame identity.");
      const wrapper = record(value);
      keys(wrapper, ["state"]);
      const state = record(wrapper.state);
      keys(state, ["snapshot"]);
      const leaf = frameLeaf(
        state.snapshot,
        "action",
        definition.actions[name],
        copy.baseRevision,
        bounds.maxBytes,
      );
      Object.defineProperty(actions, name, {
        value: leaf,
        enumerable: true,
        configurable: true,
        writable: true,
      });
      count++;
    }
  }
  if (count > bounds.maxFrames) throw Error("Form frame count bound exceeded.");
  return { resource, actions };
}
export function mergeFormFrames(
  previous: EditorSnapshot | null,
  frame: FormFrame,
  next: EditorSnapshot,
  definition: FormFrameDefinition,
  options?: Partial<FormFrameBounds>,
): EditorSnapshot {
  const bounds = frameBounds(options),
    existing = previous
      ? readFormFrames(previous, definition, bounds)
      : { resource: null, actions: {} };
  const baseRevision = previous?.baseRevision ?? next.baseRevision;
  const expected =
    frame.type === "resource"
      ? definition.resource
      : Object.hasOwn(definition.actions, frame.name)
        ? definition.actions[frame.name]
        : null;
  if (expected === null) throw Error("Unknown action form frame identity.");
  const leaf = frameLeaf(
    next,
    frame.type === "resource" ? "resource" : "action",
    expected,
    baseRevision,
    bounds.maxBytes,
  );
  if (frame.type === "resource") existing.resource = leaf;
  else
    Object.defineProperty(existing.actions, frame.name, {
      value: leaf,
      enumerable: true,
      configurable: true,
      writable: true,
    });
  const children = Object.fromEntries(
    Object.entries(existing.actions).map(([name, snapshot]) => [
      name,
      { state: { snapshot } },
    ]),
  );
  const aggregate = {
    mode: "resource" as const,
    descriptor: definition.descriptor,
    baseRevision,
    intents: {},
    editors: {
      ...(existing.resource
        ? { resource: { state: { snapshot: existing.resource } } }
        : {}),
      actions: { children },
    },
  };
  const copy = boundedFrames(aggregate, bounds);
  readFormFrames(copy, definition, bounds);
  return copy;
}

export function resourceDraftIdentity(
  descriptor: import("../client/types.ts").ResourceDescriptor,
  mode: "create" | "replace" | "patch" = "patch",
) {
  return stringifyWire({
    kind: descriptor.kind,
    version: descriptor.version,
    mode,
    fields: descriptor.fields,
  } as unknown as WireValue);
}
export function actionDraftIdentity(
  descriptor: import("../client/types.ts").ResourceDescriptor,
  action: import("../client/types.ts").ActionInput,
) {
  return stringifyWire({
    kind: descriptor.kind,
    version: descriptor.version,
    action,
  } as unknown as WireValue);
}
export function formFrameDefinition(
  descriptor: import("../client/types.ts").ResourceDescriptor,
): FormFrameDefinition {
  if (
    new Set(descriptor.action_inputs.map((action) => action.name)).size !==
    descriptor.action_inputs.length
  )
    throw Error("Duplicate action form identity.");
  return {
    descriptor: stringifyWire({
      kind: descriptor.kind,
      version: descriptor.version,
      fields: descriptor.fields,
      actions: descriptor.action_inputs,
    } as unknown as WireValue),
    resource: resourceDraftIdentity(descriptor),
    actions: Object.fromEntries(
      descriptor.action_inputs.map((action) => [
        action.name,
        actionDraftIdentity(descriptor, action),
      ]),
    ),
  };
}
