// External Cargo projects use only explicitly supplied public ROM sources.
import { copyFileSync, cpSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { hash, safePath } from './files.mjs';

export async function rustProject(context, name) {
  const template = safePath(context.bundle, `skills/rom/assets/${name}/project`);
  mkdirSync(context.project);
  cpSync(template, context.project, { recursive: true });
  const dependency = path => JSON.stringify(join(context.root, 'crates', path));
  writeFileSync(join(context.project, 'Cargo.toml'), `[package]\nname = "rom-skill-${name}-example"\nversion = "0.0.0"\nedition = "2024"\nrust-version = "1.99"\npublish = false\n\n[workspace]\n\n[dependencies]\nrom = { path = ${dependency('rom')} }\n\n[dev-dependencies]\nrom-conformance = { path = ${dependency('rom-conformance')} }\nrom-sqlite = { path = ${dependency('rom-sqlite')}, features = ["test-support"] }\ntokio = { version = "=1.53.1", features = ["rt-multi-thread", "macros", "sync", "time"] }\n`);
  copyFileSync(safePath(context.root, 'Cargo.lock'), join(context.project, 'Cargo.lock'));
  await context.command('cargo', ['test', '--offline', '--manifest-path', join(context.project, 'Cargo.toml')], { cwd: context.project });
  await context.command('cargo', ['clippy', '--offline', '--manifest-path', join(context.project, 'Cargo.toml'), '--all-targets', '--', '-D', 'warnings'], { cwd: context.project });
  return { external_project: context.project, source_profile: 1, baseline_only: true, generated_lock_sha256: hash(join(context.project, 'Cargo.lock')), source_lock_sha256: hash(safePath(context.root, 'Cargo.lock')) };
}
