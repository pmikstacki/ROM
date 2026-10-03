import type { Invocation, ProjectedView } from "./types.ts";
/** Check the committed result against the immutable submitted operation. */
export function mutationResult(
  request: Invocation,
  result: ProjectedView,
): void {
  const expected = request.expected === null ? null : BigInt(request.expected);
  if (
    expected === null &&
    result.revision !== (request.operation.type === "delete" ? 0n : 1n)
  )
    throw Error("mutation revision mismatch");
  if (
    expected !== null &&
    result.revision !== expected &&
    result.revision !== expected + 1n
  )
    throw Error("mutation revision mismatch");
  if (
    request.operation.type === "delete"
      ? result.value !== null
      : result.value === null
  )
    throw Error("mutation value mismatch");
}
