import type { SessionIdentity } from "./types.ts";
export function identity(value: SessionIdentity): SessionIdentity {
  const { principal, generation, expiresAt } = value;
  if (
    !principal ||
    !["human", "service", "embedded"].includes(principal.kind) ||
    ![principal.authority, principal.subject, generation].every(
      (s) => typeof s === "string" && s.length > 0 && s.length <= 4096,
    ) ||
    !Number.isSafeInteger(expiresAt) ||
    expiresAt < 0
  )
    throw Error("invalid session identity");
  return {
    principal: {
      authority: principal.authority,
      kind: principal.kind,
      subject: principal.subject,
    },
    generation,
    expiresAt,
  };
}
export function samePrincipal(
  left: SessionIdentity,
  right: SessionIdentity,
): boolean {
  return (
    left.principal.authority === right.principal.authority &&
    left.principal.kind === right.principal.kind &&
    left.principal.subject === right.principal.subject
  );
}
