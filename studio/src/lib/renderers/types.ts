import type { Component } from "svelte";
import type { FieldDescriptor, WireValue } from "../client/types.ts";
import type { EditorDraft } from "./editor-draft.ts";
export interface RendererProps {
  /** Local editor state; never part of the wire value. */
  draft?: EditorDraft;
  onDraftChange?: (draft: EditorDraft) => void;
  descriptor: FieldDescriptor;
  value: WireValue;
  onchange: (value: WireValue) => void;
  readonly?: boolean;
  mode?: "editor" | "detail" | "cell";
  onerror?: (message: string) => void;
  /** Human label; descriptor.name remains the canonical field identity. */
  label?: string;
}
export type FieldRenderer = Component<RendererProps>;
