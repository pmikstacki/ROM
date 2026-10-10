import {
  protocolRecord,
  protocolText,
  InvalidSessionResponse,
} from "./browser-response.ts";
import type { ProviderChoice } from "./protocol-types.ts";
export function providerResponse(value: unknown): {
  providers: ProviderChoice[];
  primary: string | null;
} {
  const body = protocolRecord(value);
  if (!Array.isArray(body.providers) || body.providers.length > 100)
    throw new InvalidSessionResponse();
  const providers = body.providers.map((value) => {
    const item = protocolRecord(value);
    return { id: protocolText(item.id), label: protocolText(item.label) };
  });
  if (new Set(providers.map((p) => p.id)).size !== providers.length)
    throw new InvalidSessionResponse();
  const primary = body.primary === null ? null : protocolText(body.primary);
  if (primary !== null && !providers.some((p) => p.id === primary))
    throw new InvalidSessionResponse();
  return { providers, primary };
}
