import type { FieldUpdate, WireObject } from "../client/types.ts";
export type ResourceFormSubmission =
  | { type: "create" | "replace"; input: WireObject }
  | { type: "patch"; input: Record<string, FieldUpdate> };
