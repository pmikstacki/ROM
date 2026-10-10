// Original execution paths are admitted only by the source-bound manifest context.
import { join } from 'node:path';
import { requireConsumerContext } from './consumer-context.mjs';
export function consumerOrigins(lease, directory, kind, context) {
  if(context===undefined)return{source_input:join(lease.root,'studio'),installed_package:join(directory,'consumer/node_modules/rom-studio')};
  requireConsumerContext(context,directory);
  for(const key of ['revision','tree','source_inventory_sha256','studio_sha256','archive_sha256'])if(context[key]!==lease.witness?.[key])throw Error('consumer execution origin witness mismatch');
  return{source_input:join(context.extracted_root,'studio'),installed_package:join(context.evidence_root,kind,'consumer/node_modules/rom-studio')};
}
