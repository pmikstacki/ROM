import { stringifyWire } from "../client/codec.ts";
import type { WireValue } from "../client/types.ts";

/** Exact bounded local editor association, not authentication, authorization, or a receipt. */
export async function editorFingerprint(
  value: WireValue,
  maxBytes: number,
): Promise<string> {
  const bytes = new TextEncoder().encode(stringifyWire(value, maxBytes));
  // WebCrypto REC 2017 sections 14.3.5 and 30.3: digest copied message bytes with SHA-256.
  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  return (
    "sha256:" +
    Array.from(new Uint8Array(digest), (byte) =>
      byte.toString(16).padStart(2, "0"),
    ).join("")
  );
}
