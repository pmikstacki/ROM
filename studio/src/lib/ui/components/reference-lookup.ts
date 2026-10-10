import type { ResourceDescriptor } from "../../client/types.ts";
import type { ReferenceLookup as GenericReferenceLookup } from "rom-ui/ui/commands/reference-lookup";
export { REFERENCE_LOOKUP, REFERENCE_LIMIT, REFERENCE_BYTES, REFERENCE_SEARCH_BYTES } from "rom-ui/ui/commands/reference-lookup";
export type { ReferenceCandidate, ReferenceLookupResult } from "rom-ui/ui/commands/reference-lookup";
/** ROM lookup retains its disclosed Resource descriptor contract. */
export interface ReferenceLookup extends GenericReferenceLookup {
  descriptor(kind: string): ResourceDescriptor | undefined;
}
