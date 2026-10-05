/** Pages share the disclosed Resource and mutation contracts. */
export type StudioPage = "resources" | "work" | "attachments" | "settings";

export function navigationBlocked(
  mutationState: string | undefined,
  workBlocked: boolean,
): boolean {
  return (
    workBlocked || mutationState === "pending" || mutationState === "unknown"
  );
}
