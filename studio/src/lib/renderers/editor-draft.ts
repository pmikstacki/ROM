import type { WireValue } from "../client/types.ts";

/** Editor state is local to a row identity and is never serialized as a value. */
export interface EditorDraft {
  text?: string;
  invalid?: string;
  state?: Record<string, unknown>;
  children?: Record<string, EditorDraft>;
  errors?: Record<string, string>;
  rows?: ListEditorRow[];
}

export interface ListEditorRow {
  id: string;
  identity: string;
  value: WireValue;
  error: string;
  draft: EditorDraft;
}
