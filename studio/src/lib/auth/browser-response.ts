import type { BrowserProtocolResult } from "./protocol-types.ts";
export class InvalidSessionResponse extends Error {}
export function protocolText(value: unknown): string {
  if (typeof value !== "string" || !value || value.length > 4096)
    throw new InvalidSessionResponse();
  return value;
}
export function protocolRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new InvalidSessionResponse();
  return value as Record<string, unknown>;
}
export function sessionResponse(
  value: unknown,
  status: number,
  now: number,
): BrowserProtocolResult {
  const body = protocolRecord(value);
  if (status < 200 || status >= 300) {
    if ((status === 401 || status === 403) && body.error === "denied")
      return { status: "denied" };
    if (
      status === 503 &&
      (body.error === "overloaded" || body.error === "closed")
    )
      return { status: "transient", code: body.error };
    if (status === 500 && body.error === "internal")
      return { status: "transient", code: "internal" };
    return { status: "transient", code: "unavailable" };
  }
  const generation = protocolText(body.generation);
  if (body.authenticated === false) {
    if (
      ["csrf_token", "user_id", "expires_at"].some((key) =>
        Object.hasOwn(body, key),
      )
    )
      throw new InvalidSessionResponse();
    return { status: "session", session: { authenticated: false, generation } };
  }
  if (body.authenticated !== true) throw new InvalidSessionResponse();
  const token = protocolText(body.csrf_token),
    subject = protocolText(body.user_id);
  if (
    typeof body.expires_at !== "number" &&
    typeof body.expires_at !== "bigint"
  )
    throw new InvalidSessionResponse();
  const expiresAt = Number(body.expires_at);
  if (!Number.isSafeInteger(expiresAt) || expiresAt < 0)
    throw new InvalidSessionResponse();
  if (expiresAt <= now) return { status: "expired" };
  return {
    status: "session",
    session: {
      authenticated: true,
      generation,
      csrf_token: token,
      user_id: subject,
      expires_at: expiresAt,
    },
  };
}
