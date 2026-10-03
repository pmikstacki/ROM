// Selected local checks produce evidence; full release acceptance stays separate.
export async function run(context) {
  for (const args of [
    ['fmt', '--all', '--', '--check'],
    ['test', '-p', 'rom-conformance', '--all-features', '--locked'],
    ['test', '-p', 'rom-consumer', '--test', 'native_conformance', '--locked'],
    ['check', '-p', 'rom', '--no-default-features', '--locked'],
  ]) await context.command('cargo', args);
  return { selected_checks_only: true, publication: false, full_release_approval: false };
}
