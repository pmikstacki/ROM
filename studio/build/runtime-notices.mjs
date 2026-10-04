// Build-only Vite hook; all collection and admission share the named notice modules.
import { collectRuntimeNotices } from './notices/collection.mjs';

export function runtimeNotices() {
  let root;
  return {
    name: 'vite-plugin-rom-runtime-notices',
    apply: 'build',
    enforce: 'post',
    configResolved(config) { root = config.root; },
    generateBundle: {
      order: 'post',
      handler(_options, bundle) {
        for (const [fileName, source] of collectRuntimeNotices(root, bundle)) {
          this.emitFile({ type: 'asset', fileName, source });
        }
      },
    },
  };
}
