// Reproducible installed-source byte check. This does not run the actual-host gate.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.argv.length !== 3) throw Error('usage: node probe.mjs ROM_ROOT');
const root = resolve(process.argv[2]), studio = join(root, 'studio');
const { build } = await import(pathToFileURL(join(studio, 'node_modules/vite/dist/node/index.js')));
const { validateRuntimeNotices } = await import(pathToFileURL(join(studio, 'build/notices/inventory.mjs')));
for (const scope of ['production', 'isolated-svelte-toolbelt']) {
  const lib = scope === 'production' ? {} : { lib: { entry: join(studio, 'node_modules/svelte-toolbelt/dist/index.js'), formats: ['es'], fileName: 'toolbelt-probe' } };
  const result = await build({ root: studio, build: { write: false, ...lib } });
  const files = new Map((Array.isArray(result) ? result : [result]).flatMap(x => x.output)
    .map(x => [x.fileName, Buffer.from(x.type === 'asset' ? x.source : x.code)]));
  const inventory = validateRuntimeNotices(files);
  for (const owner of inventory.owners) {
    const directory = owner.kind === 'vendored' ? join(studio, 'src/lib/components/ui') : join(studio, 'node_modules', owner.name);
    for (const notice of owner.notices) assert.deepEqual(files.get(notice.path), readFileSync(join(directory, notice.source)));
  }
  if (scope !== 'production') {
    const toolbelt = inventory.owners.find(x => x.name === 'svelte-toolbelt');
    assert.ok(toolbelt); assert.equal(toolbelt.license_expression, null);
    assert.ok(toolbelt.notices.some(x => x.source === 'LICENSE'));
  }
  console.log(JSON.stringify({ scope, original_notice_bytes_equal: true,
    owners: inventory.owners.map(x => ({ name: x.name, version: x.version, license_expression: x.license_expression,
      modules: x.modules.length, notices: x.notices })) }, null, 2));
}
