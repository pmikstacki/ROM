import type { QuerySpec, WireValue } from "../client/types.ts";
export interface FilterRule {
  field: string;
  filter: string;
  value: WireValue;
}
export interface FilterSet {
  glue: "and" | "or";
  rules: (FilterRule | FilterSet)[];
}
export interface FilterDraft {
  rules: FilterSet;
  order: QuerySpec["order"];
  limit: string;
}
