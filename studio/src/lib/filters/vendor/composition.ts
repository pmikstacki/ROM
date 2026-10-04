// Adapted from SVAR Filter RulesTree.ts, commit 1c581c3312c626c525ee64b8f94446a025fa141c.
// Copyright (c) 2025 XB Software Sp. z o.o. MIT; see LICENSE and provenance.json.
import type { FilterRule, FilterSet } from "../types.ts";
/** Preserve every rule. Upstream invalid-value dropping is deliberately removed. */
export function serializeBranch(
  top: FilterRule | FilterSet,
  depth = 0,
): FilterRule | FilterSet {
  if (depth > 8) throw Error("Filter group depth limit reached.");
  if (!top || typeof top !== "object" || Array.isArray(top))
    throw Error("Invalid filter rule.");
  if ("rules" in top) {
    if (!Array.isArray(top.rules) || top.rules.length > 32)
      throw Error("At most 32 filter rules are supported.");
    return {
      glue: top.glue,
      rules: top.rules.map((rule) => serializeBranch(rule, depth + 1)),
    };
  } else {
    const out: FilterRule = {
      field: top.field,
      filter: top.filter,
      value: top.value,
    };
    return out;
  }
}
