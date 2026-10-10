import type { DurablePrincipal } from "../recovery/types.ts";

export type ObservationScope =
  | { readonly kind: "public"; readonly key: string }
  | {
      readonly kind: "principal";
      readonly principal: Readonly<DurablePrincipal>;
      readonly generation: number;
    };

function object(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    throw new TypeError("Invalid observation scope");
  return value as Record<string, unknown>;
}
function bounded(value: unknown, maxBytes: number): string {
  if (
    typeof value !== "string" ||
    !value.length ||
    value.length > maxBytes ||
    new TextEncoder().encode(value).byteLength > maxBytes
  ) {
    throw new TypeError("Invalid observation scope");
  }
  return value;
}

/** Capture explicit disclosure scope; this metadata never establishes an authorization grant. */
export function captureObservationScope(value: unknown): ObservationScope {
  const input = object(value);
  if (!Object.hasOwn(input, "kind"))
    throw new TypeError("Invalid observation scope");
  if (input.kind === "public")
    return Object.freeze({
      kind: "public",
      key: bounded(Object.hasOwn(input, "key") ? input.key : undefined, 1024),
    });
  if (input.kind !== "principal")
    throw new TypeError("Invalid observation scope");
  const source = object(input.principal);
  if (
    !["authority", "kind", "subject"].every((key) =>
      Object.hasOwn(source, key),
    ) ||
    !Object.hasOwn(input, "generation") ||
    !Object.hasOwn(input, "principal")
  )
    throw new TypeError("Invalid observation scope");
  const kind = source.kind;
  const generation = input.generation;
  if (kind !== "embedded" && kind !== "human" && kind !== "service")
    throw new TypeError("Invalid observation scope");
  if (
    typeof generation !== "number" ||
    !Number.isSafeInteger(generation) ||
    generation < 0
  )
    throw new TypeError("Invalid observation scope");
  const principal = Object.freeze({
    authority: bounded(source.authority, 16384),
    kind,
    subject: bounded(source.subject, 16384),
  });
  return Object.freeze({ kind: "principal", principal, generation });
}

/** Compare fields directly: opaque identity components can contain delimiters. */
export function sameObservationScope(
  left: ObservationScope,
  right: ObservationScope,
): boolean {
  if (left.kind === "public")
    return right.kind === "public" && left.key === right.key;
  return (
    right.kind === "principal" &&
    left.generation === right.generation &&
    left.principal.authority === right.principal.authority &&
    left.principal.kind === right.principal.kind &&
    left.principal.subject === right.principal.subject
  );
}
