// Render Cargo metadata without retaining source-checkout dependency paths.
const quote = value => JSON.stringify(value);
const scopes = new Map([[null, 'dependencies'], ['dev', 'dev-dependencies'], ['build', 'build-dependencies']]);

export function renderManifest(pkg) {
  const sections = new Map();
  for (const dependency of pkg.dependencies) {
    if (!scopes.has(dependency.kind)) throw Error('unsupported dependency kind');
    if (dependency.registry && dependency.registry !== 'https://github.com/rust-lang/crates.io-index') throw Error('unsupported dependency registry');
    const scope = scopes.get(dependency.kind);
    const key = dependency.target ? `target.${quote(dependency.target)}.${scope}` : scope;
    const options = [`version = ${quote(dependency.req)}`];
    if (!dependency.uses_default_features) options.push('default-features = false');
    if (dependency.features.length) options.push(`features = ${quote(dependency.features)}`);
    if (dependency.optional) options.push('optional = true');
    if (dependency.rename) options.push(`package = ${quote(dependency.name)}`);
    if (!sections.has(key)) sections.set(key, []);
    sections.get(key).push(`${quote(dependency.rename ?? dependency.name)} = { ${options.join(', ')} }`);
  }
  const header = ['[package]', `name = ${quote(pkg.name)}`, `version = ${quote(pkg.version)}`, `edition = ${quote(pkg.edition)}`, 'publish = false'];
  if (pkg.rust_version) header.push(`rust-version = ${quote(pkg.rust_version)}`);
  if (pkg.license) header.push(`license = ${quote(pkg.license)}`);
  const output = [header.join('\n')];
  for (const [name, lines] of sections) output.push(`[${name}]\n${lines.join('\n')}`);
  if (Object.keys(pkg.features ?? {}).length) output.push('[features]\n' + Object.entries(pkg.features).map(([key, value]) => `${quote(key)} = ${quote(value)}`).join('\n'));
  output.push('[workspace]');
  return output.join('\n\n') + '\n';
}
