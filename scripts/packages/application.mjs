// Fresh external applications use declared source files and extracted libraries.
import { copyFileSync, existsSync, lstatSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { basename, dirname, join, resolve, sep } from 'node:path';
import { renderManifest } from './manifest.mjs';

// Capture the complete copied inputs before Cargo creates build output.
export function applicationInputs(root, paths) {
  const entries = [];
  function visit(path) {
    const full = join(root, path);
    const stat = lstatSync(full);
    if (stat.isDirectory()) {
      for (const name of readdirSync(full).sort()) visit(join(path, name));
    } else if (stat.isFile()) {
      entries.push([path, createHash('sha256').update(readFileSync(full)).digest('hex')]);
    } else throw Error('nonregular application evidence input');
  }
  for (const path of paths) visit(path);
  return Object.fromEntries(entries.sort(([a], [b]) => a.localeCompare(b, 'en')));
}

function copyRegular(source, destination) {
  const stat = lstatSync(source);
  if (stat.isSymbolicLink()) throw Error('symbolic application input');
  if (stat.isDirectory()) {
    mkdirSync(destination, {recursive:true});
    for (const entry of readdirSync(source)) {
      if (['node_modules', 'target'].includes(entry) || entry.startsWith('.')) continue;
      copyRegular(join(source,entry),join(destination,entry));
    }
  } else if (stat.isFile()) {
    mkdirSync(dirname(destination), {recursive:true});
    copyFileSync(source,destination);
  } else throw Error('nonregular application input');
}

export function copyApplication(root, pkg, output, patches, options = {}) {
  const source = dirname(pkg.manifest_path);
  if (!resolve(source).startsWith(resolve(root) + sep)) throw Error('application escaped source root');
  const app = join(output, basename(source));
  mkdirSync(output, {recursive:true});
  mkdirSync(app, {recursive:false});
  for (const path of ['src','tests','README.md','settings.toml']) if (existsSync(join(source,path))) copyRegular(join(source,path),join(app,path));
  if (options.providerFixture) copyRegular(join(source,'provider-fixture'),join(app,'provider-fixture'));
  if (options.sharedSupport) {
    const support = 'tests/persistence/tests/support/child_process.rs';
    copyRegular(join(root,support),join(output,support));
  }
  mkdirSync(join(app,'.cargo'));
  writeFileSync(join(app,'.cargo/config.toml'),patches);
  writeFileSync(join(app,'Cargo.toml'),renderManifest(pkg));
  copyFileSync(join(root,'Cargo.lock'),join(app,'Cargo.lock'));
  return app;
}

export function auditPackagePaths(metadata, libraries, extracted) {
  const names = new Set(libraries.map(pkg=>pkg.name));
  for (const pkg of metadata.packages.filter(pkg=>names.has(pkg.name))) {
    if (!resolve(pkg.manifest_path).startsWith(resolve(extracted) + sep)) throw Error(`${pkg.name}: consumer escaped packaged sources`);
  }
}
