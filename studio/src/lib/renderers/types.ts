import type { Component } from "svelte";
import type { FieldDescriptor, WireValue } from "../client/types.ts";
export interface RendererProps {
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
