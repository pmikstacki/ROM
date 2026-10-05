import type { ResourceDescriptor } from "../client/types.ts";

/** Session-owned optional lookup. Labels never replace the exact stored reference ID. */
export const REFERENCE_LOOKUP = Symbol("rom.reference-lookup");
export interface ReferenceCandidate {
  id: string;
  title: string;
}
export type ReferenceLookupResult =
  | { status: "ready"; candidates: ReferenceCandidate[]; limited: boolean }
  | {
      status: "unavailable" | "denied" | "error" | "cancelled";
      message: string;
    };
export interface ReferenceLookup {
  descriptor(kind: string): ResourceDescriptor | undefined;
  lookup(
    kind: string,
    search: string,
    signal: AbortSignal,
  ): Promise<ReferenceLookupResult>;
}
export const REFERENCE_LIMIT = 20;
export const REFERENCE_BYTES = 65536;
export const REFERENCE_SEARCH_BYTES = 1024;
