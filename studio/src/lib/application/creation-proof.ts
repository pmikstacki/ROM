import { stringifyWire } from "../client/codec.ts";
import type { Operation, WireObject } from "../client/types.ts";
import type { EditorDraft } from "../renderers/editor-draft.ts";
import type { CreationDraft } from "./creation-drafts.ts";

function invalidEditors(editors: Record<string, EditorDraft>): boolean {
  const pending = Object.values(editors);
  while (pending.length) {
    const editor = pending.pop()!;
    if (editor.invalid || Object.values(editor.errors ?? {}).some(Boolean))
      return true;
    pending.push(...Object.values(editor.children ?? {}));
    for (const row of editor.rows ?? []) {
      if (row.error) return true;
      pending.push(row.draft);
    }
  }
  return false;
}

/** Associate a validated, bounded editor snapshot only with its represented create. */
export function creationDraftMatches(
  draft: CreationDraft,
  id: string,
  operation: Operation,
): boolean {
  if (
    draft.id !== id ||
    operation.type !== "create" ||
    invalidEditors(draft.form.editors)
  )
    return false;
  const input: WireObject = Object.create(null);
  for (const [name, value] of Object.entries(draft.form.intents)) {
    if (!value || typeof value !== "object" || Array.isArray(value))
      return false;
    if (value.mode === "value" && Object.hasOwn(value, "value"))
      input[name] = value.value;
    else if (value.mode === "null") input[name] = null;
    else if (value.mode !== "omit") return false;
  }
  const keys = Object.keys(input).sort(),
    operationKeys = Object.keys(operation.input).sort();
  return (
    keys.length === operationKeys.length &&
    keys.every((key, index) => key === operationKeys[index]) &&
    keys.every(
      (key) =>
        stringifyWire(input[key]) === stringifyWire(operation.input[key]),
    )
  );
}
