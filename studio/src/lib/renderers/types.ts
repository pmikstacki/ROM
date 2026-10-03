import type { Component } from "svelte";
import type { FieldDescriptor, WireValue } from "../client/types.ts";
export interface RendererProps {
  descriptor: FieldDescriptor;
  value: WireValue;
  onchange: (value: WireValue) => void;
  readonly?: boolean;
  mode?: "editor" | "detail" | "cell";
  onerror?: (message: string) => void;
}
export type FieldRenderer = Component<RendererProps>;
