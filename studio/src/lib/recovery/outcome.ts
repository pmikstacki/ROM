import { RemoteError } from "../client/request.ts";
import type { AcceptedIntent } from "./record.ts";
import type { MutationRecoveryState } from "./types.ts";
/** A current refusal cannot erase an earlier attempt's unknown commit outcome. */
export function failedAttempt(problem: unknown, earlierUncertain: boolean): {
  phase: AcceptedIntent["phase"];
  uncertain: boolean;
  knowledge: AcceptedIntent["knowledge"];
  error: NonNullable<MutationRecoveryState["error"]>;
} {
  const category = problem instanceof RemoteError && /^[a-z][a-z0-9_]{0,63}$/.test(problem.category)
    ? problem.category : "outcome_unknown";
  const definitive = category === "not_committed" && !earlierUncertain;
  const phase = category === "not_committed" ? "rejected"
    : category === "conflict" ? "conflict"
    : problem instanceof RemoteError && category !== "outcome_unknown" && problem.status < 500 ? "rejected" : "unknown";
  return {
    phase,
    uncertain: !definitive,
    knowledge: definitive ? "not_committed" : "unknown",
    error: { category, reason: definitive ? "The attempt did not commit." : "The earlier mutation outcome is not confirmed." },
  };
}
