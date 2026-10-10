// Keep wire values out of evidence unless they belong to the fixed public vocabulary.
const categories = new Set([
  "denied", "missing", "conflict", "identity_mismatch", "identity_expired",
  "history_gap", "too_large", "overloaded", "closed", "outcome_unknown",
  "not_committed", "invalid", "internal", "timeout",
]);

export function serverTerminalCategory(value) {
  return categories.has(value) ? value : "other";
}
